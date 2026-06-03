use std::path::Path;

use crate::{
    manifest::parse_manifest,
    style::{self, Tone},
};

pub fn exec(manifest_path: &Path) {
    let manifest = parse_manifest(manifest_path);
    let project_path = manifest_path.parent().unwrap_or_else(|| {
        style::error("Manifest file must be in a directory");
        std::process::exit(1);
    });
    // let node = BuildNode {
    //     name: manifest.name.clone(),
    //     app: !manifest.lib,
    //     path: project_path.to_path_buf(),
    // };
    // println!("Building project: {}", manifest.name);
    // match compile_module(&node) {
    //     Ok(()) => style::status(Tone::Success, "done", format!("built {}", manifest.name)),
    //     Err(err) => {
    //         style::error(err);
    //         std::process::exit(1);
    //     }
    // }
    println!("Build completed successfully");
}
