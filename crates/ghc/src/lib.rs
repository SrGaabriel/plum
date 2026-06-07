use std::process::{Command, Stdio};

use heck::ToPascalCase;
use plum_graph::{BuildNode, DependencyGraph};
use thiserror::Error;

const BUILD_FOLDER_NAME: &str = "target";

#[derive(Debug, Error)]
pub enum CompilationError {
    #[error("failed to compile module: {0}")]
    IoError(#[from] std::io::Error),
    #[error("compilation failed for module '{0}' with exit code {1}")]
    CompilationError(String, i32),
}

pub fn compile_module(node: &BuildNode, graph: &DependencyGraph) -> Result<(), CompilationError> {
    let module_path = &node.path;
    let name = graph.resolve(node.name);

    let mut command = Command::new("ghc");
    command.current_dir(module_path);
    command.arg("-outputdir").arg(BUILD_FOLDER_NAME);
    if node.manifest.lib {
        let lib_name = name.to_pascal_case();
        command.arg("-isrc").arg(lib_name);
    } else {
        command.arg("-isrc").arg("src/Main.hs");
    }
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let output = command.output()?;
    if !output.status.success() {
        return Err(CompilationError::CompilationError(
            name.to_string(),
            output.status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}
