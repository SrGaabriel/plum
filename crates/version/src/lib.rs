use std::{cmp::Ordering, fmt::Display, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub components: Box<[u64]>,
}

impl Version {
    pub fn lowest() -> Self {
        Version {
            components: Box::new([]),
        }
    }

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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
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

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(serde::de::Error::custom)
    }
}
