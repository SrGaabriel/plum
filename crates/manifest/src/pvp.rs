use std::{cmp::Ordering, fmt::Display, num::ParseIntError, str::FromStr};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub components: Box<[u64]>,
}

impl Version {
    pub fn major1(&self) -> Option<u64> {
        self.components.first().copied()
    }

    pub fn major2(&self) -> Option<u64> {
        self.components.get(1).copied()
    }

    pub fn patch(&self) -> Option<u64> {
        self.components.get(2).copied()
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.components.len().max(other.components.len());
        for i in 0..max_len {
            let a = self.components.get(i).copied();
            let b = other.components.get(i).copied();
            if a.cmp(&b) != Ordering::Equal {
                return a.cmp(&b);
            }
        }
        Ordering::Equal
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .components
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(".");
        write!(f, "{s}")
    }
}

impl FromStr for Version {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let components = s
            .split('.')
            .map(str::parse)
            .collect::<Result<Box<[u64]>, _>>()?;
        Ok(Version { components })
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(serde::de::Error::custom)
    }
}

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

impl<'de> serde::Deserialize<'de> for VersionReq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        VersionReq::from_str(&s).map_err(serde::de::Error::custom)
    }
}
