use std::{path::Path, process::exit};

use plum_manifest::{Manifest, ctx::{Context, OS}};

use crate::style;

pub fn parse_manifest(context: &Context, path: &Path) -> Manifest {
    if !path.exists() {
        style::error(format!("manifest file not found: {}", path.display()));
        exit(1);
    }
    let content = std::fs::read_to_string(path).unwrap_or_else(|err| {
        style::error(format!("failed to read manifest file: {err}"));
        exit(1);
    });
    Manifest::parse(&context, &content).unwrap_or_else(|err| {
        style::error(format!("failed to parse manifest file: {err}"));
        exit(1);
    })
}

pub fn derive_context() -> Context {
    let os = if cfg!(target_os = "windows") {
        OS::Windows
    } else if cfg!(target_os = "macos") {
        OS::MacOS
    } else if cfg!(target_os = "linux") {
        OS::Linux
    } else {
        style::error(format!("unsupported operating system: {}", std::env::consts::OS));
        exit(1);
    };
    let arch = std::env::consts::ARCH.to_string();
    let compiler = "todo".to_string();
    Context { os, arch, compiler }
}
