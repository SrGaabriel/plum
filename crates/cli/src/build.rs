use std::path::Path;

use crate::{
    manifest::{derive_context, parse_manifest},
    style::{self, Tone},
};

pub async fn exec(manifest_path: &Path) {
    let context = derive_context();
    let manifest = parse_manifest(&context, manifest_path);
    println!("Manifest: {manifest:?}");
    let project_path = manifest_path.parent().unwrap_or_else(|| {
        style::error("Manifest file must be in a directory");
        std::process::exit(1);
    });

    let result = plum_build::build(context, &manifest).await;
    match result {
        Ok(summary) => {
            println!("built {} modules", summary.built);
        }
        Err(err) => {
            style::error(format!("build failed: {err}"));
            std::process::exit(1);
        }
    }
}
