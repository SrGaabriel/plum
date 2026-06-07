use plum_manifest::{Manifest, ctx::Context};
use plum_resolver::{Resolver, ResolverError};
use plum_scheduler::{BuildSummary, SchedulerError, run_build};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error(transparent)]
    Scheduler(#[from] SchedulerError),
    #[error(transparent)]
    Resolver(#[from] ResolverError),
}

pub async fn build(context: Context, manifest: &Manifest) -> Result<BuildSummary, BuildError> {
    let resolver = Resolver::new(context);
    let graph = resolver.resolve_dependencies(manifest).await?;

    let workers = num_cpus::get();
    let (cancel_tx, cancel_rx) = flume::bounded(1);
    
    let result = run_build(&graph, workers, Some(&cancel_rx), |_| true)?;
    Ok(result)
}
