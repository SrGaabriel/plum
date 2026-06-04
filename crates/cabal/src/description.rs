use std::fmt::{self, Display};

use plum_version::Version;

use crate::condition::Condition;
use crate::version::VersionRange;

pub type ModuleName = String;

#[derive(Debug, Clone, Default)]
pub struct PackageDescription {
    pub cabal_version: Option<Version>,
    pub name: String,
    pub version: Option<Version>,
    pub synopsis: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub bug_reports: Option<String>,
    pub package_url: Option<String>,
    pub license: Option<String>,
    pub license_files: Vec<String>,
    pub copyright: Option<String>,
    pub author: Option<String>,
    pub maintainer: Option<String>,
    pub stability: Option<String>,
    pub category: Option<String>,
    pub build_type: Option<BuildType>,
    pub tested_with: Vec<String>,
    pub data_files: Vec<String>,
    pub data_dir: Option<String>,
    pub extra_source_files: Vec<String>,
    pub extra_doc_files: Vec<String>,
    pub flags: Vec<Flag>,
    pub source_repos: Vec<SourceRepo>,
    pub library: Option<CondTree<Library>>,
    pub sub_libraries: Vec<(String, CondTree<Library>)>,
    pub executables: Vec<(String, CondTree<Executable>)>,
    pub test_suites: Vec<(String, CondTree<TestSuite>)>,
    pub benchmarks: Vec<(String, CondTree<Benchmark>)>,
}

#[derive(Debug, Clone, Default)]
pub struct BuildInfo {
    pub buildable: Option<bool>,
    pub build_depends: Vec<Dependency>,
    pub hs_source_dirs: Vec<String>,
    pub default_language: Option<Language>,
    pub other_languages: Vec<Language>,
    pub default_extensions: Vec<String>,
    pub other_extensions: Vec<String>,
    pub ghc_options: Vec<String>,
    pub ghc_prof_options: Vec<String>,
    pub cpp_options: Vec<String>,
    pub cc_options: Vec<String>,
    pub cxx_options: Vec<String>,
    pub ld_options: Vec<String>,
    pub other_modules: Vec<ModuleName>,
    pub c_sources: Vec<String>,
    pub cxx_sources: Vec<String>,
    pub js_sources: Vec<String>,
    pub extra_libraries: Vec<String>,
    pub extra_lib_dirs: Vec<String>,
    pub include_dirs: Vec<String>,
    pub includes: Vec<String>,
    pub install_includes: Vec<String>,
    pub pkgconfig_depends: Vec<String>,
    pub build_tool_depends: Vec<String>,
    pub frameworks: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub exposed_modules: Vec<ModuleName>,
    pub reexported_modules: Vec<String>,
    pub visibility: Option<Visibility>,
    pub build_info: BuildInfo,
}

#[derive(Debug, Clone, Default)]
pub struct Executable {
    pub main_is: Option<String>,
    pub scope: Option<ExecutableScope>,
    pub build_info: BuildInfo,
}

#[derive(Debug, Clone, Default)]
pub struct TestSuite {
    pub kind: Option<TestType>,
    pub main_is: Option<String>,
    pub test_module: Option<ModuleName>,
    pub build_info: BuildInfo,
}

#[derive(Debug, Clone, Default)]
pub struct Benchmark {
    pub kind: Option<BenchmarkType>,
    pub main_is: Option<String>,
    pub build_info: BuildInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub sublibs: Vec<String>,
    pub range: VersionRange,
}

#[derive(Debug, Clone)]
pub struct Flag {
    pub name: String,
    pub description: Option<String>,
    pub default: bool,
    pub manual: bool,
}

impl Default for Flag {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            default: true,
            manual: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceRepo {
    pub kind: String,
    pub repo_type: Option<String>,
    pub location: Option<String>,
    pub module: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub subdir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildType {
    Simple,
    Configure,
    Make,
    Custom,
    Hooks,
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableScope {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub struct CondTree<T> {
    pub data: T,
    pub branches: Vec<CondBranch<T>>,
}

#[derive(Debug, Clone)]
pub struct CondBranch<T> {
    pub condition: Condition,
    pub then_tree: CondTree<T>,
    pub else_tree: Option<CondTree<T>>,
}

impl<T: Default> Default for CondTree<T> {
    fn default() -> Self {
        Self {
            data: T::default(),
            branches: Vec::new(),
        }
    }
}

pub trait Merge {
    fn merge(&mut self, other: Self);
}

macro_rules! merge_fields {
    ($self:ident, $other:ident, vecs: [$($v:ident),* $(,)?], opts: [$($o:ident),* $(,)?]) => {
        $( $self.$v.extend($other.$v); )*
        $( if $other.$o.is_some() { $self.$o = $other.$o; } )*
    };
}

impl Merge for BuildInfo {
    fn merge(&mut self, other: Self) {
        self.buildable = match (self.buildable, other.buildable) {
            (Some(a), Some(b)) => Some(a && b),
            (a, b) => b.or(a),
        };
        merge_fields!(self, other,
            vecs: [
                build_depends, hs_source_dirs, other_languages, default_extensions,
                other_extensions, ghc_options, ghc_prof_options, cpp_options, cc_options,
                cxx_options, ld_options, other_modules, c_sources, cxx_sources, js_sources,
                extra_libraries, extra_lib_dirs, include_dirs, includes, install_includes,
                pkgconfig_depends, build_tool_depends, frameworks,
            ],
            opts: [default_language]
        );
    }
}

impl Merge for Library {
    fn merge(&mut self, other: Self) {
        self.exposed_modules.extend(other.exposed_modules);
        self.reexported_modules.extend(other.reexported_modules);
        if other.visibility.is_some() {
            self.visibility = other.visibility;
        }
        self.build_info.merge(other.build_info);
    }
}

impl Merge for Executable {
    fn merge(&mut self, other: Self) {
        if other.main_is.is_some() {
            self.main_is = other.main_is;
        }
        if other.scope.is_some() {
            self.scope = other.scope;
        }
        self.build_info.merge(other.build_info);
    }
}

impl Merge for TestSuite {
    fn merge(&mut self, other: Self) {
        if other.kind.is_some() {
            self.kind = other.kind;
        }
        if other.main_is.is_some() {
            self.main_is = other.main_is;
        }
        if other.test_module.is_some() {
            self.test_module = other.test_module;
        }
        self.build_info.merge(other.build_info);
    }
}

impl Merge for Benchmark {
    fn merge(&mut self, other: Self) {
        if other.kind.is_some() {
            self.kind = other.kind;
        }
        if other.main_is.is_some() {
            self.main_is = other.main_is;
        }
        self.build_info.merge(other.build_info);
    }
}

impl BuildType {
    pub fn from_cabal(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "simple" => BuildType::Simple,
            "configure" => BuildType::Configure,
            "make" => BuildType::Make,
            "custom" => BuildType::Custom,
            "hooks" => BuildType::Hooks,
            _ => BuildType::Other(s.to_string()),
        }
    }
}

impl Display for BuildType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BuildType::Simple => "Simple",
            BuildType::Configure => "Configure",
            BuildType::Make => "Make",
            BuildType::Custom => "Custom",
            BuildType::Hooks => "Hooks",
            BuildType::Other(s) => s,
        })
    }
}

impl Visibility {
    pub fn from_cabal(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Ok(Visibility::Public),
            "private" => Ok(Visibility::Private),
            other => Err(format!("expected `public` or `private`, found `{other}`")),
        }
    }
}

impl Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        })
    }
}

impl ExecutableScope {
    pub fn from_cabal(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "public" => Ok(ExecutableScope::Public),
            "private" => Ok(ExecutableScope::Private),
            other => Err(format!("expected `public` or `private`, found `{other}`")),
        }
    }
}

impl Display for ExecutableScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ExecutableScope::Public => "public",
            ExecutableScope::Private => "private",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Haskell98,
    Haskell2010,
    Ghc2021,
    Ghc2024,
    Other(String),
}

impl Language {
    pub fn from_cabal(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "haskell98" => Language::Haskell98,
            "haskell2010" => Language::Haskell2010,
            "ghc2021" => Language::Ghc2021,
            "ghc2024" => Language::Ghc2024,
            _ => Language::Other(s.to_string()),
        }
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Language::Haskell98 => "Haskell98",
            Language::Haskell2010 => "Haskell2010",
            Language::Ghc2021 => "GHC2021",
            Language::Ghc2024 => "GHC2024",
            Language::Other(s) => s,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestType {
    ExitcodeStdio,
    Detailed,
    Other(String),
}

impl TestType {
    pub fn from_cabal(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "exitcode-stdio-1.0" => TestType::ExitcodeStdio,
            "detailed-0.9" => TestType::Detailed,
            _ => TestType::Other(s.to_string()),
        }
    }
}

impl Display for TestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TestType::ExitcodeStdio => "exitcode-stdio-1.0",
            TestType::Detailed => "detailed-0.9",
            TestType::Other(s) => s,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkType {
    ExitcodeStdio,
    Other(String),
}

impl BenchmarkType {
    pub fn from_cabal(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "exitcode-stdio-1.0" => BenchmarkType::ExitcodeStdio,
            _ => BenchmarkType::Other(s.to_string()),
        }
    }
}

impl Display for BenchmarkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BenchmarkType::ExitcodeStdio => "exitcode-stdio-1.0",
            BenchmarkType::Other(s) => s,
        })
    }
}

impl Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        match self.sublibs.as_slice() {
            [] => {}
            [one] => write!(f, ":{one}")?,
            many => write!(f, ":{{{}}}", many.join(","))?,
        }
        if !matches!(self.range, VersionRange::Any) {
            write!(f, " {}", self.range)?;
        }
        Ok(())
    }
}
