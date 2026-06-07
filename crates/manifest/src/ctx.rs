use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_dhall::StaticType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Context {
    pub os: OS,
    pub arch: String,
    pub compiler: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OS {
    Windows,
    Linux,
    MacOS,
}

impl StaticType for Context {
    fn static_type() -> serde_dhall::SimpleType {
        serde_dhall::SimpleType::Record(HashMap::from([
            ("os".to_string(), OS::static_type()),
            ("arch".to_string(), serde_dhall::SimpleType::Text),
            ("compiler".to_string(), serde_dhall::SimpleType::Text),
        ]))
    }
}

impl StaticType for OS {
    fn static_type() -> serde_dhall::SimpleType {
        serde_dhall::SimpleType::Union(HashMap::from([
            ("Windows".to_string(), None),
            ("Linux".to_string(), None),
            ("MacOS".to_string(), None),
        ]))
    }
}
