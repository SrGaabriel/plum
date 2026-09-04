mod build;
mod clean;
mod graph;
mod manifest;
mod style;
mod util;

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
    Graph {
        #[clap(long, default_value = "plum.dhall")]
        manifest: PathBuf,
        #[clap(subcommand)]
        graph: Option<Graph>,
    },
    Clean {
        #[clap(long, default_value = "plum.dhall")]
        manifest: PathBuf,
    },
}

#[derive(Debug, clap::Subcommand)]
enum Graph {
    Search { name: String },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Build { manifest } => build::exec(&manifest).await,
        Command::Clean { manifest } => clean::exec(&manifest),
        Command::Graph {
            manifest,
            graph: None,
        } => graph::exec(&manifest).await,
        Command::Graph {
            manifest,
            graph: Some(Graph::Search { name }),
        } => graph::search(&manifest, name).await,
    }
}
