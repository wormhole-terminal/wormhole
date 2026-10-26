//! A Warp theme file (`~/.warp/themes/*.yaml`) as a palette. The format is a few keys and two
//! small maps of colours, so this reads that much of YAML (block and `{ … }` maps, quoted or bare
//! scalars, `#` comments) and nothing more.

use std::collections::BTreeMap;

use crate::theme::{Builder, Theme, parse_hex};

/// The ANSI colours' names in palette order, normal then bright.
const ANSI: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

/// The scalars of a YAML text by dotted path (`terminal_colors.normal.red`). A key whose value is a
/// map of its own (a gradient) has no scalar, so it is not in the result.
pub(crate) fn scalars(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    // The keys of the maps being read, each with the indent of its own line.
    let mut stack: Vec<(usize, String)> = Vec::new();
    for line in text.lines() {
        let line = strip_comment(line);
        let content = line.trim();
        if content.is_empty() || content.starts_with('-') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        while stack.last().is_some_and(|(at, _)| *at >= indent) {
            stack.pop();
        }
        let Some((key, value)) = content.split_once(':') else { continue };
        let key = scalar(key);
        let value = value.trim();
        let path =
            stack.iter().map(|(_, key)| key.as_str()).chain([key.as_str()]).collect::<Vec<_>>().join(".");
        if value.is_empty() {
            stack.push((indent, key));
        } else if let Some(inner) = value.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
            flow(&path, inner, &mut out);
        } else {
            out.insert(path, scalar(value));
        }
    }
    out
}

/// The pairs of a `{ a: b, c: { d: e } }` map, one level of nesting deep.
fn flow(prefix: &str, inner: &str, out: &mut BTreeMap<String, String>) {
    let mut depth = 0;
    let mut start = 0;
    let mut parts = Vec::new();
    for (i, c) in inner.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&inner[start..]);
    for part in parts {
        let Some((key, value)) = part.split_once(':') else { continue };
        let (key, value) = (scalar(key), value.trim());
        let path = format!("{prefix}.{key}");
        match value.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
            Some(nested) => flow(&path, nested, out),
            None => {
                out.insert(path, scalar(value));
            }
        }
    }
}

/// A scalar without its quotes.
fn scalar(text: &str) -> String {
    let text = text.trim();
    for quote in ['\'', '"'] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|rest| rest.strip_suffix(quote)) {
            return inner.to_string();
        }
    }
    text.to_string()
}

/// The line without a trailing comment: a `#` after whitespace, outside quotes. `'#112233'` and
/// `#112233` as a bare value after `: ` are colours, so only a `#` that starts the line or follows
/// a quoted scalar or a space-separated value counts.
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut previous = ' ';
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            // A bare `#rrggbb` right after `key:` is a colour (YAML would read it as a comment, so
            // files quote them); keep it.
            None if c == '#' && previous.is_whitespace() && !line[..i].trim_end().ends_with(':') => {
                return &line[..i];
            }
            None => {}
        }
        previous = c;
    }
    line
}

/// The palette of a theme file, named `label`. It needs a background or a foreground.
pub(crate) fn palette(label: &str, text: &str) -> Option<Theme> {
    let values = scalars(text);
    let color = |path: &str| values.get(path).and_then(|v| parse_hex(v).ok());
    let mut builder = Builder {
        foreground: color("foreground"),
        background: color("background"),
        cursor: color("cursor"),
        ..Builder::default()
    };
    for (i, name) in ANSI.iter().enumerate() {
        builder.palette[i] = color(&format!("terminal_colors.normal.{name}"));
        builder.palette[i + 8] = color(&format!("terminal_colors.bright.{name}"));
    }
    (builder.background.is_some() || builder.foreground.is_some()).then(|| builder.finish(label))
}

/// The theme's own name, from its `name` key.
pub(crate) fn name(text: &str) -> Option<String> {
    scalars(text).remove("name").filter(|name| !name.is_empty())
}
