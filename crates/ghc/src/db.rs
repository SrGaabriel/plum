use std::{path::Path, process::Command};

use plum_manifest::Manifest;

use crate::CompilationError;

pub fn register_pkg(path: &Path, manifest: &Manifest) -> Result<(), CompilationError> {
    let mut command = Command::new("ghc-pkg");
    command.arg("register");
    command.arg("--force");
    command.arg("--global");
    command.arg(path.join("plum.conf"));

    let output = command.output()?;
    if !output.status.success() {
        return Err(CompilationError::RegistrationError());
    }

    Ok(())
}
