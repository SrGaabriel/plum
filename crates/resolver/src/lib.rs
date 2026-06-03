mod cache;
mod git;
mod hackage;

use async_compression::tokio::bufread::GzipDecoder;
use futures_util::TryStreamExt;
use plum_graph::DependencyGraph;
use plum_manifest::{Dependency, DependencySpec, Manifest};
use reqwest::{Client, ClientBuilder};
use tokio::io::BufReader;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

use crate::git::GitDependency;

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error(transparent)]
    GraphError(#[from] plum_graph::GraphError),
    #[error("io error unpacking '{0}': {1}")]
    Io(String, #[source] std::io::Error),
    #[error("manifest parse error for '{0}': {1}")]
    ManifestParse(String, #[source] plum_manifest::Error),
    #[error("conflicting specifications for package '{0}': {1}")]
    ConflictingSpecs(String, String),
    #[error("no source specified for dependency '{0}'")]
    NoSource(String),
    #[error("git needs a rev, tag, or branch specified for '{0}'")]
    GitNoRef(String),
    #[error("could not parse git url for '{0}': {1}")]
    GitUrlParse(String, String),
    #[error("unsupported git host for '{0}'")]
    GitUnsupportedHost(String),
    #[error("failed to fetch git dependency '{0}': {1}")]
    GitFetch(String, #[source] reqwest::Error),
    #[error("failed to run `git ls-remote` for '{0}': {1}")]
    GitLsRemote(String, #[source] std::io::Error),
    #[error("`git ls-remote` failed for '{0}': {1}")]
    GitLsRemoteFailed(String, String),
    #[error("ref '{1}' not found for '{0}'")]
    GitRefNotFound(String, String),
}

#[derive(Debug, Clone, Default)]
pub struct Resolver {
    client: Client,
}

impl Resolver {
    pub fn new() -> Self {
        let client = ClientBuilder::new()
            .build()
            .expect("failed to build HTTP client");
        Self { client }
    }

    pub fn resolve_dependencies(
        &self,
        manifest: &Manifest,
    ) -> Result<DependencyGraph, ResolverError> {
        let mut graph_builder = plum_graph::DependencyGraphBuilder::new();
        for (name, dep) in &manifest.dependencies {
            let node_spec = self.resolve_dependency(name, dep)?;
            graph_builder.add(node_spec);
        }
        let graph = graph_builder.build()?;
        Ok(graph)
    }

    fn resolve_dependency(
        &self,
        name: &str,
        dep: &Dependency,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let src = DepSource::from_dep_spec(name, dep)?;

        todo!()
    }

    async fn fetch_git_dependency(
        &self,
        name: &str,
        dep: GitDependency<'_>,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let commit = dep.resolve_commit().await?;

        let dest = cache::directory().join(&commit);
        if dest.exists() {
            todo!();
        }

        let url = dep.commit_archive_url(&commit);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ResolverError::GitFetch(dep.repo.to_string(), e))?;

        let tmp = cache::directory().join(format!(".tmp-{commit}"));
        tokio::fs::create_dir_all(&tmp)
            .await
            .map_err(|e| ResolverError::Io(dep.repo.to_string(), e))?;

        let stream = response.bytes_stream().map_err(std::io::Error::other);
        let reader = StreamReader::new(stream);
        let decoder = GzipDecoder::new(reader);
        let buffered = BufReader::new(decoder);
        let mut archive = Archive::new(buffered);
        archive
            .unpack(&tmp)
            .await
            .map_err(|e| ResolverError::Io(dep.repo.to_string(), e))?;

        tokio::fs::rename(&tmp, &dest)
            .await
            .map_err(|e| ResolverError::Io(dep.repo.to_string(), e))?;

        self.node_spec_from(name, &dest)
    }

    fn node_spec_from(
        &self,
        name: &str,
        path: &std::path::Path,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let manifest_path = path.join("plum.toml");
        let manifest_str = std::fs::read_to_string(&manifest_path)
            .map_err(|e| ResolverError::Io(name.to_string(), e))?;
        let manifest = Manifest::parse(&manifest_str)
            .map_err(|e| ResolverError::ManifestParse(name.to_string(), e))?;

        let node_spec = plum_graph::NodeSpec {
            name: manifest.name.clone(),
            path: path.to_path_buf(),
            dependencies: manifest.dependencies.keys().cloned().collect(),
            manifest,
        };
        Ok(node_spec)
    }
}

enum DepSource<'a> {
    Git(GitDependency<'a>),
    Path(&'a str),
    Repo(String),
}

impl<'a> DepSource<'a> {
    fn from_dep_spec(name: &'a str, spec: &'a Dependency) -> Result<Self, ResolverError> {
        match spec {
            Dependency::Version(version) => Ok(DepSource::Repo(version.to_string())),
            Dependency::Detailed(details) => {
                if let Some(git) = &details.git {
                    DepSource::from_git_spec(name, git, details)
                } else if let Some(path) = &details.path {
                    Ok(DepSource::Path(path))
                } else if let Some(url) = &details.url {
                    Ok(DepSource::Repo(url.to_string()))
                } else {
                    Err(ResolverError::NoSource(name.to_string()))
                }
            }
        }
    }

    fn from_git_spec(
        name: &'a str,
        git: &'a str,
        details: &'a DependencySpec,
    ) -> Result<Self, ResolverError> {
        let set: Vec<git::Ref> = [
            details.rev.as_deref().map(git::Ref::Rev),
            details.tag.as_deref().map(git::Ref::Tag),
            details.branch.as_deref().map(git::Ref::Branch),
        ]
        .into_iter()
        .flatten()
        .collect();

        match set.as_slice() {
            [] => Err(ResolverError::GitNoRef(name.to_string())),
            [one] => Ok(DepSource::Git(
                match one {
                    git::Ref::Rev(r) => GitDependency::rev(git, r),
                    git::Ref::Tag(t) => GitDependency::tag(git, t),
                    git::Ref::Branch(b) => GitDependency::branch(git, b),
                }
                .ok_or_else(|| ResolverError::GitUrlParse(name.to_string(), git.to_string()))?,
            )),
            many => Err(ResolverError::ConflictingSpecs(
                name.to_string(),
                many.iter()
                    .map(git::Ref::name)
                    .collect::<Vec<_>>()
                    .join(", "),
            )),
        }
    }
}
