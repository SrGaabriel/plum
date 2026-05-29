use kdl_derive::{FromKdl, ToKdl};

#[derive(Debug, Clone, ToKdl, FromKdl)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    #[kdl(default)]
    pub lib: bool,
}
