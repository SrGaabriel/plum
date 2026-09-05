use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use heck::ToPascalCase;
use plum_graph::{BuildNode, DependencyGraph};
use plum_manifest::BUILD_FOLDER_NAME;
use thiserror::Error;

pub mod db;

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("failed to compile module: {0}")]
    IoError(#[from] std::io::Error),
    #[error("compilation failed for module '{0}' with exit code {1}")]
    CompilationError(String, i32),
    #[error("failed to register package in database: {0}")]
    RegistrationError(),
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub unit_id: String,
    pub package_db: Option<PathBuf>,
    pub transitive_dbs: Vec<PathBuf>,
}

pub fn compile_module(
    node: &BuildNode,
    graph: &DependencyGraph,
    deps_artifacts: &[Artifact],
) -> Result<Artifact, CompilationError> {
    println!("building module: {node:?} with deps: {deps_artifacts:?}");

    let module_path = &node.path;
    let build_folder = module_path.join(BUILD_FOLDER_NAME);
    std::fs::create_dir_all(&build_folder)?;

    let name = graph.resolve(node.name);
    let unit_id = format!("{}-{}", name, node.manifest.version.unwrap_or_default());

    let mut command = Command::new("ghc");
    command.current_dir(module_path);
    command
        .arg("--make")
        .arg("-hide-all-packages")
        .arg("-this-unit-id")
        .arg(&unit_id);

    command.arg("-outputdir").arg(&build_folder);

    let output_file: Option<PathBuf>;
    for dep_artifact in deps_artifacts {}

    if node.manifest.lib {
        output_file = None;
        let lib_name = name.to_pascal_case();
        command.arg("-isrc").arg(lib_name);
    } else {
        let output_str = format!("{BUILD_FOLDER_NAME}/{name}");
        command.arg("-o").arg(&output_str);
        command.arg("-isrc").arg("src/Main.hs");
        output_file = Some(module_path.join(output_str));
    }
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let output = command.output()?;
    println!("output of {command:?} is {output:?}");
    if !output.status.success() {
        return Err(CompilationError::CompilationError(
            name.to_string(),
            output.status.code().unwrap_or(-1),
        ));
    }
    Ok(Artifact {
        unit_id,
    })
}
