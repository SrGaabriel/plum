use std::path::{Path, PathBuf};

use crate::style;

pub fn project_dir(manifest_path: &Path) -> PathBuf {
    let manifest_path = manifest_path.canonicalize().unwrap_or_else(|err| {
        style::error(format!("could not resolve manifest path: {err}"));
        std::process::exit(1);
    });
    manifest_path
        .parent()
        .unwrap_or_else(|| {
            style::error("manifest file must be in a directory");
            std::process::exit(1);
        })
        .to_path_buf()
}

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("{n} {word}")
    } else {
        format!("{n} {word}s")
    }
}
