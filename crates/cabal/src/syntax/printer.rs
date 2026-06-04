use crate::syntax::Field;

pub fn print(fields: &[Field]) -> String {
    let mut out = String::new();
    print_block(fields, 0, &mut out);
    out
}

fn print_block(fields: &[Field], indent: usize, out: &mut String) {
    let widest = fields
        .iter()
        .filter_map(|f| match f {
            Field::Field { name, .. } => Some(name.text.len()),
            Field::Section { .. } => None,
        })
        .max()
        .unwrap_or(0);
    let column = indent + widest + 2;

    for (i, field) in fields.iter().enumerate() {
        match field {
            Field::Field { name, value } => {
                pad(out, indent);
                out.push_str(&name.text);
                out.push(':');
                match value.split_first() {
                    None => out.push('\n'),
                    Some((first, rest)) => {
                        let written = indent + name.text.len() + 1;
                        pad(out, column.saturating_sub(written).max(1));
                        out.push_str(&first.text);
                        out.push('\n');
                        for line in rest {
                            pad(out, column);
                            out.push_str(&line.text);
                            out.push('\n');
                        }
                    }
                }
            }
            Field::Section {
                name,
                arg,
                fields: body,
                ..
            } => {
                if i > 0 {
                    out.push('\n');
                }
                pad(out, indent);
                out.push_str(&name.text);
                if !arg.is_empty() {
                    out.push(' ');
                    out.push_str(arg);
                }
                out.push('\n');
                print_block(body, indent + 4, out);
            }
        }
    }
}

fn pad(out: &mut String, n: usize) {
    for _ in 0..n {
        out.push(' ');
    }
}
