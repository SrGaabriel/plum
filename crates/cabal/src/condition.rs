use std::fmt::{self, Display};

use winnow::ascii::space0;
use winnow::combinator::{alt, delimited, fail, opt, preceded, repeat};
use winnow::token::take_while;
use winnow::{ModalResult, Parser};

use crate::version::{VersionRange, range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    Bool(bool),
    Os(String),
    Arch(String),
    Flag(String),
    Impl {
        compiler: String,
        range: Option<VersionRange>,
    },
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

impl Condition {
    pub fn parse(input: &str) -> Result<Self, String> {
        or_cond.parse(input.trim()).map_err(|e| e.to_string())
    }
}

fn or_cond(input: &mut &str) -> ModalResult<Condition> {
    let first = and_cond(input)?;
    let rest: Vec<Condition> =
        repeat(0.., preceded((space0, "||", space0), and_cond)).parse_next(input)?;
    Ok(rest
        .into_iter()
        .fold(first, |a, b| Condition::Or(Box::new(a), Box::new(b))))
}

fn and_cond(input: &mut &str) -> ModalResult<Condition> {
    let first = not_cond(input)?;
    let rest: Vec<Condition> =
        repeat(0.., preceded((space0, "&&", space0), not_cond)).parse_next(input)?;
    Ok(rest
        .into_iter()
        .fold(first, |a, b| Condition::And(Box::new(a), Box::new(b))))
}

fn not_cond(input: &mut &str) -> ModalResult<Condition> {
    space0.parse_next(input)?;
    let negated = opt(preceded("!", space0)).parse_next(input)?;
    let inner = atom(input)?;
    Ok(match negated {
        Some(_) => Condition::Not(Box::new(inner)),
        None => inner,
    })
}

fn atom(input: &mut &str) -> ModalResult<Condition> {
    space0.parse_next(input)?;
    let cond = alt((
        delimited(("(", space0), or_cond, (space0, ")")),
        predicate,
        "true".value(Condition::Bool(true)),
        "false".value(Condition::Bool(false)),
    ))
    .parse_next(input)?;
    space0.parse_next(input)?;
    Ok(cond)
}

fn predicate(input: &mut &str) -> ModalResult<Condition> {
    let name = ident(input)?;
    space0.parse_next(input)?;
    "(".parse_next(input)?;
    space0.parse_next(input)?;
    let cond = match name {
        "os" => Condition::Os(ident(input)?.to_ascii_lowercase()),
        "arch" => Condition::Arch(ident(input)?.to_ascii_lowercase()),
        "flag" => Condition::Flag(ident(input)?.to_ascii_lowercase()),
        "impl" => {
            let compiler = ident(input)?.to_ascii_lowercase();
            space0.parse_next(input)?;
            let range = opt(range).parse_next(input)?;
            Condition::Impl { compiler, range }
        }
        _ => return fail.parse_next(input),
    };
    space0.parse_next(input)?;
    ")".parse_next(input)?;
    Ok(cond)
}

fn ident<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    take_while(1.., |c: char| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_'
    })
    .parse_next(input)
}

impl Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Condition::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Condition::Os(n) => write!(f, "os({n})"),
            Condition::Arch(n) => write!(f, "arch({n})"),
            Condition::Flag(n) => write!(f, "flag({n})"),
            Condition::Impl {
                compiler,
                range: Some(r),
            } => write!(f, "impl({compiler} {r})"),
            Condition::Impl {
                compiler,
                range: None,
            } => write!(f, "impl({compiler})"),
            Condition::Not(c) => {
                f.write_str("!")?;
                not_operand(f, c)
            }
            Condition::And(a, b) => {
                and_operand(f, a)?;
                f.write_str(" && ")?;
                and_operand(f, b)
            }
            Condition::Or(a, b) => write!(f, "{a} || {b}"),
        }
    }
}

fn and_operand(f: &mut fmt::Formatter<'_>, c: &Condition) -> fmt::Result {
    if matches!(c, Condition::Or(..)) {
        write!(f, "({c})")
    } else {
        write!(f, "{c}")
    }
}

fn not_operand(f: &mut fmt::Formatter<'_>, c: &Condition) -> fmt::Result {
    if matches!(c, Condition::And(..) | Condition::Or(..)) {
        write!(f, "({c})")
    } else {
        write!(f, "{c}")
    }
}
