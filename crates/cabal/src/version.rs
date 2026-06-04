use std::fmt::{self, Display};

use plum_version::Version;
use winnow::ascii::{digit1, space0};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated};
use winnow::{ModalResult, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionRange {
    Any,
    None,
    This(Version),
    Wildcard(Version),
    Earlier(Version),
    EarlierEqual(Version),
    Later(Version),
    LaterEqual(Version),
    Caret(Version),
    Union(Box<VersionRange>, Box<VersionRange>),
    Intersection(Box<VersionRange>, Box<VersionRange>),
}

impl VersionRange {
    pub fn contains(&self, version: &Version) -> bool {
        match self {
            VersionRange::Any => true,
            VersionRange::None => false,
            VersionRange::This(v) => version == v,
            VersionRange::Wildcard(base) => version >= base && version < &wildcard_upper(base),
            VersionRange::Earlier(v) => version < v,
            VersionRange::EarlierEqual(v) => version <= v,
            VersionRange::Later(v) => version > v,
            VersionRange::LaterEqual(v) => version >= v,
            VersionRange::Caret(v) => version >= v && version < &major_upper_bound(v),
            VersionRange::Union(a, b) => a.contains(version) || b.contains(version),
            VersionRange::Intersection(a, b) => a.contains(version) && b.contains(version),
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        range.parse(input.trim()).map_err(|e| e.to_string())
    }
}

fn wildcard_upper(base: &Version) -> Version {
    let mut comps = base.components.to_vec();
    match comps.last_mut() {
        Some(last) => *last += 1,
        None => comps.push(1),
    }
    Version {
        components: comps.into(),
    }
}

fn major_upper_bound(v: &Version) -> Version {
    let c = &v.components;
    let upper = match c.len() {
        0 => vec![1],
        1 => vec![c[0], 1],
        _ => vec![c[0], c[1] + 1],
    };
    Version {
        components: upper.into(),
    }
}

pub fn range(input: &mut &str) -> ModalResult<VersionRange> {
    let first = and_range(input)?;
    let rest: Vec<VersionRange> =
        repeat(0.., preceded((space0, "||", space0), and_range)).parse_next(input)?;
    Ok(rest.into_iter().fold(first, |acc, r| {
        VersionRange::Union(Box::new(acc), Box::new(r))
    }))
}

fn and_range(input: &mut &str) -> ModalResult<VersionRange> {
    let first = primary(input)?;
    let rest: Vec<VersionRange> =
        repeat(0.., preceded((space0, "&&", space0), primary)).parse_next(input)?;
    Ok(rest.into_iter().fold(first, |acc, r| {
        VersionRange::Intersection(Box::new(acc), Box::new(r))
    }))
}

fn primary(input: &mut &str) -> ModalResult<VersionRange> {
    space0.parse_next(input)?;
    let r = alt((delimited(("(", space0), range, (space0, ")")), constraint)).parse_next(input)?;
    space0.parse_next(input)?;
    Ok(r)
}

fn constraint(input: &mut &str) -> ModalResult<VersionRange> {
    alt((
        "-any".value(VersionRange::Any),
        "-none".value(VersionRange::None),
        preceded(("^>=", space0), version).map(VersionRange::Caret),
        preceded((">=", space0), version).map(VersionRange::LaterEqual),
        preceded(("<=", space0), version).map(VersionRange::EarlierEqual),
        preceded((">", space0), version).map(VersionRange::Later),
        preceded(("<", space0), version).map(VersionRange::Earlier),
        preceded(("==", space0), exact_or_wildcard),
    ))
    .parse_next(input)
}

fn exact_or_wildcard(input: &mut &str) -> ModalResult<VersionRange> {
    let mut comps = vec![digit1.parse_to::<u64>().parse_next(input)?];
    let mut wildcard = false;
    while opt('.').parse_next(input)?.is_some() {
        if opt('*').parse_next(input)?.is_some() {
            wildcard = true;
            break;
        }
        comps.push(digit1.parse_to::<u64>().parse_next(input)?);
    }
    let v = Version {
        components: comps.into(),
    };
    Ok(if wildcard {
        VersionRange::Wildcard(v)
    } else {
        VersionRange::This(v)
    })
}

fn version(input: &mut &str) -> ModalResult<Version> {
    let comps: Vec<u64> = separated(1.., digit1.parse_to::<u64>(), '.').parse_next(input)?;
    Ok(Version {
        components: comps.into(),
    })
}

impl Display for VersionRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionRange::Any => f.write_str("-any"),
            VersionRange::None => f.write_str("-none"),
            VersionRange::This(v) => write!(f, "=={v}"),
            VersionRange::Wildcard(v) => write!(f, "=={v}.*"),
            VersionRange::Earlier(v) => write!(f, "<{v}"),
            VersionRange::EarlierEqual(v) => write!(f, "<={v}"),
            VersionRange::Later(v) => write!(f, ">{v}"),
            VersionRange::LaterEqual(v) => write!(f, ">={v}"),
            VersionRange::Caret(v) => write!(f, "^>={v}"),
            VersionRange::Union(a, b) => write!(f, "{a} || {b}"),
            VersionRange::Intersection(a, b) => {
                and_operand(f, a)?;
                f.write_str(" && ")?;
                and_operand(f, b)
            }
        }
    }
}

fn and_operand(f: &mut fmt::Formatter<'_>, r: &VersionRange) -> fmt::Result {
    if matches!(r, VersionRange::Union(..)) {
        write!(f, "({r})")
    } else {
        write!(f, "{r}")
    }
}
