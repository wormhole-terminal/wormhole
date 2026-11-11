//! The JSON pieces of a resolved theme, shared by the bundled-theme dump (`theme_dump`) and the
//! runtime resolver (`theme_resolve`), so Swift reads one shape from both.

use wormhole_core::theme::tokens::{Appearance, Color, Metrics, ThemeTokens, Translucency};

/// A JSON string. Theme names come from users' files, so this escapes properly.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `[r, g, b, a]`, the f32 channels as the app draws them (shortest text that reads back exactly).
pub(crate) fn color(c: Color) -> String {
    format!("[{}, {}, {}, {}]", c.r, c.g, c.b, c.a)
}

pub(crate) fn appearance(t: &ThemeTokens) -> &'static str {
    match t.appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    }
}

pub(crate) fn terminal(t: &ThemeTokens) -> String {
    let c = &t.terminal;
    let opt = |c: Option<Color>| c.map_or("null".into(), color);
    format!(
        "{{\"foreground\": {}, \"background\": {}, \"cursor\": {}, \"cursorText\": {}, \"selectionBackground\": {}, \"selectionForeground\": {}, \"ansi\": [{}]}}",
        color(c.foreground),
        color(c.background),
        color(c.cursor),
        opt(c.cursor_text),
        color(c.selection_background),
        opt(c.selection_foreground),
        c.ansi.iter().map(|c| color(*c)).collect::<Vec<_>>().join(", "),
    )
}

/// One resolved variant: the UI tokens, the syntax styles, the metrics and, for a window that lets the
/// desktop through at `look`, the text colours core judges again on what shows behind it.
pub(crate) fn variant(t: &ThemeTokens, look: Option<Translucency>) -> String {
    let ui = t
        .ui_tokens()
        .map(|(name, c)| format!("{}: {}", string(name), color(c)))
        .collect::<Vec<_>>()
        .join(", ");
    let syntax = t
        .syntax
        .iter()
        .map(|(k, v)| format!("{}: [{}, {}, {}]", string(k), color(v.color), v.bold, v.italic))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\"ui\": {{{ui}}},\n  \"syntax\": {{{syntax}}},\n  \"metrics\": {},\n  \"translucent\": {}}}",
        metrics(&t.metrics),
        look.map_or("null".into(), |look| translucent(t, look)),
    )
}

/// `ThemeTokens::translucent`'s colours, or `null` when the window at `look` is opaque.
fn translucent(t: &ThemeTokens, look: Translucency) -> String {
    t.translucent(look).map_or("null".into(), |c| {
        format!(
            "{{\"text\": {}, \"muted\": {}, \"faint\": {}, \"branch\": {}, \"selectedText\": {}, \"selectedMuted\": {}}}",
            color(c.text),
            color(c.muted),
            color(c.faint),
            color(c.branch),
            color(c.selected_text),
            color(c.selected_muted),
        )
    })
}

fn metrics(m: &Metrics) -> String {
    let keys = [
        "radius_sm",
        "radius_chip",
        "radius",
        "radius_lg",
        "border_width",
        "divider_width",
        "focus_ring_width",
        "ui_font_size",
    ];
    let mut body = keys.iter().filter_map(|k| m.get(k).map(|v| format!("\"{k}\": {v}"))).collect::<Vec<_>>();
    body.push(format!("\"ui_font_family\": {}", m.ui_font_family.as_deref().map_or("null".into(), string)));
    format!("{{{}}}", body.join(", "))
}
