//! Every bundled theme's resolved tokens as JSON, for the Swift UI's `DesignSystem` to read.
//!
//! The derivation rules (`wormhole-core::theme::tokens`) are the single source of truth for what a
//! token is; Swift never re-implements them, it loads this dump. Each bundled theme is written
//! as it is shown normally and as Increase Contrast shows it (`ThemeTokens::high_contrast`).
//! A theme is light or dark by itself (`appearance`); the app picks one theme for each mode.
//! Deterministic: same core, same bytes. `examples/dump_themes.rs` writes it, and the test
//! `tests/theme_dump.rs` fails when the committed copy is stale.

use std::fmt::Write as _;

use wormhole_core::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT};
use wormhole_core::theme::tokens::{
    self, Appearance, Color, METRIC_TOKENS, Metrics, ThemeTokens, UI_TOKENS, builtins,
};

/// Bumped when the shape of the JSON changes, so the Swift loader can refuse a file it can't read.
const FORMAT: u32 = 1;

pub fn json() -> String {
    let mut j = String::new();
    let _ = write!(
        j,
        "{{\n\"format\": {FORMAT},\n\"defaultDark\": {},\n\"defaultLight\": {},\n",
        s(&id(DEFAULT_THEME)),
        s(&id(DEFAULT_THEME_LIGHT))
    );
    j.push_str("\"catalog\": [");
    let names = UI_TOKENS.iter().chain(METRIC_TOKENS);
    j.push_str(
        &names
            .map(|t| format!("[{}, {}, {}]", s(t.name), s(t.group), s(t.description)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    j.push_str("],\n\"themes\": [\n");
    j.push_str(&builtins().iter().map(theme).collect::<Vec<_>>().join(",\n"));
    j.push_str("\n]\n}\n");
    j
}

/// The slug that selects a bundled theme, as `settings.theme` accepts it.
fn id(name: &str) -> String {
    tokens::slug::builtin_slug(name).map_or_else(|| tokens::slug::derive_id(name), str::to_string)
}

fn theme(t: &ThemeTokens) -> String {
    let family = tokens::builtin_family(&t.name)
        .map_or("null".into(), |(f, short)| format!("[{}, {}]", s(f), s(short)));
    let appearance = match t.appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    };
    format!(
        "{{\"id\": {}, \"name\": {}, \"family\": {family}, \"appearance\": \"{appearance}\",\n \"terminal\": {},\n \"normal\": {},\n \"highContrast\": {}}}",
        s(&id(&t.name)),
        s(&t.name),
        terminal(t),
        variant(t),
        variant(&t.high_contrast()),
    )
}

fn terminal(t: &ThemeTokens) -> String {
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

/// One resolved variant: the UI tokens, the syntax styles and the metrics.
fn variant(t: &ThemeTokens) -> String {
    let ui =
        t.ui_tokens().map(|(name, c)| format!("{}: {}", s(name), color(c))).collect::<Vec<_>>().join(", ");
    let syntax = t
        .syntax
        .iter()
        .map(|(k, v)| format!("{}: [{}, {}, {}]", s(k), color(v.color), v.bold, v.italic))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{\"ui\": {{{ui}}},\n  \"syntax\": {{{syntax}}},\n  \"metrics\": {}}}", metrics(&t.metrics))
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
    ];
    let body = keys.iter().filter_map(|k| m.get(k).map(|v| format!("\"{k}\": {v}"))).collect::<Vec<_>>();
    format!("{{{}}}", body.join(", "))
}

/// `[r, g, b, a]`, the f32 channels as the app draws them (shortest text that reads back exactly).
fn color(c: Color) -> String {
    format!("[{}, {}, {}, {}]", c.r, c.g, c.b, c.a)
}

fn s(text: &str) -> String {
    format!("{text:?}")
}
