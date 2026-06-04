use std::ops::Range;

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ErrorKind {
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("invalid value for `{field}`: {detail}")]
    InvalidValue { field: String, detail: String },
    #[error("invalid condition: {0}")]
    InvalidCondition(String),
    #[error("`import` refers to unknown `{0}`")]
    UnknownImport(String),
}

#[derive(Debug, Clone)]
pub struct Located {
    pub kind: ErrorKind,
    pub span: Range<usize>,
}

impl Located {
    pub fn new(kind: ErrorKind, span: Range<usize>) -> Self {
        Self { kind, span }
    }
}

pub type Result<T> = std::result::Result<T, Located>;

#[derive(Debug, Error, Diagnostic)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
    #[source_code]
    src: NamedSource<String>,
    #[label("here")]
    at: SourceSpan,
}

impl Error {
    pub fn new(located: Located, name: &str, source: &str) -> Self {
        Self {
            at: (located.span.start, located.span.len()).into(),
            src: NamedSource::new(name, source.to_string()),
            kind: located.kind,
        }
    }
}
