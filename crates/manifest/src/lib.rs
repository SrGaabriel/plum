pub mod ctx;
pub mod pvp;

use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    ctx::Context,
    pvp::{Version, VersionReq},
};

pub const MANIFEST_FILE: &str = "plum.dhall";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_version")]
    pub version: Option<Version>,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub lib: bool,
    #[serde(rename = "ghcOptions", default)]
    pub ghc_options: Vec<String>,
    #[serde(default)]
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

fn deserialize_version<'de, D>(deserializer: D) -> Result<Option<Version>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Version::deserialize(deserializer).map(Some)
}

impl Manifest {
    pub fn parse(ctx: &Context, content: &str) -> Result<Self, Error> {
        let context_str = serde_dhall::serialize(ctx)
            .static_type_annotation()
            .to_string()?;
        let applied = format!("({content}) ({context_str})");
        let manifest: Self = serde_dhall::from_str(&applied).parse()?;
        Ok(manifest)
    }
}
