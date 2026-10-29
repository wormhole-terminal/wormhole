//! Writing a version 1 file.

use serde_json::{Map, Number, Value};

use super::super::keys::{METRIC_ORDER, TERMINAL_ORDER, metric_internal, new_node};
use super::super::refs::{Rename, rewrite_refs};
use super::super::{BaseLayer, CURRENT_VERSION};
use crate::theme::tokens::file::{MetricsFile, SyntaxOverride, TerminalFile, ThemeFile, ansi_index};
use crate::theme::tokens::model::Appearance;

/// `file` as version 1 text: keys in a fixed, readable order, expressions with version 1
/// names, no comments (the file is written again from what was read).
pub fn write_v1(file: &ThemeFile) -> String {
    let mut root = Map::new();
    if let Some(schema) = &file.schema {
        root.insert("$schema".into(), schema.clone().into());
    }
    root.insert("version".into(), CURRENT_VERSION.into());
    if let Some(id) = &file.id {
        root.insert("id".into(), id.clone().into());
    }
    if let Some(name) = &file.name {
        root.insert("name".into(), name.clone().into());
    }
    if let Some(base) = &file.base {
        root.insert("base".into(), base_value(base));
    }
    layers(&mut root, file);
    if !file.variants.is_empty() {
        let items = file
            .variants
            .iter()
            .map(|v| {
                let mut m = Map::new();
                m.insert("name".into(), v.name.clone().into());
                if let Some(base) = &v.base {
                    m.insert("base".into(), base_value(base));
                }
                layers(&mut m, &v.to_file());
                Value::Object(m)
            })
            .collect();
        root.insert("variants".into(), Value::Array(items));
    }
    // Plain maps of strings and numbers cannot fail to serialize.
    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_else(|_| "{}".into()) + "\n"
}

fn base_value(base: &BaseLayer) -> Value {
    let mut m = Map::new();
    m.insert("from".into(), base.from.clone().into());
    if let Some(copied) = &base.copied {
        m.insert("copied".into(), copied.clone().into());
    }
    layers(&mut m, &base.file);
    Value::Object(m)
}

/// The five blocks every layer has, in the order a file lists them.
fn layers(into: &mut Map<String, Value>, file: &ThemeFile) {
    if let Some(a) = file.appearance {
        let word = match a {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        };
        into.insert("appearance".into(), word.into());
    }
    if !file.terminal.is_empty() {
        into.insert("terminal".into(), terminal_value(&file.terminal));
    }
    if !file.ui.is_empty() {
        into.insert("ui".into(), Value::Object(file.ui.iter().map(|(k, v)| (k.clone(), expr(v))).collect()));
    }
    if !file.syntax.is_empty() {
        let syntax = file.syntax.iter().map(|(k, v)| (k.clone(), syntax_value(v))).collect();
        into.insert("syntax".into(), Value::Object(syntax));
    }
    if !file.metrics.is_empty() {
        into.insert("metrics".into(), metrics_value(&file.metrics));
    }
}

/// An expression with the resolver's node names turned into version 1's.
fn expr(src: &str) -> Value {
    expression_text(src).into()
}

fn terminal_value(t: &TerminalFile) -> Value {
    let mut m = Map::new();
    let fields = [
        ("background", &t.background),
        ("foreground", &t.foreground),
        ("cursor", &t.cursor),
        ("cursor.text", &t.cursor_text),
        ("selection.background", &t.selection_background),
        ("selection.foreground", &t.selection_foreground),
    ];
    debug_assert!(fields.iter().map(|(k, _)| *k).eq(TERMINAL_ORDER.iter().copied()));
    for (key, value) in fields {
        if let Some(v) = value {
            m.insert(key.into(), expr(v));
        }
    }
    // By palette index, whichever of its names the file used.
    let mut ansi: Vec<(usize, &String)> =
        t.ansi.iter().filter_map(|(k, v)| ansi_index(k).map(|i| (i, v))).collect();
    ansi.sort_by_key(|(i, _)| *i);
    for (i, v) in ansi {
        m.insert(format!("ansi.{i}"), expr(v));
    }
    Value::Object(m)
}

fn syntax_value(s: &SyntaxOverride) -> Value {
    match s {
        SyntaxOverride::Color(c) => expr(c),
        SyntaxOverride::Style { color, bold, italic } => {
            let mut m = Map::new();
            if let Some(c) = color {
                m.insert("color".into(), expr(c));
            }
            if let Some(b) = bold {
                m.insert("bold".into(), (*b).into());
            }
            if let Some(i) = italic {
                m.insert("italic".into(), (*i).into());
            }
            Value::Object(m)
        }
    }
}

fn metrics_value(m: &MetricsFile) -> Value {
    let mut out = Map::new();
    for key in METRIC_ORDER {
        let Some(internal) = metric_internal(key) else { continue };
        if internal == "ui_font_family" {
            if let Some(f) = &m.ui_font_family {
                out.insert((*key).into(), f.clone().into());
            }
        } else if let Some(v) = m.get(internal) {
            out.insert((*key).into(), number(v));
        }
    }
    Value::Object(out)
}

/// A whole number of pixels without a `.0`; a fraction as its shortest decimal (not the f64
/// the f32 widens to).
fn number(v: f32) -> Value {
    if v.fract() == 0.0 && v.abs() < 1e7 {
        // Far below where an f32 stops holding every integer, so the cast is exact.
        return Value::from(v as i64);
    }
    v.to_string().parse::<f64>().ok().and_then(Number::from_f64).map_or(Value::Null, Value::Number)
}

/// An expression as a version 1 file writes it (the resolver's node names turned into
/// version 1's), as text.
pub(in super::super) fn expression_text(src: &str) -> String {
    let text = rewrite_refs(src, &|name| new_node(name).map_or(Rename::Keep, Rename::To));
    text.unwrap_or_else(|_| src.to_string())
}
