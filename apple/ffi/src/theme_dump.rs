//! Every bundled theme's resolved tokens as JSON, for the Swift UI's `DesignSystem` to read.
//!
//! The derivation rules (`wormhole-core::theme::tokens`) are the single source of truth for what a
//! token is; Swift never re-implements them, it loads this dump. Each bundled theme is written
//! as it is shown normally and as Increase Contrast shows it (`ThemeTokens::high_contrast`).
//! A theme is light or dark by itself (`appearance`); the app picks one theme for each mode.
//! The normal variant also carries the text colours of a window at the settings' default
//! opacity and blur (`translucent`); Increase Contrast windows are opaque, so theirs is `null`.
//! Other opacities, primary and branch colours and user themes are the runtime resolver's
//! (`theme_resolve`). Deterministic: same core, same bytes. `examples/dump_themes.rs` writes it, and
//! `tests/theme_dump.rs` fails when the committed copy is stale.

use std::fmt::Write as _;

use wormhole_core::config::Settings;
use wormhole_core::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT};
use wormhole_core::theme::tokens::{self, METRIC_TOKENS, ThemeTokens, Translucency, UI_TOKENS, builtins};

use crate::theme_json::{appearance, string, terminal, variant};

/// Bumped when the shape of the JSON changes, so the Swift loader can refuse a file it can't read.
const FORMAT: u32 = 2;

pub fn json() -> String {
    let defaults = Settings::default();
    let look = Translucency { opacity: defaults.opacity, blur: defaults.blur };
    let mut j = String::new();
    let _ = write!(
        j,
        "{{\n\"format\": {FORMAT},\n\"defaultDark\": {},\n\"defaultLight\": {},\n\"defaultLook\": {{\"opacity\": {}, \"blur\": {}}},\n",
        string(&id(DEFAULT_THEME)),
        string(&id(DEFAULT_THEME_LIGHT)),
        look.opacity,
        look.blur,
    );
    j.push_str("\"catalog\": [");
    let names = UI_TOKENS.iter().chain(METRIC_TOKENS);
    j.push_str(
        &names
            .map(|t| format!("[{}, {}, {}]", string(t.name), string(t.group), string(t.description)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    j.push_str("],\n\"themes\": [\n");
    j.push_str(&builtins().iter().map(|t| theme(t, look)).collect::<Vec<_>>().join(",\n"));
    j.push_str("\n]\n}\n");
    j
}

/// The slug that selects a bundled theme, as `settings.theme` accepts it.
pub(crate) fn id(name: &str) -> String {
    tokens::slug::builtin_slug(name).map_or_else(|| tokens::slug::derive_id(name), str::to_string)
}

fn theme(t: &ThemeTokens, look: Translucency) -> String {
    let family = tokens::builtin_family(&t.name)
        .map_or("null".into(), |(f, short)| format!("[{}, {}]", string(f), string(short)));
    format!(
        "{{\"id\": {}, \"name\": {}, \"family\": {family}, \"appearance\": \"{}\",\n \"terminal\": {},\n \"normal\": {},\n \"highContrast\": {}}}",
        string(&id(&t.name)),
        string(&t.name),
        appearance(t),
        terminal(t),
        variant(t, Some(look)),
        variant(&t.high_contrast(), None),
    )
}
