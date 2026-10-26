//! A Warp theme file (`~/.warp/themes/*.yaml`) as a palette. The format is a few keys and two
//! small maps of colours, so this reads that much of YAML (block and `{ … }` maps, quoted or bare
//! scalars, `#` comments) and nothing more.

use std::collections::BTreeMap;

use super::proposal::{Proposal, Reason, Setting};
use crate::theme::{Builder, parse_hex};

/// How deep the maps of a theme file go: `terminal_colors.normal.red` is three keys. Anything
/// nested deeper is not read, so a file of thousands of indents costs a line each, not a path.
const MAX_KEYS: usize = 4;

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
        if stack.len() >= MAX_KEYS {
            continue;
        }
        let Some((key, value)) = content.split_once(':') else { continue };
        let key = scalar(key);
        let value = value.trim();
        let path =
            stack.iter().map(|(_, key)| key.as_str()).chain([key.as_str()]).collect::<Vec<_>>().join(".");
        if value.is_empty() {
            stack.push((indent, key));
        } else if let Some(inner) = value.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
            flow(&path, inner, 1, &mut out);
        } else {
            out.insert(path, scalar(value));
        }
    }
    out
}

/// The pairs of a `{ a: b, c: { d: e } }` map; `level` counts the maps being read, and a map inside
/// the second is left alone (a flow map written as deep as a hostile file likes would otherwise
/// be re-scanned at every level).
fn flow(prefix: &str, inner: &str, level: usize, out: &mut BTreeMap<String, String>) {
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
            Some(nested) if level < 2 => flow(&path, nested, level + 1, out),
            Some(_) => {}
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

/// Offer the palette of a theme file, named `label`, for `setting`, when it has a background and a
/// foreground. `from` is how the review list names it.
pub(crate) fn propose(proposal: &mut Proposal, setting: Setting, from: String, label: &str, text: &str) {
    let values = scalars(text);
    let color = |path: &str| values.get(path).and_then(|v| parse_hex(v).ok());
    let mut builder = Builder {
        foreground: color("foreground"),
        background: color("background"),
        cursor: color("cursor"),
        ..Builder::default()
    };
    builder.fill_ansi(|bright, name| {
        color(&format!("terminal_colors.{}.{name}", if bright { "bright" } else { "normal" }))
    });
    if builder.has_any() {
        proposal.propose_palette(setting, from, label, builder);
    } else {
        proposal.skip(from, Reason::UnsupportedValue);
    }
}

/// The theme's own name, from its `name` key.
pub(crate) fn name(text: &str) -> Option<String> {
    scalars(text).remove("name").filter(|name| !name.is_empty())
}
