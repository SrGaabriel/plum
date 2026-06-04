use crate::description::{
    Benchmark, BuildInfo, CondTree, Dependency, Executable, Flag, Library, PackageDescription,
    SourceRepo, TestSuite,
};
use crate::syntax::Field;

pub fn lower(pkg: &PackageDescription) -> Vec<Field> {
    let mut out = Vec::new();

    if let Some(v) = &pkg.cabal_version {
        single(&mut out, "cabal-version", v.to_string());
    }
    single(&mut out, "name", pkg.name.clone());
    if let Some(v) = &pkg.version {
        single(&mut out, "version", v.to_string());
    }
    text(&mut out, "synopsis", pkg.synopsis.as_deref());
    text(&mut out, "description", pkg.description.as_deref());
    text(&mut out, "homepage", pkg.homepage.as_deref());
    text(&mut out, "bug-reports", pkg.bug_reports.as_deref());
    text(&mut out, "package-url", pkg.package_url.as_deref());
    text(&mut out, "license", pkg.license.as_deref());
    match pkg.license_files.as_slice() {
        [] => {}
        [one] => single(&mut out, "license-file", one.clone()),
        many => list(&mut out, "license-files", many),
    }
    text(&mut out, "copyright", pkg.copyright.as_deref());
    text(&mut out, "author", pkg.author.as_deref());
    text(&mut out, "maintainer", pkg.maintainer.as_deref());
    text(&mut out, "stability", pkg.stability.as_deref());
    text(&mut out, "category", pkg.category.as_deref());
    if let Some(bt) = &pkg.build_type {
        single(&mut out, "build-type", bt.to_string());
    }
    if !pkg.tested_with.is_empty() {
        out.push(Field::leaf("tested-with", comma_lines(&pkg.tested_with)));
    }
    text(&mut out, "data-dir", pkg.data_dir.as_deref());
    list(&mut out, "data-files", &pkg.data_files);
    list(&mut out, "extra-source-files", &pkg.extra_source_files);
    list(&mut out, "extra-doc-files", &pkg.extra_doc_files);

    for flag in &pkg.flags {
        out.push(lower_flag(flag));
    }
    for repo in &pkg.source_repos {
        out.push(lower_source_repo(repo));
    }
    if let Some(tree) = &pkg.library {
        out.push(section_tree("library", String::new(), tree, &lower_library));
    }
    for (name, tree) in &pkg.sub_libraries {
        out.push(section_tree("library", name.clone(), tree, &lower_library));
    }
    for (name, tree) in &pkg.executables {
        out.push(section_tree(
            "executable",
            name.clone(),
            tree,
            &lower_executable,
        ));
    }
    for (name, tree) in &pkg.test_suites {
        out.push(section_tree(
            "test-suite",
            name.clone(),
            tree,
            &lower_test_suite,
        ));
    }
    for (name, tree) in &pkg.benchmarks {
        out.push(section_tree(
            "benchmark",
            name.clone(),
            tree,
            &lower_benchmark,
        ));
    }

    out
}

fn section_tree<T>(
    section: &str,
    arg: String,
    tree: &CondTree<T>,
    lower_data: &dyn Fn(&T, &mut Vec<Field>),
) -> Field {
    let mut body = Vec::new();
    lower_tree(tree, lower_data, &mut body);
    Field::section(section, arg, body)
}

fn lower_tree<T>(
    tree: &CondTree<T>,
    lower_data: &dyn Fn(&T, &mut Vec<Field>),
    out: &mut Vec<Field>,
) {
    lower_data(&tree.data, out);
    for branch in &tree.branches {
        let mut then_fields = Vec::new();
        lower_tree(&branch.then_tree, lower_data, &mut then_fields);
        out.push(Field::section(
            "if",
            branch.condition.to_string(),
            then_fields,
        ));
        if let Some(else_tree) = &branch.else_tree {
            let mut else_fields = Vec::new();
            lower_tree(else_tree, lower_data, &mut else_fields);
            out.push(Field::section("else", String::new(), else_fields));
        }
    }
}

fn lower_library(lib: &Library, out: &mut Vec<Field>) {
    list(out, "exposed-modules", &lib.exposed_modules);
    if !lib.reexported_modules.is_empty() {
        out.push(Field::leaf(
            "reexported-modules",
            comma_lines(&lib.reexported_modules),
        ));
    }
    if let Some(v) = lib.visibility {
        single(out, "visibility", v.to_string());
    }
    lower_build_info(&lib.build_info, out);
}

fn lower_executable(exe: &Executable, out: &mut Vec<Field>) {
    if let Some(m) = &exe.main_is {
        single(out, "main-is", m.clone());
    }
    if let Some(s) = exe.scope {
        single(out, "scope", s.to_string());
    }
    lower_build_info(&exe.build_info, out);
}

fn lower_test_suite(test: &TestSuite, out: &mut Vec<Field>) {
    if let Some(k) = &test.kind {
        single(out, "type", k.to_string());
    }
    if let Some(m) = &test.main_is {
        single(out, "main-is", m.clone());
    }
    if let Some(m) = &test.test_module {
        single(out, "test-module", m.clone());
    }
    lower_build_info(&test.build_info, out);
}

fn lower_benchmark(bench: &Benchmark, out: &mut Vec<Field>) {
    if let Some(k) = &bench.kind {
        single(out, "type", k.to_string());
    }
    if let Some(m) = &bench.main_is {
        single(out, "main-is", m.clone());
    }
    lower_build_info(&bench.build_info, out);
}

fn lower_build_info(bi: &BuildInfo, out: &mut Vec<Field>) {
    if !bi.build_depends.is_empty() {
        out.push(Field::leaf("build-depends", dep_lines(&bi.build_depends)));
    }
    list(out, "other-modules", &bi.other_modules);
    list(out, "hs-source-dirs", &bi.hs_source_dirs);
    if let Some(lang) = &bi.default_language {
        single(out, "default-language", lang.to_string());
    }
    display_list(out, "other-languages", &bi.other_languages);
    list(out, "default-extensions", &bi.default_extensions);
    list(out, "other-extensions", &bi.other_extensions);
    words(out, "ghc-options", &bi.ghc_options);
    words(out, "ghc-prof-options", &bi.ghc_prof_options);
    words(out, "cpp-options", &bi.cpp_options);
    words(out, "cc-options", &bi.cc_options);
    words(out, "cxx-options", &bi.cxx_options);
    words(out, "ld-options", &bi.ld_options);
    list(out, "c-sources", &bi.c_sources);
    list(out, "cxx-sources", &bi.cxx_sources);
    list(out, "js-sources", &bi.js_sources);
    list(out, "extra-libraries", &bi.extra_libraries);
    list(out, "extra-lib-dirs", &bi.extra_lib_dirs);
    list(out, "include-dirs", &bi.include_dirs);
    list(out, "includes", &bi.includes);
    list(out, "install-includes", &bi.install_includes);
    if !bi.pkgconfig_depends.is_empty() {
        out.push(Field::leaf(
            "pkgconfig-depends",
            comma_lines(&bi.pkgconfig_depends),
        ));
    }
    if !bi.build_tool_depends.is_empty() {
        out.push(Field::leaf(
            "build-tool-depends",
            comma_lines(&bi.build_tool_depends),
        ));
    }
    list(out, "frameworks", &bi.frameworks);
    if let Some(b) = bi.buildable {
        single(out, "buildable", bool_str(b));
    }
}

fn lower_flag(flag: &Flag) -> Field {
    let mut body = Vec::new();
    text(&mut body, "description", flag.description.as_deref());
    single(&mut body, "default", bool_str(flag.default));
    single(&mut body, "manual", bool_str(flag.manual));
    Field::section("flag", flag.name.clone(), body)
}

fn lower_source_repo(repo: &SourceRepo) -> Field {
    let mut body = Vec::new();
    text(&mut body, "type", repo.repo_type.as_deref());
    text(&mut body, "location", repo.location.as_deref());
    text(&mut body, "module", repo.module.as_deref());
    text(&mut body, "branch", repo.branch.as_deref());
    text(&mut body, "tag", repo.tag.as_deref());
    text(&mut body, "subdir", repo.subdir.as_deref());
    Field::section("source-repository", repo.kind.clone(), body)
}

fn single(out: &mut Vec<Field>, name: &str, value: String) {
    out.push(Field::leaf(name, vec![value]));
}

fn text(out: &mut Vec<Field>, name: &str, value: Option<&str>) {
    if let Some(s) = value {
        out.push(Field::leaf(name, s.lines().map(String::from).collect()));
    }
}

fn list(out: &mut Vec<Field>, name: &str, items: &[String]) {
    if !items.is_empty() {
        out.push(Field::leaf(name, items.to_vec()));
    }
}

fn display_list<T: std::fmt::Display>(out: &mut Vec<Field>, name: &str, items: &[T]) {
    if !items.is_empty() {
        out.push(Field::leaf(
            name,
            items.iter().map(ToString::to_string).collect(),
        ));
    }
}

fn words(out: &mut Vec<Field>, name: &str, items: &[String]) {
    if !items.is_empty() {
        out.push(Field::leaf(name, vec![items.join(" ")]));
    }
}

fn comma_lines(items: &[String]) -> Vec<String> {
    let last = items.len().saturating_sub(1);
    items
        .iter()
        .enumerate()
        .map(|(i, s)| if i < last { format!("{s},") } else { s.clone() })
        .collect()
}

fn dep_lines(deps: &[Dependency]) -> Vec<String> {
    let last = deps.len().saturating_sub(1);
    deps.iter()
        .enumerate()
        .map(|(i, d)| {
            if i < last {
                format!("{d},")
            } else {
                d.to_string()
            }
        })
        .collect()
}

fn bool_str(b: bool) -> String {
    if b { "True" } else { "False" }.to_string()
}
