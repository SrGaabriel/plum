use std::ops::Range;

use crate::syntax::{Field, FieldLine, Name};

pub fn parse(source: &str) -> Vec<Field> {
    let mut parser = Parser {
        source,
        pos: 0,
        line: 1,
        line_start: 0,
    };
    parser.block(BlockEnd::Top)
}

#[derive(Clone, Copy)]
enum BlockEnd {
    Top,
    Brace,
    Layout { parent_col: usize },
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    line_start: usize,
}

const TAB_WIDTH: usize = 8;

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

impl<'a> Parser<'a> {
    fn bytes(&self) -> &'a [u8] {
        self.source.as_bytes()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn cur(&self) -> u8 {
        self.bytes()[self.pos]
    }

    fn cur_is(&self, b: u8) -> bool {
        !self.at_end() && self.cur() == b
    }

    fn peek1(&self) -> Option<u8> {
        self.bytes().get(self.pos + 1).copied()
    }

    fn skip_inline_ws(&mut self) {
        while !self.at_end() && matches!(self.cur(), b' ' | b'\t' | b'\r') {
            self.pos += 1;
        }
    }

    fn skip_line(&mut self) {
        while !self.at_end() && self.cur() != b'\n' {
            self.pos += 1;
        }
        if !self.at_end() {
            self.pos += 1;
            self.line += 1;
            self.line_start = self.pos;
        }
    }

    fn column(&self) -> usize {
        let mut col = 0;
        for &b in &self.bytes()[self.line_start..self.pos] {
            if b == b'\t' {
                col += TAB_WIDTH - col % TAB_WIDTH;
            } else {
                col += 1;
            }
        }
        col
    }

    fn ident_len_here(&self) -> usize {
        self.bytes()[self.pos..]
            .iter()
            .take_while(|&&b| is_ident(b))
            .count()
    }

    fn skip_to_token(&mut self) -> Option<usize> {
        loop {
            self.skip_inline_ws();
            if self.at_end() {
                return None;
            }
            match self.cur() {
                b'\n' => {
                    self.pos += 1;
                    self.line += 1;
                    self.line_start = self.pos;
                }
                b'-' if self.peek1() == Some(b'-') => self.skip_line(),
                _ => return Some(self.column()),
            }
        }
    }

    fn block(&mut self, end: BlockEnd) -> Vec<Field> {
        let mut items = Vec::new();
        while let Some(col) = self.skip_to_token() {
            if self.cur_is(b'}') {
                if let BlockEnd::Brace = end {
                    self.pos += 1;
                }
                break;
            }
            if let BlockEnd::Layout { parent_col } = end
                && col <= parent_col
            {
                break;
            }
            if self.ident_len_here() == 0 {
                self.skip_line();
                continue;
            }
            items.push(self.parse_element(col));
        }
        items
    }

    fn parse_element(&mut self, col: usize) -> Field {
        let name_line = self.line;
        let (raw, name_span) = self.read_name();
        let name = Name {
            text: raw.to_ascii_lowercase(),
            span: name_span,
        };
        self.skip_inline_ws();

        if self.cur_is(b':') {
            self.pos += 1;
            self.parse_field(name, col, name_line)
        } else {
            self.parse_section(name, col)
        }
    }

    fn parse_field(&mut self, name: Name, col: usize, name_line: usize) -> Field {
        let mut value = Vec::new();
        let (text, brace_stop) = self.read_value();
        if !text.is_empty() {
            value.push(FieldLine {
                text,
                line: name_line,
            });
        }
        if !brace_stop {
            while let Some(col2) = self.skip_to_token() {
                if col2 <= col || self.cur_is(b'}') {
                    break;
                }
                let line = self.line;
                let (text, brace_stop) = self.read_value();
                if !text.is_empty() {
                    value.push(FieldLine { text, line });
                }
                if brace_stop {
                    break;
                }
            }
        }
        Field::Field { name, value }
    }

    fn parse_section(&mut self, name: Name, col: usize) -> Field {
        let (arg, arg_span, brace) = self.read_arg();
        let fields = if brace || (self.skip_to_token().is_some() && self.cur_is(b'{')) {
            self.pos += 1;
            self.block(BlockEnd::Brace)
        } else {
            self.block(BlockEnd::Layout { parent_col: col })
        };
        Field::Section {
            name,
            arg,
            arg_span,
            fields,
        }
    }

    fn read_name(&mut self) -> (String, Range<usize>) {
        let start = self.pos;
        while !self.at_end() && is_ident(self.cur()) {
            self.pos += 1;
        }
        (self.source[start..self.pos].to_string(), start..self.pos)
    }

    fn read_value(&mut self) -> (String, bool) {
        self.skip_inline_ws();
        let start = self.pos;
        let mut depth: i32 = 0;
        let mut brace_stop = false;
        while !self.at_end() {
            match self.cur() {
                b'\n' => break,
                b'{' => depth += 1,
                b'}' if depth == 0 => {
                    brace_stop = true;
                    break;
                }
                b'}' => depth -= 1,
                _ => {}
            }
            self.pos += 1;
        }
        (
            self.source[start..self.pos].trim_end().to_string(),
            brace_stop,
        )
    }

    fn read_arg(&mut self) -> (String, Range<usize>, bool) {
        let start = self.pos;
        let mut brace = false;
        while !self.at_end() {
            match self.cur() {
                b'\n' => break,
                b'{' => {
                    brace = true;
                    break;
                }
                _ => self.pos += 1,
            }
        }
        let raw = &self.source[start..self.pos];
        let trimmed = raw.trim();
        let offset = start + (raw.len() - raw.trim_start().len());
        (trimmed.to_string(), offset..offset + trimmed.len(), brace)
    }
}
