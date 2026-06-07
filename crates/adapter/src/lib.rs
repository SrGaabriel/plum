use std::collections::hash_map::Entry;

use fxhash::FxHashMap;
use plum_cabal::{
    BuildInfo, CondTree, Dependency as CabalDependency, Executable, Library, PackageDescription,
    SourceRepo, VersionRange,
};
use plum_manifest::{
    Dependency, Manifest,
    pvp::{Version, VersionReq},
};

pub fn adapt(cabal: PackageDescription) -> Manifest {
    let library = cabal.library;
    let lib = library.is_some();

    let mut library_infos = Vec::new();
    if let Some(tree) = library {
        collect_build_info(tree, &|lib: Library| lib.build_info, &mut library_infos);
    }

    let mut component_infos = Vec::new();
    for (_, tree) in cabal.sub_libraries {
        collect_build_info(tree, &|lib: Library| lib.build_info, &mut component_infos);
    }
    for (_, tree) in cabal.executables {
        collect_build_info(
            tree,
            &|exe: Executable| exe.build_info,
            &mut component_infos,
        );
    }

    let mut ghc_options = Vec::new();
    let mut ranges: FxHashMap<String, VersionRange> = FxHashMap::default();
    for info in library_infos {
        ghc_options.extend(info.ghc_options);
        merge_dependencies(&mut ranges, info.build_depends);
    }
    for info in component_infos {
        merge_dependencies(&mut ranges, info.build_depends);
    }

    let dependencies = ranges
        .into_iter()
        .map(|(name, range)| (name, Dependency::Version(reduce_range(range))))
        .collect();

    Manifest {
        name: cabal.name,
        version: cabal.version,
        description: cabal.description.or(cabal.synopsis),
        repository: pick_repository(cabal.source_repos),
        license: cabal.license,
        lib,
        ghc_options,
        dependencies,
    }
}

fn collect_build_info<T, F: Fn(T) -> BuildInfo>(
    tree: CondTree<T>,
    project: &F,
    out: &mut Vec<BuildInfo>,
) {
    out.push(project(tree.data));
    for branch in tree.branches {
        collect_build_info(branch.then_tree, project, out);
        if let Some(else_tree) = branch.else_tree {
            collect_build_info(else_tree, project, out);
        }
    }
}

fn merge_dependencies(ranges: &mut FxHashMap<String, VersionRange>, deps: Vec<CabalDependency>) {
    for dep in deps {
        match ranges.entry(dep.name) {
            Entry::Occupied(slot) => {
                let slot = slot.into_mut();
                let previous = std::mem::replace(slot, VersionRange::Any);
                *slot = VersionRange::Intersection(Box::new(previous), Box::new(dep.range));
            }
            Entry::Vacant(slot) => {
                slot.insert(dep.range);
            }
        }
    }
}

fn pick_repository(repos: Vec<SourceRepo>) -> Option<String> {
    let mut head = None;
    let mut fallback = None;
    for repo in repos {
        let Some(location) = repo.location else {
            continue;
        };
        if repo.kind.eq_ignore_ascii_case("head") {
            head.get_or_insert(location);
        } else if fallback.is_none() {
            fallback = Some(location);
        }
    }
    head.or(fallback)
}

fn reduce_range(range: VersionRange) -> VersionReq {
    match range {
        VersionRange::Any => VersionReq::GreaterEq(Version::lowest()),
        VersionRange::None => VersionReq::Less(Version::lowest()),
        VersionRange::This(v) => VersionReq::Exact(v),
        VersionRange::Earlier(v) => VersionReq::Less(v),
        VersionRange::EarlierEqual(v) => VersionReq::LessEq(v),
        VersionRange::Later(v) => VersionReq::Greater(v),
        VersionRange::LaterEqual(v) => VersionReq::GreaterEq(v),
        VersionRange::Wildcard(v) | VersionRange::Caret(v) => VersionReq::Caret(v),
        VersionRange::Intersection(a, b) => reduce_range(*a).intersect(reduce_range(*b)),
        VersionRange::Union(a, b) => reduce_range(*a).union(reduce_range(*b)),
    }
}
