mod condition;
mod description;
mod error;
mod eval;
mod interpret;
mod lower;
mod syntax;
mod version;

use std::fmt::{self, Display};

pub use condition::Condition;
pub use description::{
    Benchmark, BenchmarkType, BuildInfo, BuildType, CondBranch, CondTree, Dependency, Executable,
    ExecutableScope, Flag, Language, Library, Merge, ModuleName, PackageDescription, SourceRepo,
    TestSuite, TestType, Visibility,
};
pub use error::{Error, ErrorKind};
pub use eval::{Compiler, Environment, ResolvedPackage};
pub use plum_version::Version;
pub use syntax::{Field, FieldLine, Name};
pub use version::VersionRange;

pub fn parse(input: &str) -> Result<PackageDescription, Box<Error>> {
    parse_named(input, "package.cabal")
}

pub fn parse_named(input: &str, name: &str) -> Result<PackageDescription, Box<Error>> {
    let fields = syntax::parse(input);
    interpret::interpret(&fields).map_err(|located| Box::new(Error::new(located, name, input)))
}

pub fn parse_fields(input: &str) -> Vec<Field> {
    syntax::parse(input)
}

pub fn render(pkg: &PackageDescription) -> String {
    syntax::print(&lower::lower(pkg))
}

impl Display for PackageDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&render(self))
    }
}
