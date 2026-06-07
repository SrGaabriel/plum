mod cache;
mod git;
mod hackage;

use std::path::{Path, PathBuf};

use async_compression::tokio::bufread::GzipDecoder;
use futures_util::TryStreamExt;
use plum_graph::DependencyGraph;
use plum_manifest::{
    Dependency, DependencySpec, Manifest,
    ctx::Context,
    pvp::{Version, VersionReq},
};
use reqwest::{Client, ClientBuilder, Response};
use tokio::io::BufReader;
use tokio_tar::Archive;
use tokio_util::io::StreamReader;

use crate::git::GitDependency;

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error(transparent)]
    GraphError(#[from] plum_graph::GraphError),
    #[error("io error for '{0}': {1}")]
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
    #[error("version mismatch for '{0}': expected {1}, found {2}")]
    VersionMismatch(String, VersionReq, Version),
    #[error("no plum manifest or cabal file found in '{0}'")]
    NoManifest(String),
    #[error("'{0}' has multiple cabal files; a package must have exactly one: {1}")]
    MultipleCabalFiles(String, String),
    #[error("cabal parse error for '{0}': {1}")]
    CabalParse(String, #[source] Box<plum_cabal::Error>),
}

#[derive(Debug, Clone)]
pub struct Resolver {
    context: Context,
    client: Client,
}

impl Resolver {
    pub fn new(context: Context) -> Self {
        let client = ClientBuilder::new()
            .build()
            .expect("failed to build HTTP client");
        Self { context, client }
    }

    pub async fn resolve_dependencies(
        &self,
        manifest: &Manifest,
    ) -> Result<DependencyGraph, ResolverError> {
        let mut graph_builder = plum_graph::DependencyGraphBuilder::new();
        for (name, dep) in &manifest.dependencies {
            let node_spec = self.resolve_dependency(name, dep).await?;
            graph_builder.add(node_spec);
        }
        let graph = graph_builder.build()?;
        Ok(graph)
    }

    async fn resolve_dependency(
        &self,
        name: &str,
        dep: &Dependency,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let src = DepSource::from_dep_spec(name, dep)?;

        let dep = match src {
            DepSource::Git(git_dep, version) => {
                self.fetch_git_dependency(name, version, git_dep).await
            }
            DepSource::Path(path, version) => self.node_spec_from(name, version, &path),
            DepSource::Repo(version) => self.fetch_repo_dependency(name, version).await,
        }?;

        Ok(dep)
    }

    async fn fetch_git_dependency(
        &self,
        name: &str,
        version: Option<&VersionReq>,
        dep: GitDependency<'_>,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let commit = dep.resolve_commit().await?;

        let dest = cache::directory().join(&commit);
        if dest.exists() {
            return self.node_spec_from(name, version, &dest);
        }

        let url = dep.commit_archive_url(&commit);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| ResolverError::GitFetch(dep.repo.to_string(), e))?;

        self.download_and_unpack(name, &commit, version, response)
            .await
    }

    async fn fetch_repo_dependency(
        &self,
        name: &str,
        version: &VersionReq,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        todo!()
    }

    async fn download_and_unpack(
        &self,
        name: &str,
        hash: &str,
        version: Option<&VersionReq>,
        response: Response,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let tmp = cache::directory().join(format!(".tmp-{name}"));
        tokio::fs::create_dir_all(&tmp)
            .await
            .map_err(|e| ResolverError::Io(name.to_string(), e))?;

        let stream = response.bytes_stream().map_err(std::io::Error::other);
        let reader = StreamReader::new(stream);
        let decoder = GzipDecoder::new(reader);
        let buffered = BufReader::new(decoder);
        let mut archive = Archive::new(buffered);
        archive
            .unpack(&tmp)
            .await
            .map_err(|e| ResolverError::Io(name.to_string(), e))?;

        let dest = cache::directory().join(hash);
        tokio::fs::rename(tmp, &dest)
            .await
            .map_err(|e| ResolverError::Io(name.to_string(), e))?;

        self.node_spec_from(name, version, &dest)
    }

    fn node_spec_from(
        &self,
        name: &str,
        version: Option<&VersionReq>,
        path: &std::path::Path,
    ) -> Result<plum_graph::NodeSpec, ResolverError> {
        let manifest = self.read_manifest(path)?;
        if let Some(version_req) = version
            && let Some(manifest_version) = &manifest.version
            && !version_req.matches(manifest_version)
        {
            return Err(ResolverError::VersionMismatch(
                name.to_string(),
                version_req.clone(),
                manifest_version.clone(),
            ));
        }

        let node_spec = plum_graph::NodeSpec {
            name: manifest.name.clone(),
            path: path.to_path_buf(),
            dependencies: manifest.dependencies.keys().cloned().collect(),
            manifest,
        };
        Ok(node_spec)
    }

    pub fn read_manifest(&self, root: &Path) -> Result<Manifest, ResolverError> {
        let manifest_path = root.join(plum_manifest::MANIFEST_FILE);
        if manifest_path.exists() {
            let manifest_str = std::fs::read_to_string(&manifest_path)
                .map_err(|e| ResolverError::Io(path_str(root), e))?;
            return Manifest::parse(&self.context, &manifest_str)
                .map_err(|e| ResolverError::ManifestParse(path_str(root), e));
        }

        let Some(cabal_path) = find_cabal_file(root)? else {
            return Err(ResolverError::NoManifest(path_str(root)));
        };
        let cabal_str = std::fs::read_to_string(&cabal_path)
            .map_err(|e| ResolverError::Io(path_str(&cabal_path), e))?;
        let name = cabal_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("package.cabal");
        let package = plum_cabal::parse_named(&cabal_str, name)
            .map_err(|e| ResolverError::CabalParse(path_str(&cabal_path), e))?;
        Ok(plum_adapter::adapt(package))
    }
}

fn find_cabal_file(root: &Path) -> Result<Option<PathBuf>, ResolverError> {
    let mut cabals = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| ResolverError::Io(path_str(root), e))? {
        let path = entry
            .map_err(|e| ResolverError::Io(path_str(root), e))?
            .path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("cabal"))
        {
            cabals.push(path);
        }
    }
    cabals.sort();
    match cabals.len() {
        0 => Ok(None),
        1 => Ok(cabals.pop()),
        _ => {
            let names = cabals
                .iter()
                .map(|p| path_str(p))
                .collect::<Vec<_>>()
                .join(", ");
            Err(ResolverError::MultipleCabalFiles(path_str(root), names))
        }
    }
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

enum DepSource<'a> {
    Git(GitDependency<'a>, Option<&'a VersionReq>),
    Path(PathBuf, Option<&'a VersionReq>),
    Repo(&'a VersionReq),
}

impl<'a> DepSource<'a> {
    fn from_dep_spec(name: &'a str, spec: &'a Dependency) -> Result<Self, ResolverError> {
        match spec {
            Dependency::Version(version) => Ok(DepSource::Repo(version)),
            Dependency::Detailed(details) => {
                if let Some(git) = &details.git {
                    DepSource::from_git_spec(name, git, details)
                } else if let Some(path) = &details.path {
                    let path = match Path::new(path).canonicalize() {
                        Ok(p) => p,
                        Err(e) => {
                            return Err(ResolverError::Io(name.to_string(), e));
                        }
                    };
                    Ok(DepSource::Path(path, details.version.as_ref()))
                } else if let Some(version) = &details.version {
                    Ok(DepSource::Repo(version))
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
                details.version.as_ref(),
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
