use std::{path::Path, process::exit};

use miette::Report;
use plum_manifest::Manifest;

use crate::style;

pub fn parse_manifest(path: &Path) -> Manifest {
    if !path.exists() {
        style::error(format!("Manifest file not found: {}", path.display()));
        exit(1);
    }
    let content = std::fs::read_to_string(path).unwrap_or_else(|err| {
        style::error(format!("Failed to read manifest file: {err}"));
        exit(1);
    });
    Manifest::from_kdl_str(&content).unwrap_or_else(|err| {
        let report = Report::new(err).with_source_code(content);
        style::error("Failed to parse manifest file");
        eprintln!("{report:?}");
        exit(1);
    })
}
