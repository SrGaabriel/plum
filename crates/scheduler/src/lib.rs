use std::sync::atomic::{AtomicBool, Ordering};

use flume::{Receiver, RecvError, Selector};
use fxhash::FxHashMap;
use plum_ghc::CompilationError;
use plum_graph::{BuildNode, DependencyGraph, NodeIndex};
use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub struct BuildSummary {
    pub built: usize,
    pub skipped: usize,
    pub total_in_plan: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum GHCError {
    #[error("failed to spawn ghc for '{package}': {source}")]
    Spawn {
        package: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Compilation(#[from] CompilationError),
}

enum Outcome {
    Built(NodeIndex),
    Failed(NodeIndex, GHCError),
    Skipped(NodeIndex),
}

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error(transparent)]
    Build(#[from] GHCError),
    #[error("build cancelled")]
    Cancelled,
}

enum Event {
    Result(Result<Outcome, RecvError>),
    Cancel,
}

struct CoordResult {
    completed: usize,
    skipped: usize,
    first_error: Option<GHCError>,
    external_cancel: bool,
}

pub fn run_build<G>(
    graph: &DependencyGraph,
    num_workers: usize,
    cancel_rx: Option<&Receiver<()>>,
    is_fresh: G,
) -> Result<BuildSummary, SchedulerError>
where
    G: Fn(&BuildNode) -> bool,
{
    let rebuild = graph.rebuild_set(is_fresh);
    let total = rebuild.len();
    if total == 0 {
        return Ok(BuildSummary {
            built: 0,
            skipped: 0,
            total_in_plan: 0,
        });
    }

    let mut remaining: FxHashMap<NodeIndex, usize> = FxHashMap::default();
    for &n in &rebuild {
        let c = graph
            .dependencies(n)
            .filter(|d| rebuild.contains(d))
            .count();
        remaining.insert(n, c);
    }
    let initial: Vec<NodeIndex> = rebuild
        .iter()
        .copied()
        .filter(|n| remaining[n] == 0)
        .collect();

    let (work_tx, work_rx) = flume::unbounded::<NodeIndex>();
    let (result_tx, result_rx) = flume::unbounded::<Outcome>();
    let cancelled = AtomicBool::new(false);

    let coord = std::thread::scope(|s| {
        for _ in 0..num_workers.max(1) {
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();
            let cancelled = &cancelled;
            s.spawn(move || {
                while let Ok(node) = work_rx.recv() {
                    if cancelled.load(Ordering::Relaxed) {
                        let _ = result_tx.send(Outcome::Skipped(node));
                        continue;
                    }
                    let out = match plum_ghc::compile_module(graph.node(node), &graph) {
                        Ok(()) => Outcome::Built(node),
                        Err(e) => Outcome::Failed(node, e.into()),
                    };
                    let _ = result_tx.send(out);
                }
            });
        }
        drop(work_rx);
        drop(result_tx);

        let mut completed = 0usize;
        let mut skipped = 0usize;
        let mut inflight = 0usize;
        let mut first_error: Option<GHCError> = None;
        let mut external_cancel = false;

        for n in initial {
            let _ = work_tx.send(n);
            inflight += 1;
        }

        while inflight > 0 {
            let event = match (cancel_rx, cancelled.load(Ordering::Relaxed)) {
                (Some(c), false) => Selector::new()
                    .recv(&result_rx, Event::Result)
                    .recv(c, |_| Event::Cancel)
                    .wait(),
                _ => Event::Result(result_rx.recv()),
            };

            match event {
                Event::Cancel => {
                    cancelled.store(true, Ordering::Relaxed);
                    external_cancel = true;
                }
                Event::Result(Err(_)) => break,
                Event::Result(Ok(outcome)) => {
                    inflight -= 1;
                    match outcome {
                        Outcome::Built(node) => {
                            completed += 1;
                            if !cancelled.load(Ordering::Relaxed) {
                                for dep in graph.dependents(node) {
                                    if let Some(c) = remaining.get_mut(&dep) {
                                        *c -= 1;
                                        if *c == 0 {
                                            let _ = work_tx.send(dep);
                                            inflight += 1;
                                        }
                                    }
                                }
                            }
                        }
                        Outcome::Failed(_, e) => {
                            cancelled.store(true, Ordering::Relaxed);
                            if first_error.is_none() {
                                first_error = Some(e);
                            }
                        }
                        Outcome::Skipped(_) => skipped += 1,
                    }
                }
            }
        }
        drop(work_tx);

        CoordResult {
            completed,
            skipped,
            first_error,
            external_cancel,
        }
    });

    if let Some(e) = coord.first_error {
        return Err(SchedulerError::Build(e));
    }
    if coord.external_cancel {
        return Err(SchedulerError::Cancelled);
    }
    Ok(BuildSummary {
        built: coord.completed,
        skipped: coord.skipped,
        total_in_plan: total,
    })
}
