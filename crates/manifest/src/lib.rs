use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub lib: bool,
    pub dependencies: FxHashMap<String, String>,
}

impl Manifest {
    pub fn parse(content: &str) -> Result<Self, serde_dhall::Error> {
        let manifest: Self = serde_dhall::from_str(content).parse()?;
        Ok(manifest)
    }
}
