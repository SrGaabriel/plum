mod parser;
mod printer;

pub use parser::parse;
pub use printer::print;

use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Name {
    pub text: String,
    pub span: Range<usize>,
}

impl Name {
    pub fn synthetic(text: &str) -> Self {
        Self {
            text: text.to_string(),
            span: 0..0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldLine {
    pub text: String,
    pub line: usize,
}

impl FieldLine {
    pub fn synthetic(text: String) -> Self {
        Self { text, line: 0 }
    }
}

#[derive(Debug, Clone)]
pub enum Field {
    Field {
        name: Name,
        value: Vec<FieldLine>,
    },
    Section {
        name: Name,
        arg: String,
        arg_span: Range<usize>,
        fields: Vec<Field>,
    },
}

impl Field {
    pub fn name(&self) -> &str {
        match self {
            Field::Field { name, .. } | Field::Section { name, .. } => &name.text,
        }
    }

    pub fn leaf(name: &str, value: Vec<String>) -> Self {
        Field::Field {
            name: Name::synthetic(name),
            value: value.into_iter().map(FieldLine::synthetic).collect(),
        }
    }

    pub fn section(name: &str, arg: String, fields: Vec<Field>) -> Self {
        Field::Section {
            name: Name::synthetic(name),
            arg,
            arg_span: 0..0,
            fields,
        }
    }
}
