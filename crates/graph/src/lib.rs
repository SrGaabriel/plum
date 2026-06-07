use std::path::PathBuf;

use fxhash::{FxHashMap, FxHashSet};
use lasso::{Rodeo, RodeoReader, Spur};
use petgraph::Direction::{Incoming, Outgoing};
use petgraph::algo::{tarjan_scc, toposort};
use petgraph::graph::DiGraph;
pub use petgraph::graph::NodeIndex;
use plum_manifest::Manifest;

#[derive(Debug, Clone)]
pub struct NodeSpec {
    pub name: String,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildNode {
    pub name: Spur,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub dependencies: Vec<Spur>,
}

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("duplicate package in graph: '{0}'")]
    DuplicatePackage(String),

    #[error("package '{package}' depends on unknown package '{missing}'")]
    DependencyNotFound { package: String, missing: String },

    #[error("circular dependency among packages: {}", .0.join(" -> "))]
    CircularDependency(Vec<String>),
}

#[derive(Default)]
pub struct DependencyGraphBuilder {
    specs: Vec<NodeSpec>,
}

impl DependencyGraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, spec: NodeSpec) -> &mut Self {
        self.specs.push(spec);
        self
    }

    pub fn build(self) -> Result<DependencyGraph, GraphError> {
        let mut rodeo = Rodeo::default();
        let mut graph: DiGraph<BuildNode, ()> = DiGraph::new();
        let mut name_to_idx: FxHashMap<Spur, NodeIndex> = FxHashMap::default();

        for spec in self.specs {
            let name = rodeo.get_or_intern(&spec.name);

            let mut seen = FxHashSet::default();
            let dependencies: Vec<Spur> = spec
                .dependencies
                .iter()
                .map(|d| rodeo.get_or_intern(d))
                .filter(|d| seen.insert(*d))
                .collect();

            let idx = graph.add_node(BuildNode {
                name,
                path: spec.path,
                manifest: spec.manifest,
                dependencies,
            });

            if name_to_idx.insert(name, idx).is_some() {
                return Err(GraphError::DuplicatePackage(
                    rodeo.resolve(&name).to_string(),
                ));
            }
        }

        let mut edges: Vec<(NodeIndex, NodeIndex)> = Vec::new();
        for n in graph.node_indices() {
            let node = &graph[n];
            for &dep in &node.dependencies {
                match name_to_idx.get(&dep) {
                    Some(&dep_idx) => edges.push((dep_idx, n)),
                    None => {
                        return Err(GraphError::DependencyNotFound {
                            package: rodeo.resolve(&node.name).to_string(),
                            missing: rodeo.resolve(&dep).to_string(),
                        });
                    }
                }
            }
        }
        for (from, to) in edges {
            graph.add_edge(from, to, ());
        }

        let interner = rodeo.into_reader();
        for scc in tarjan_scc(&graph) {
            let is_cycle = scc.len() > 1 || graph.contains_edge(scc[0], scc[0]);
            if is_cycle {
                let members = scc
                    .iter()
                    .map(|&i| interner.resolve(&graph[i].name).to_string())
                    .collect();
                return Err(GraphError::CircularDependency(members));
            }
        }

        Ok(DependencyGraph {
            graph,
            interner,
            name_to_idx,
        })
    }
}

#[derive(Debug)]
pub struct DependencyGraph {
    graph: DiGraph<BuildNode, ()>,
    interner: RodeoReader,
    name_to_idx: FxHashMap<Spur, NodeIndex>,
}

impl DependencyGraph {
    pub fn resolve(&self, sym: Spur) -> &str {
        self.interner.resolve(&sym)
    }

    pub fn node_name(&self, n: NodeIndex) -> &str {
        self.interner.resolve(&self.graph[n].name)
    }

    pub fn index_of(&self, name: &str) -> Option<NodeIndex> {
        self.interner
            .get(name)
            .and_then(|s| self.name_to_idx.get(&s).copied())
    }

    pub fn node(&self, n: NodeIndex) -> &BuildNode {
        &self.graph[n]
    }

    pub fn node_indices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.node_indices()
    }

    pub fn dependencies(&self, n: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors_directed(n, Incoming)
    }

    pub fn dependents(&self, n: NodeIndex) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.neighbors_directed(n, Outgoing)
    }

    pub fn dependency_count(&self, n: NodeIndex) -> usize {
        self.graph.neighbors_directed(n, Incoming).count()
    }

    pub fn roots(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph
            .node_indices()
            .filter(move |&n| self.graph.neighbors_directed(n, Incoming).next().is_none())
    }

    pub fn build_order(&self) -> Vec<NodeIndex> {
        toposort(&self.graph, None).expect("graph validated acyclic at construction")
    }

    pub fn rebuild_set<F>(&self, is_fresh: F) -> FxHashSet<NodeIndex>
    where
        F: Fn(&BuildNode) -> bool,
    {
        let mut dirty = FxHashSet::default();
        for n in self.build_order() {
            let dep_dirty = self
                .graph
                .neighbors_directed(n, Incoming)
                .any(|d| dirty.contains(&d));
            if dep_dirty || !is_fresh(&self.graph[n]) {
                dirty.insert(n);
            }
        }
        dirty
    }
}
