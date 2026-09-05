use std::path::Path;

use crate::{
    manifest::{derive_context, parse_manifest},
    style::{self},
    util::project_dir,
};

pub async fn exec(manifest_path: &Path) {
    let context = derive_context();
    let manifest = parse_manifest(&context, manifest_path);
    println!("Manifest: {manifest:?}");
    let project_path = project_dir(manifest_path);

    let result = plum_build::build(context, &manifest, &project_path).await;
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
