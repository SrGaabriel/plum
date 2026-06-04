pub mod pvp;

use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::pvp::{Version, VersionReq};

pub const MANIFEST_FILE: &str = "plum.dhall";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub lib: bool,
    pub dependencies: FxHashMap<String, Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    Version(VersionReq),
    Detailed(DependencySpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    pub git: Option<String>,
    pub path: Option<String>,
    pub rev: Option<String>,
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub version: Option<VersionReq>,
}

pub type Error = Box<serde_dhall::Error>;

impl Manifest {
    pub fn parse(content: &str) -> Result<Self, Error> {
        let manifest: Self = serde_dhall::from_str(content).parse()?;
        Ok(manifest)
    }
}
