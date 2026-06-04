use std::str::FromStr;

use fxhash::FxHashMap;
use plum_version::Version;

use crate::condition::Condition;
use crate::description::{
    Benchmark, BenchmarkType, BuildInfo, BuildType, CondBranch, CondTree, Dependency, Executable,
    ExecutableScope, Flag, Language, Library, PackageDescription, SourceRepo, TestSuite, TestType,
    Visibility,
};
use std::ops::Range;

use crate::error::{ErrorKind, Located, Result};
use crate::syntax::{Field, FieldLine};
use crate::version::VersionRange;

pub fn interpret(fields: &[Field]) -> Result<PackageDescription> {
    let commons = collect_commons(fields);

    let mut pkg = PackageDescription::default();
    for field in fields {
        match field {
            Field::Field { name, value } => {
                set_package_field(&mut pkg, &name.text, value, &name.span)?;
            }
            Field::Section {
                name,
                arg,
                fields: body,
                ..
            } => match name.text.as_str() {
                "library" => {
                    let tree = component_tree::<Library>(body, &commons)?;
                    if arg.is_empty() {
                        pkg.library = Some(tree);
                    } else {
                        pkg.sub_libraries.push((arg.clone(), tree));
                    }
                }
                "executable" => pkg
                    .executables
                    .push((arg.clone(), component_tree::<Executable>(body, &commons)?)),
                "test-suite" => pkg
                    .test_suites
                    .push((arg.clone(), component_tree::<TestSuite>(body, &commons)?)),
                "benchmark" => pkg
                    .benchmarks
                    .push((arg.clone(), component_tree::<Benchmark>(body, &commons)?)),
                "flag" => pkg.flags.push(parse_flag(arg, body)?),
                "source-repository" => pkg.source_repos.push(parse_source_repo(arg, body)),
                _ => {}
            },
        }
    }

    if pkg.library.is_none() {
        let top_fields: Vec<Field> = fields
            .iter()
            .filter(|f| matches!(f, Field::Field { .. }))
            .cloned()
            .collect();
        if let Ok(tree) = build_tree::<Library>(&top_fields)
            && library_has_content(&tree.data)
        {
            pkg.library = Some(tree);
        }
    }

    if pkg.name.is_empty() {
        return Err(Located::new(ErrorKind::MissingField("name"), 0..0));
    }
    Ok(pkg)
}

type Commons<'a> = FxHashMap<String, &'a [Field]>;

fn collect_commons(fields: &[Field]) -> Commons<'_> {
    let mut commons = Commons::default();
    for field in fields {
        if let Field::Section {
            name,
            arg,
            fields: body,
            ..
        } = field
            && name.text == "common"
        {
            commons.insert(arg.to_ascii_lowercase(), body.as_slice());
        }
    }
    commons
}

fn component_tree<C: Component>(body: &[Field], commons: &Commons<'_>) -> Result<CondTree<C>> {
    let mut seen = Vec::new();
    let expanded = expand_imports(body, commons, &mut seen)?;
    build_tree::<C>(&expanded)
}

fn expand_imports(
    fields: &[Field],
    commons: &Commons<'_>,
    seen: &mut Vec<String>,
) -> Result<Vec<Field>> {
    let mut out = Vec::new();
    for field in fields {
        match field {
            Field::Field { name, value } if name.text == "import" => {
                for import in comma_list(value) {
                    let key = import.to_ascii_lowercase();
                    if seen.contains(&key) {
                        continue;
                    }
                    let body = commons.get(&key).ok_or_else(|| {
                        Located::new(ErrorKind::UnknownImport(import), name.span.clone())
                    })?;
                    seen.push(key);
                    out.extend(expand_imports(body, commons, seen)?);
                    seen.pop();
                }
            }
            other => out.push(other.clone()),
        }
    }
    Ok(out)
}

fn build_tree<C: Component>(fields: &[Field]) -> Result<CondTree<C>> {
    let mut data = C::default();
    let mut branches = Vec::new();
    let mut i = 0;

    while i < fields.len() {
        match &fields[i] {
            Field::Field { name, value } => {
                if name.text != "import" {
                    set_component_field(&mut data, &name.text, value, &name.span)?;
                }
                i += 1;
            }
            Field::Section {
                name,
                arg,
                arg_span,
                fields: body,
            } => {
                if name.text == "if" {
                    let condition = parse_condition(arg, arg_span)?;
                    let then_tree = build_tree::<C>(body)?;
                    let (else_tree, consumed) = build_else::<C>(&fields[i + 1..])?;
                    branches.push(CondBranch {
                        condition,
                        then_tree,
                        else_tree,
                    });
                    i += 1 + consumed;
                } else {
                    i += 1;
                }
            }
        }
    }

    Ok(CondTree { data, branches })
}

fn build_else<C: Component>(rest: &[Field]) -> Result<(Option<CondTree<C>>, usize)> {
    match rest.first() {
        Some(Field::Section {
            name,
            arg,
            arg_span,
            fields: body,
        }) if name.text == "elif" => {
            let condition = parse_condition(arg, arg_span)?;
            let then_tree = build_tree::<C>(body)?;
            let (nested, consumed) = build_else::<C>(&rest[1..])?;
            let tree = CondTree {
                data: C::default(),
                branches: vec![CondBranch {
                    condition,
                    then_tree,
                    else_tree: nested,
                }],
            };
            Ok((Some(tree), 1 + consumed))
        }
        Some(Field::Section {
            name, fields: body, ..
        }) if name.text == "else" => Ok((Some(build_tree::<C>(body)?), 1)),
        _ => Ok((None, 0)),
    }
}

fn parse_condition(arg: &str, span: &Range<usize>) -> Result<Condition> {
    Condition::parse(arg).map_err(|e| Located::new(ErrorKind::InvalidCondition(e), span.clone()))
}

fn library_has_content(lib: &Library) -> bool {
    !lib.exposed_modules.is_empty()
        || !lib.build_info.build_depends.is_empty()
        || !lib.build_info.other_modules.is_empty()
        || !lib.build_info.hs_source_dirs.is_empty()
}

trait Component: Default {
    fn build_info_mut(&mut self) -> &mut BuildInfo;
    fn set_specific(
        &mut self,
        name: &str,
        value: &[FieldLine],
        span: &Range<usize>,
    ) -> Result<bool>;
}

fn set_component_field<C: Component>(
    component: &mut C,
    name: &str,
    value: &[FieldLine],
    span: &Range<usize>,
) -> Result<()> {
    if component.set_specific(name, value, span)? {
        return Ok(());
    }
    set_build_info_field(component.build_info_mut(), name, value, span)
}

impl Component for Library {
    fn build_info_mut(&mut self) -> &mut BuildInfo {
        &mut self.build_info
    }

    fn set_specific(
        &mut self,
        name: &str,
        value: &[FieldLine],
        span: &Range<usize>,
    ) -> Result<bool> {
        match name {
            "exposed-modules" => self.exposed_modules.extend(token_list(value)),
            "reexported-modules" => self.reexported_modules.extend(comma_list(value)),
            "visibility" => {
                self.visibility = Some(
                    Visibility::from_cabal(&single(value)).map_err(|e| invalid(name, span, e))?,
                );
            }
            "signatures" => {}
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl Component for Executable {
    fn build_info_mut(&mut self) -> &mut BuildInfo {
        &mut self.build_info
    }

    fn set_specific(
        &mut self,
        name: &str,
        value: &[FieldLine],
        span: &Range<usize>,
    ) -> Result<bool> {
        match name {
            "main-is" => self.main_is = Some(single(value)),
            "scope" => {
                self.scope = Some(
                    ExecutableScope::from_cabal(&single(value))
                        .map_err(|e| invalid(name, span, e))?,
                );
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl Component for TestSuite {
    fn build_info_mut(&mut self) -> &mut BuildInfo {
        &mut self.build_info
    }

    fn set_specific(
        &mut self,
        name: &str,
        value: &[FieldLine],
        _span: &Range<usize>,
    ) -> Result<bool> {
        match name {
            "type" => self.kind = Some(TestType::from_cabal(&single(value))),
            "main-is" => self.main_is = Some(single(value)),
            "test-module" => self.test_module = Some(single(value)),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl Component for Benchmark {
    fn build_info_mut(&mut self) -> &mut BuildInfo {
        &mut self.build_info
    }

    fn set_specific(
        &mut self,
        name: &str,
        value: &[FieldLine],
        _span: &Range<usize>,
    ) -> Result<bool> {
        match name {
            "type" => self.kind = Some(BenchmarkType::from_cabal(&single(value))),
            "main-is" => self.main_is = Some(single(value)),
            _ => return Ok(false),
        }
        Ok(true)
    }
}

fn set_build_info_field(
    bi: &mut BuildInfo,
    name: &str,
    value: &[FieldLine],
    span: &Range<usize>,
) -> Result<()> {
    match name {
        "buildable" => {
            bi.buildable = Some(parse_bool(&single(value)).map_err(|e| invalid(name, span, e))?);
        }
        "build-depends" | "build-deps" => {
            for entry in comma_list(value) {
                bi.build_depends
                    .push(parse_dependency(&entry).map_err(|e| invalid("build-depends", span, e))?);
            }
        }
        "hs-source-dirs" | "hs-source-dir" => bi.hs_source_dirs.extend(token_list(value)),
        "default-language" => bi.default_language = Some(Language::from_cabal(&single(value))),
        "other-languages" => bi
            .other_languages
            .extend(token_list(value).iter().map(|s| Language::from_cabal(s))),
        "default-extensions" | "extensions" => bi.default_extensions.extend(token_list(value)),
        "other-extensions" => bi.other_extensions.extend(token_list(value)),
        "ghc-options" => bi.ghc_options.extend(word_list(value)),
        "ghc-prof-options" => bi.ghc_prof_options.extend(word_list(value)),
        "cpp-options" => bi.cpp_options.extend(word_list(value)),
        "cc-options" => bi.cc_options.extend(word_list(value)),
        "cxx-options" => bi.cxx_options.extend(word_list(value)),
        "ld-options" => bi.ld_options.extend(word_list(value)),
        "other-modules" => bi.other_modules.extend(token_list(value)),
        "c-sources" => bi.c_sources.extend(token_list(value)),
        "cxx-sources" => bi.cxx_sources.extend(token_list(value)),
        "js-sources" => bi.js_sources.extend(token_list(value)),
        "extra-libraries" => bi.extra_libraries.extend(token_list(value)),
        "extra-lib-dirs" => bi.extra_lib_dirs.extend(token_list(value)),
        "include-dirs" => bi.include_dirs.extend(token_list(value)),
        "includes" => bi.includes.extend(token_list(value)),
        "install-includes" => bi.install_includes.extend(token_list(value)),
        "pkgconfig-depends" => bi.pkgconfig_depends.extend(comma_list(value)),
        "build-tool-depends" | "build-tools" => bi.build_tool_depends.extend(comma_list(value)),
        "frameworks" => bi.frameworks.extend(token_list(value)),
        _ => {}
    }
    Ok(())
}

fn set_package_field(
    pkg: &mut PackageDescription,
    name: &str,
    value: &[FieldLine],
    span: &Range<usize>,
) -> Result<()> {
    match name {
        "cabal-version" => pkg.cabal_version = parse_version("cabal-version", value, span).ok(),
        "name" => pkg.name = single(value),
        "version" => pkg.version = Some(parse_version("version", value, span)?),
        "synopsis" => pkg.synopsis = Some(free_text(value)),
        "description" => pkg.description = Some(free_text(value)),
        "homepage" => pkg.homepage = Some(single(value)),
        "bug-reports" => pkg.bug_reports = Some(single(value)),
        "package-url" => pkg.package_url = Some(single(value)),
        "license" => pkg.license = Some(single(value)),
        "license-file" | "license-files" => pkg.license_files.extend(token_list(value)),
        "copyright" => pkg.copyright = Some(single(value)),
        "author" => pkg.author = Some(single(value)),
        "maintainer" => pkg.maintainer = Some(single(value)),
        "stability" => pkg.stability = Some(single(value)),
        "category" => pkg.category = Some(single(value)),
        "build-type" => pkg.build_type = Some(BuildType::from_cabal(&single(value))),
        "tested-with" => pkg.tested_with.extend(comma_list(value)),
        "data-files" => pkg.data_files.extend(token_list(value)),
        "data-dir" => pkg.data_dir = Some(single(value)),
        "extra-source-files" => pkg.extra_source_files.extend(token_list(value)),
        "extra-doc-files" => pkg.extra_doc_files.extend(token_list(value)),
        _ => {}
    }
    Ok(())
}

fn parse_flag(arg: &str, body: &[Field]) -> Result<Flag> {
    let mut flag = Flag {
        name: arg.to_string(),
        ..Flag::default()
    };
    for field in body {
        if let Field::Field { name, value } = field {
            match name.text.as_str() {
                "description" => flag.description = Some(free_text(value)),
                "default" => {
                    flag.default = parse_bool(&single(value))
                        .map_err(|e| invalid("default", &name.span, e))?;
                }
                "manual" => {
                    flag.manual =
                        parse_bool(&single(value)).map_err(|e| invalid("manual", &name.span, e))?;
                }
                _ => {}
            }
        }
    }
    Ok(flag)
}

fn parse_source_repo(arg: &str, body: &[Field]) -> SourceRepo {
    let mut repo = SourceRepo {
        kind: arg.to_string(),
        ..SourceRepo::default()
    };
    for field in body {
        if let Field::Field { name, value } = field {
            let v = single(value);
            match name.text.as_str() {
                "type" => repo.repo_type = Some(v),
                "location" => repo.location = Some(v),
                "module" => repo.module = Some(v),
                "branch" => repo.branch = Some(v),
                "tag" => repo.tag = Some(v),
                "subdir" => repo.subdir = Some(v),
                _ => {}
            }
        }
    }
    repo
}

fn invalid(field: &str, span: &Range<usize>, detail: String) -> Located {
    Located::new(
        ErrorKind::InvalidValue {
            field: field.to_string(),
            detail,
        },
        span.clone(),
    )
}

fn single(value: &[FieldLine]) -> String {
    value
        .iter()
        .map(|l| l.text.trim())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn free_text(value: &[FieldLine]) -> String {
    let mut out = String::new();
    let mut prev: Option<usize> = None;
    for line in value {
        if let Some(p) = prev {
            for _ in 0..line.line.saturating_sub(p).max(1) {
                out.push('\n');
            }
        }
        let text = line.text.trim();
        if text != "." {
            out.push_str(text);
        }
        prev = Some(line.line);
    }
    out.trim().to_string()
}

fn comma_list(value: &[FieldLine]) -> Vec<String> {
    let joined = value
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let mut entries = Vec::new();
    let mut depth = 0u32;
    let mut start = 0;
    for (i, b) in joined.bytes().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                push_entry(&mut entries, &joined[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    push_entry(&mut entries, &joined[start..]);
    entries
}

fn push_entry(entries: &mut Vec<String>, raw: &str) {
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
        entries.push(trimmed.to_string());
    }
}

fn token_list(value: &[FieldLine]) -> Vec<String> {
    value
        .iter()
        .flat_map(|l| l.text.split(|c: char| c.is_whitespace() || c == ','))
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn word_list(value: &[FieldLine]) -> Vec<String> {
    value
        .iter()
        .flat_map(|l| l.text.split_whitespace())
        .map(String::from)
        .collect()
}

fn parse_bool(s: &str) -> std::result::Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        other => Err(format!("expected a boolean, found `{other}`")),
    }
}

fn parse_version(field: &str, value: &[FieldLine], span: &Range<usize>) -> Result<Version> {
    let raw = single(value);
    let start = raw
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| invalid(field, span, format!("expected a version, found `{raw}`")))?;
    let rest = &raw[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(rest.len());
    let digits = rest[..end].trim_end_matches('.');
    Version::from_str(digits).map_err(|e| invalid(field, span, e.to_string()))
}

fn parse_dependency(entry: &str) -> std::result::Result<Dependency, String> {
    let entry = entry.trim();
    let name_len = entry
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
        .count();
    if name_len == 0 {
        return Err(format!("expected a package name in `{entry}`"));
    }
    let name = entry[..name_len].to_string();
    let mut rest = entry[name_len..].trim_start();

    let mut sublibs = Vec::new();
    if let Some(after) = rest.strip_prefix(':') {
        let after = after.trim_start();
        if let Some(inner) = after.strip_prefix('{') {
            let end = inner
                .find('}')
                .ok_or_else(|| format!("unterminated `{{` in `{entry}`"))?;
            sublibs = inner[..end]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            rest = inner[end + 1..].trim_start();
        } else {
            let sub_len = after
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
                .count();
            sublibs.push(after[..sub_len].to_string());
            rest = after[sub_len..].trim_start();
        }
    }

    let range = if rest.is_empty() {
        VersionRange::Any
    } else {
        VersionRange::parse(rest)?
    };
    Ok(Dependency {
        name,
        sublibs,
        range,
    })
}
