use std::{path::PathBuf, process::{Command, Stdio}};

use heck::ToPascalCase;
use thiserror::Error;

const BUILD_FOLDER_NAME: &str = "target";

pub struct BuildNode {
    pub name: String,
    pub path: PathBuf,
    pub app: bool,
}

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("failed to compile module: {0}")]
    IoError(#[from] std::io::Error),
    #[error("compilation failed for module '{0}' with exit code {1}")]
    CompilationError(String, i32),
}

pub fn compile_module(node: &BuildNode) -> Result<(), BuildError> {
    let module_path = &node.path;

    let mut command = Command::new("ghc");
    command.current_dir(module_path);
    command.arg("-outputdir").arg(BUILD_FOLDER_NAME);
    if node.app {
        command.arg("-isrc").arg("src/Main.hs");
    } else {
        let lib_name = node.name.to_pascal_case();
        command.arg("-isrc").arg(lib_name);
    }
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let output = command.output()?;
    if !output.status.success() {
        return Err(BuildError::CompilationError(
            node.name.clone(),
            output.status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}
