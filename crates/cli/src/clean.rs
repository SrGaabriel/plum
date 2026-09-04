use std::path::Path;

use plum_manifest::BUILD_FOLDER_NAME;

use crate::{
    manifest::{derive_context, parse_manifest},
    style::{self},
};

pub fn exec(manifest_path: &Path) {
    let context = derive_context();
    let _ = parse_manifest(&context, manifest_path);
    let project_path = manifest_path.parent().unwrap_or_else(|| {
        style::error("manifest file must be in a directory");
        std::process::exit(1);
    });

    let artifacts_folder = project_path.join(BUILD_FOLDER_NAME);

    if !artifacts_folder.exists() {
        return style::success(format!(
            "build folder already clean: {}",
            artifacts_folder.display()
        ));
    }
    let scan_progress = style::spinner(format!(
        "scanning artifacts folder: {}",
        artifacts_folder.display()
    ));
    let dir_content = fs_extra::dir::get_dir_content(&artifacts_folder).unwrap_or_else(|err| {
        style::error(format!(
            "failed to get artifacts folder content: {}: {err}",
            artifacts_folder.display()
        ));
        std::process::exit(1);
    });
    scan_progress.finish_and_clear();

    let progress = style::spinner(format!(
        "removing artifacts folder: {} ({} bytes, {} files)",
        artifacts_folder.display(),
        dir_content.dir_size,
        dir_content.files.len(),
    ));
    fs_extra::dir::remove(&artifacts_folder).unwrap_or_else(|err| {
        progress.finish_and_clear();
        style::error(format!(
            "failed to remove artifacts folder: {}: {err}",
            artifacts_folder.display()
        ));
        std::process::exit(1);
    });
    progress.finish_and_clear();

    style::success(format!(
        "removed artifacts folder: {} ({} bytes, {} files)",
        artifacts_folder.display(),
        dir_content.dir_size,
        dir_content.files.len(),
    ));
}
