use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

pub use plum_version::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReq {
    Exact(Version),
    Caret(Version),
    LessEq(Version),
    Less(Version),
    GreaterEq(Version),
    Greater(Version),
}

impl VersionReq {
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionReq::Exact(v) => version == v,
            VersionReq::Caret(v) => {
                if version.major1() != v.major1() || version.major2() != v.major2() {
                    return false;
                }
                version >= v
            }
            VersionReq::LessEq(v) => version <= v,
            VersionReq::Less(v) => version < v,
            VersionReq::GreaterEq(v) => version >= v,
            VersionReq::Greater(v) => version > v,
        }
    }

    pub fn union(self, other: VersionReq) -> VersionReq {
        match (self.lower_bound(), other.lower_bound()) {
            (Some(va), Some(vb)) => {
                if va <= vb {
                    self
                } else {
                    other
                }
            }
            _ => VersionReq::GreaterEq(Version::lowest()),
        }
    }

    pub fn intersect(self, other: VersionReq) -> VersionReq {
        match (self.lower_bound(), other.lower_bound()) {
            (Some(va), Some(vb)) => {
                if va >= vb {
                    self
                } else {
                    other
                }
            }
            (Some(_), None) => self,
            (None, Some(_)) => other,
            (None, None) => match (self.upper_bound(), other.upper_bound()) {
                (Some(va), Some(vb)) if vb < va => other,
                _ => self,
            },
        }
    }

    pub fn upper_bound(&self) -> Option<&Version> {
        match self {
            VersionReq::Less(v) | VersionReq::LessEq(v) => Some(v),
            VersionReq::Exact(_)
            | VersionReq::Caret(_)
            | VersionReq::GreaterEq(_)
            | VersionReq::Greater(_) => None,
        }
    }

    pub fn lower_bound(&self) -> Option<&Version> {
        match self {
            VersionReq::Exact(v)
            | VersionReq::Caret(v)
            | VersionReq::GreaterEq(v)
            | VersionReq::Greater(v) => Some(v),
            VersionReq::Less(_) | VersionReq::LessEq(_) => None,
        }
    }
}

impl FromStr for VersionReq {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.replace(' ', "");

        let op = s
            .chars()
            .take_while(|c| !c.is_ascii_digit())
            .collect::<String>();
        let version_str = s[op.len()..].trim();
        let version = Version::from_str(version_str).map_err(|e| e.to_string())?;
        match op.as_str() {
            "^" => Ok(VersionReq::Caret(version)),
            "<=" => Ok(VersionReq::LessEq(version)),
            "<" => Ok(VersionReq::Less(version)),
            ">=" => Ok(VersionReq::GreaterEq(version)),
            ">" => Ok(VersionReq::Greater(version)),
            "" => Ok(VersionReq::Exact(version)),
            _ => unreachable!(),
        }
    }
}

impl Display for VersionReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionReq::Exact(v) => write!(f, "{v}"),
            VersionReq::Caret(v) => write!(f, "^ {v}"),
            VersionReq::LessEq(v) => write!(f, "<= {v}"),
            VersionReq::Less(v) => write!(f, "< {v}"),
            VersionReq::GreaterEq(v) => write!(f, ">= {v}"),
            VersionReq::Greater(v) => write!(f, "> {v}"),
        }
    }
}

impl Serialize for VersionReq {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        VersionReq::from_str(&s).map_err(serde::de::Error::custom)
    }
}
