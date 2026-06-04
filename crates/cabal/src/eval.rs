use fxhash::FxHashMap;
use plum_version::Version;

use crate::condition::Condition;
use crate::description::{
    Benchmark, CondTree, Executable, Library, Merge, PackageDescription, TestSuite,
};

#[derive(Debug, Clone)]
pub struct Compiler {
    pub flavor: String,
    pub version: Version,
}

impl Compiler {
    pub fn new(flavor: impl Into<String>, version: Version) -> Self {
        Self {
            flavor: flavor.into(),
            version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub os: String,
    pub arch: String,
    pub compiler: Compiler,
    pub flag_overrides: FxHashMap<String, bool>,
}

impl Environment {
    pub fn new(os: impl Into<String>, arch: impl Into<String>, compiler: Compiler) -> Self {
        Self {
            os: os.into(),
            arch: arch.into(),
            compiler,
            flag_overrides: FxHashMap::default(),
        }
    }

    pub fn with_flag(mut self, name: impl Into<String>, value: bool) -> Self {
        self.flag_overrides
            .insert(name.into().to_ascii_lowercase(), value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Option<Version>,
    pub flags: FxHashMap<String, bool>,
    pub library: Option<Library>,
    pub sub_libraries: Vec<(String, Library)>,
    pub executables: Vec<(String, Executable)>,
    pub test_suites: Vec<(String, TestSuite)>,
    pub benchmarks: Vec<(String, Benchmark)>,
}

impl PackageDescription {
    pub fn evaluate(&self, env: &Environment) -> ResolvedPackage {
        let flags = self.resolve_flags(env);

        ResolvedPackage {
            name: self.name.clone(),
            version: self.version.clone(),
            library: self.library.as_ref().map(|t| t.resolve(env, &flags)),
            sub_libraries: self
                .sub_libraries
                .iter()
                .map(|(n, t)| (n.clone(), t.resolve(env, &flags)))
                .collect(),
            executables: self
                .executables
                .iter()
                .map(|(n, t)| (n.clone(), t.resolve(env, &flags)))
                .collect(),
            test_suites: self
                .test_suites
                .iter()
                .map(|(n, t)| (n.clone(), t.resolve(env, &flags)))
                .collect(),
            benchmarks: self
                .benchmarks
                .iter()
                .map(|(n, t)| (n.clone(), t.resolve(env, &flags)))
                .collect(),
            flags,
        }
    }

    fn resolve_flags(&self, env: &Environment) -> FxHashMap<String, bool> {
        let mut flags = FxHashMap::default();
        for flag in &self.flags {
            let key = flag.name.to_ascii_lowercase();
            let value = env
                .flag_overrides
                .get(&key)
                .copied()
                .unwrap_or(flag.default);
            flags.insert(key, value);
        }
        for (key, value) in &env.flag_overrides {
            flags.entry(key.clone()).or_insert(*value);
        }
        flags
    }
}

impl<T: Merge + Clone> CondTree<T> {
    fn resolve(&self, env: &Environment, flags: &FxHashMap<String, bool>) -> T {
        let mut acc = self.data.clone();
        for branch in &self.branches {
            if eval(&branch.condition, env, flags) {
                acc.merge(branch.then_tree.resolve(env, flags));
            } else if let Some(else_tree) = &branch.else_tree {
                acc.merge(else_tree.resolve(env, flags));
            }
        }
        acc
    }
}

fn eval(cond: &Condition, env: &Environment, flags: &FxHashMap<String, bool>) -> bool {
    match cond {
        Condition::Bool(b) => *b,
        Condition::Os(name) => env.os.eq_ignore_ascii_case(name),
        Condition::Arch(name) => env.arch.eq_ignore_ascii_case(name),
        Condition::Flag(name) => flags
            .get(&name.to_ascii_lowercase())
            .copied()
            .unwrap_or(false),
        Condition::Impl { compiler, range } => {
            env.compiler.flavor.eq_ignore_ascii_case(compiler)
                && range
                    .as_ref()
                    .is_none_or(|r| r.contains(&env.compiler.version))
        }
        Condition::Not(c) => !eval(c, env, flags),
        Condition::And(a, b) => eval(a, env, flags) && eval(b, env, flags),
        Condition::Or(a, b) => eval(a, env, flags) || eval(b, env, flags),
    }
}
