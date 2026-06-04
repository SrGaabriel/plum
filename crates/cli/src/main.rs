mod build;
mod manifest;
mod style;

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, clap::Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Build {
        #[clap(long, default_value = "plum.dhall")]
        manifest: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { manifest } => build::exec(&manifest),
    }
}
