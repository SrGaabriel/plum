use std::{path::Path, process::exit};

use plum_manifest::Manifest;

use crate::style;

pub fn parse_manifest(path: &Path) -> Manifest {
    if !path.exists() {
        style::error(format!("manifest file not found: {}", path.display()));
        exit(1);
    }
    let content = std::fs::read_to_string(path).unwrap_or_else(|err| {
        style::error(format!("failed to read manifest file: {err}"));
        exit(1);
    });
    Manifest::parse(&content).unwrap_or_else(|err| {
        style::error(format!("failed to parse manifest file: {err}"));
        exit(1);
    })
}
