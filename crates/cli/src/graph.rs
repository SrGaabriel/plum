use std::fmt::Write as _;
use std::path::Path;

use mago_pager::Pager;
use plum_graph::{DependencyGraph, NodeIndex};
use plum_resolver::Resolver;

use crate::manifest::{derive_context, parse_manifest};
use crate::style::{self, Painter, Style};
use crate::util::{plural, project_dir};

pub async fn exec(manifest_path: &Path) {
    let context = derive_context();
    let manifest = parse_manifest(&context, manifest_path);
    let project_path = project_dir(manifest_path);

    let resolver = Resolver::new(context);
    let spinner = style::spinner("resolving dependencies");
    let resolved = resolver.resolve_dependencies(&manifest, &project_path).await;
    spinner.finish_and_clear();
    let graph = resolved.unwrap_or_else(|err| {
        style::error(format!("could not generate dependency graph: {err}"));
        std::process::exit(1);
    });

    let listing = render(&graph, Painter::auto());

    let pager_env = if std::env::var_os("LESS").is_none() {
        vec![("LESS", "FRX")]
    } else {
        Vec::new()
    };
    let pager = Pager::new().envs(pager_env).spawn().ok();
    print!("{listing}");
    drop(pager);
}

const INDENT: usize = 2;

pub fn render(graph: &DependencyGraph, palette: Painter) -> String {
    let layers = graph.layers();
    let height = layers.len();
    let gutter = height.to_string().len();

    let mut out = String::new();
    out.push('\n');
    out.push_str(&headline(graph, palette));
    out.push('\n');

    for (depth, layer) in layers.iter().enumerate().rev() {
        out.push('\n');
        let indent = (height - 1 - depth) * INDENT;
        for (i, &n) in layer.iter().enumerate() {
            let number = if i == 0 {
                palette.dim(format!("{:>gutter$}", depth + 1))
            } else {
                " ".repeat(gutter)
            };
            let label = label(graph, n, palette);
            let _ = write!(out, "  {number}  {}{label}", " ".repeat(indent));

            let mut deps: Vec<&str> = graph.dependencies(n).map(|d| graph.node_name(d)).collect();
            if deps.is_empty() {
                out.push('\n');
                continue;
            }
            deps.sort_unstable();
        }
    }
    out
}

fn headline(graph: &DependencyGraph, palette: Painter) -> String {
    let mut sinks: Vec<NodeIndex> = graph.sinks().collect();
    sinks.sort_by_key(|&n| graph.node_name(n));
    let subject = sinks
        .iter()
        .map(|&n| label(graph, n, palette))
        .collect::<Vec<_>>()
        .join(&palette.dim(", "));

    let stats = plural(graph.node_count(), "package");
    format!("{subject} - {}", palette.dim(stats))
}

fn label(graph: &DependencyGraph, n: NodeIndex, palette: Painter) -> String {
    let name = if graph.is_sink(n) {
        palette.paint(graph.node_name(n), Style::new().bold().cyan())
    } else {
        palette.paint(graph.node_name(n), Style::new().bold())
    };
    match &graph.node(n).manifest.version {
        Some(version) => format!("{name} {}", palette.dim(version)),
        None => name,
    }
}

pub async fn search(manifest_path: &Path, name: String) {
    let context = derive_context();
    let manifest = parse_manifest(&context, manifest_path);

    todo!()
}
