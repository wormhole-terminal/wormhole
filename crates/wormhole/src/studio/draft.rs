//! The theme being edited, as a small theme *file* that `extends` the current theme:
//! reading and writing the draft's own values by token name, what changed, the token
//! list's groups and filters, and the names a save or an export uses. Pure, so it is
//! tested without a window.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use wormhole_core::config::Config;
use wormhole_core::theme::registry::{ThemeId, UserThemes};
use wormhole_core::theme::tokens::{
    Appearance, Area, METRIC_TOKENS, SyntaxOverride, ThemeFile, ThemeTokens, UI_TOKENS, default_rule,
    syntax_keys, used_in,
};

/// What the Studio asks of the themes folder, as functions, so a test can stand in and never
/// read or write the user's.
#[derive(Clone, Copy)]
pub(super) struct Themes {
    /// Turns a draft into tokens, with the user's themes available for `extends`.
    pub(super) resolve: Resolve,
    /// The file a theme saved under this name would replace, if the folder has one.
    pub(super) saved_path: fn(&str) -> Option<PathBuf>,
    /// Writes the draft into the folder and says where it went.
    pub(super) save: fn(&ThemeFile) -> Result<PathBuf, String>,
    /// Whether a pick's value (a name, a path or a slug) names the theme saved at this path.
    pub(super) selects: fn(&str, &Path) -> bool,
}

impl Themes {
    /// The user's themes folder.
    pub(super) const USER: Themes =
        Themes {
        resolve: resolve_in_profile,
        saved_path: saved_in_profile,
        save: save_in_profile,
        selects: selects_in_profile,
    };
}

// The Studio edits the themes of the profile Wormhole is in (RF-21): its own folder, not Default's.
fn resolve_in_profile(file: &ThemeFile) -> Result<ThemeTokens, String> {
    wormhole_core::config::resolve_theme_file_in(&crate::active_profile::paths().themes_dir(), file)
}

fn saved_in_profile(name: &str) -> Option<PathBuf> {
    Config::saved_theme_path_in(&crate::active_profile::paths().themes_dir(), name)
}

fn save_in_profile(file: &ThemeFile) -> Result<PathBuf, String> {
    Config::save_theme_to(&crate::active_profile::paths().themes_dir(), file)
}

fn selects_in_profile(setting: &str, path: &Path) -> bool {
    let themes = UserThemes::read(&crate::active_profile::paths().themes_dir());
    themes.select(setting) == Some(ThemeId::File(path.to_path_buf()))
}

/// Turns a draft into tokens. The Studio resolves with the user's themes available for
/// `extends` (`wormhole_core::config::resolve_theme_file`); a test passes one that
/// reads nothing from disk.
pub(super) type Resolve = fn(&ThemeFile) -> Result<ThemeTokens, String>;

/// Terminal colors the token editor can edit, as `terminal.*` token names.
pub(super) const TERMINAL_TOKENS: &[&str] =
    &["terminal.background", "terminal.foreground", "terminal.cursor", "terminal.selection_background"];

/// The 16 palette slots, as `terminal.ansi.*` token names.
pub(super) const ANSI_NAMES: [&str; 16] = [
    "terminal.ansi.0",
    "terminal.ansi.1",
    "terminal.ansi.2",
    "terminal.ansi.3",
    "terminal.ansi.4",
    "terminal.ansi.5",
    "terminal.ansi.6",
    "terminal.ansi.7",
    "terminal.ansi.8",
    "terminal.ansi.9",
    "terminal.ansi.10",
    "terminal.ansi.11",
    "terminal.ansi.12",
    "terminal.ansi.13",
    "terminal.ansi.14",
    "terminal.ansi.15",
];

/// A draft that starts out identical to `base`: extend it by name if it can be
/// resolved that way (built-in or saved JSON theme), otherwise copy its palette.
pub(super) fn draft_for(base: &ThemeTokens, resolve: Resolve) -> ThemeFile {
    let mut draft = ThemeFile { name: Some(format!("{} Custom", base.name)), ..Default::default() };
    let by_name = ThemeFile { extends: Some(base.name.clone()), ..Default::default() };
    if resolve(&by_name).is_ok_and(|t| t.terminal == base.terminal) {
        draft.extends = Some(base.name.clone());
    } else {
        let t = &base.terminal;
        draft.terminal.foreground = Some(t.foreground.to_hex());
        draft.terminal.background = Some(t.background.to_hex());
        draft.terminal.cursor = Some(t.cursor.to_hex());
        draft.terminal.selection_background = Some(t.selection_background.to_hex());
        // Set only when the theme sets them: otherwise they keep following the background and
        // foreground, as they do in the theme being copied.
        draft.terminal.cursor_text = t.cursor_text.map(|c| c.to_hex());
        draft.terminal.selection_foreground = t.selection_foreground.map(|c| c.to_hex());
        for (i, color) in t.ansi.iter().enumerate() {
            draft.terminal.ansi.insert(i.to_string(), color.to_hex());
        }
    }
    draft
}

/// Whether `name` is a metric (`metrics.radius_chip`) rather than a color.
pub(super) fn is_metric(name: &str) -> bool {
    name.starts_with("metrics.")
}

/// A metric as the token editor shows and takes it: pixels without a unit.
pub(super) fn number(value: f32) -> String {
    format!("{value}")
}

/// The draft's own value for a token (not the extended or derived one).
pub(super) fn override_of(draft: &ThemeFile, name: &str) -> Option<String> {
    if is_metric(name) {
        return draft.metrics.get(name).map(number);
    }
    if let Some(index) = name.strip_prefix("terminal.ansi.") {
        return draft.terminal.ansi.get(index).cloned();
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        return match draft.syntax.get(key)? {
            SyntaxOverride::Color(color) => Some(color.clone()),
            SyntaxOverride::Style { color, .. } => color.clone(),
        };
    }
    match name {
        "terminal.background" => draft.terminal.background.clone(),
        "terminal.foreground" => draft.terminal.foreground.clone(),
        "terminal.cursor" => draft.terminal.cursor.clone(),
        "terminal.selection_background" => draft.terminal.selection_background.clone(),
        _ => draft.ui.get(name).cloned(),
    }
}

pub(super) fn set_override(draft: &mut ThemeFile, name: &str, value: Option<String>) -> Result<(), String> {
    fn put(slot: &mut Option<String>, value: Option<String>) {
        *slot = value;
    }
    if is_metric(name) {
        let value = match value {
            Some(v) => Some(
                v.trim_end_matches("px")
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| format!("{name}: {v:?} is not a number of pixels"))?,
            ),
            None => None,
        };
        draft.metrics.set(name, value);
        return Ok(());
    }
    if let Some(index) = name.strip_prefix("terminal.ansi.") {
        match value {
            Some(v) => draft.terminal.ansi.insert(index.to_string(), v),
            None => draft.terminal.ansi.remove(index),
        };
        return Ok(());
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        set_syntax_color(draft, key, value);
        return Ok(());
    }
    match name {
        "terminal.background" => put(&mut draft.terminal.background, value),
        "terminal.foreground" => put(&mut draft.terminal.foreground, value),
        "terminal.cursor" => put(&mut draft.terminal.cursor, value),
        "terminal.selection_background" => put(&mut draft.terminal.selection_background, value),
        _ => {
            match value {
                Some(v) => draft.ui.insert(name.to_string(), v),
                None => draft.ui.remove(name),
            };
        }
    }
    Ok(())
}

/// A syntax key's color, keeping the bold and italic a theme may have set next to it.
fn set_syntax_color(draft: &mut ThemeFile, key: &str, value: Option<String>) {
    let style = match draft.syntax.remove(key) {
        Some(SyntaxOverride::Style { bold, italic, .. }) if bold.is_some() || italic.is_some() => {
            Some(SyntaxOverride::Style { color: value, bold, italic })
        }
        _ => value.map(SyntaxOverride::Color),
    };
    if let Some(style) = style {
        draft.syntax.insert(key.to_string(), style);
    }
}

/// Every value the draft sets itself, by token name (bold and italic as their own entries).
fn own_values(draft: &ThemeFile) -> BTreeMap<String, String> {
    let mut values: BTreeMap<String, String> = draft.ui.clone();
    let t = &draft.terminal;
    let fields = [
        ("foreground", &t.foreground),
        ("background", &t.background),
        ("cursor", &t.cursor),
        ("cursor_text", &t.cursor_text),
        ("selection_background", &t.selection_background),
        ("selection_foreground", &t.selection_foreground),
    ];
    for (key, value) in fields {
        if let Some(value) = value {
            values.insert(format!("terminal.{key}"), value.clone());
        }
    }
    for (key, value) in &t.ansi {
        values.insert(format!("terminal.ansi.{key}"), value.clone());
    }
    for info in METRIC_TOKENS {
        if let Some(value) = draft.metrics.get(info.name) {
            values.insert(info.name.to_string(), number(value));
        }
    }
    for (key, style) in &draft.syntax {
        let (color, bold, italic) = match style {
            SyntaxOverride::Color(color) => (Some(color), None, None),
            SyntaxOverride::Style { color, bold, italic } => (color.as_ref(), *bold, *italic),
        };
        if let Some(color) = color {
            values.insert(format!("syntax.{key}"), color.clone());
        }
        if let Some(bold) = bold {
            values.insert(format!("syntax.{key}:bold"), bold.to_string());
        }
        if let Some(italic) = italic {
            values.insert(format!("syntax.{key}:italic"), italic.to_string());
        }
    }
    values
}

/// How many values `draft` sets differently from `initial` (added, changed or removed):
/// what closing without saving would lose.
pub(super) fn changes(initial: &ThemeFile, draft: &ThemeFile) -> usize {
    let (before, after) = (own_values(initial), own_values(draft));
    let names: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    names.into_iter().filter(|name| before.get(*name) != after.get(*name)).count()
}

/// The draft as Export and Copy JSON write it: the theme file itself, so only what the draft
/// sets, and expressions exactly as typed.
pub(super) fn export_json(draft: &ThemeFile) -> String {
    draft.to_json_pretty()
}

/// The file name Export suggests: the theme's name in lower case with dashes.
pub(super) fn export_file_name(name: &str) -> String {
    let stem: String =
        name.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let mut stem = stem.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-");
    if stem.is_empty() {
        stem = "theme".into();
    }
    format!("{stem}.json")
}

/// Where Export's file dialog starts: the folder the last export went to, while it is still
/// a folder, otherwise the home folder (and the root of the disk where there is none either).
pub(super) fn export_dir(
    remembered: Option<&str>,
    home: Option<PathBuf>,
    is_dir: impl Fn(&Path) -> bool,
) -> PathBuf {
    remembered
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute() && is_dir(dir))
        .or(home)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// The name Save Copy gives: "<name> copy", then "<name> copy 2" and on while `taken`.
pub(super) fn copy_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    let first = format!("{name} copy");
    if !taken(&first) {
        return first;
    }
    (2..).map(|n| format!("{name} copy {n}")).find(|candidate| !taken(candidate)).unwrap_or(first)
}

/// A group of the token list: its band's title and the tokens in it, in list order.
pub(super) struct Group {
    pub(super) name: &'static str,
    pub(super) tokens: Vec<String>,
}

/// The token list's groups: the UI tokens' groups as the catalogue declares them, the
/// metrics ("Shape"), the terminal's colors and palette, and the syntax colors that have a
/// rule of their own (the rest follow one of those).
pub(super) fn groups() -> &'static [Group] {
    static GROUPS: LazyLock<Vec<Group>> = LazyLock::new(|| {
        let mut groups: Vec<Group> = Vec::new();
        for info in UI_TOKENS.iter().chain(METRIC_TOKENS) {
            match groups.last_mut() {
                Some(group) if group.name == info.group => group.tokens.push(info.name.to_string()),
                _ => groups.push(Group { name: info.group, tokens: vec![info.name.to_string()] }),
            }
        }
        let names = |list: &[&str]| list.iter().map(|n| n.to_string()).collect();
        groups.push(Group { name: "Terminal", tokens: names(TERMINAL_TOKENS) });
        groups.push(Group { name: "Terminal ANSI", tokens: names(&ANSI_NAMES) });
        let syntax = syntax_keys()
            .map(|key| format!("syntax.{key}"))
            .filter(|node| {
                default_rule(node, Appearance::Dark).is_some_and(|rule| !rule.starts_with("syntax."))
            })
            .collect();
        groups.push(Group { name: "Syntax", tokens: syntax });
        groups
    });
    &GROUPS
}

/// The group a token is listed in.
pub(super) fn group_of(name: &str) -> Option<&'static str> {
    groups().iter().find(|g| g.tokens.iter().any(|t| t == name)).map(|g| g.name)
}

/// What the token list shows: every token, the draft's own values, or the
/// tokens that fall short of a contrast minimum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Show {
    #[default]
    All,
    Changed,
    LowContrast,
}

/// Whether the token list keeps `name` under the area filter: every token under All (`None`),
/// otherwise the ones a component of that part of the app draws with.
pub(super) fn in_area(name: &str, area: Option<Area>) -> bool {
    area.is_none_or(|area| used_in(name, area))
}

/// A band's count: the group's size in the area, also while `show` or the text list fewer of it.
pub(super) fn group_size(group: &Group, area: Option<Area>) -> usize {
    group.tokens.iter().filter(|name| in_area(name, area)).count()
}

/// The groups and tokens the list shows in `area` (`None` for All) for `show` and the filter text
/// `query` (any part of a token's or its group's name, any case): what all three keep. A group
/// nothing in it matches is left out.
pub(super) fn visible<'a>(
    show: Show,
    area: Option<Area>,
    query: &str,
    changed: &dyn Fn(&str) -> bool,
    low: &dyn Fn(&str) -> bool,
) -> Vec<(&'a Group, Vec<&'a str>)> {
    let query = query.trim().to_lowercase();
    groups()
        .iter()
        .filter_map(|group| {
            let group_hit = !query.is_empty() && group.name.to_lowercase().contains(&query);
            let tokens: Vec<&str> = group
                .tokens
                .iter()
                .map(String::as_str)
                .filter(|name| in_area(name, area))
                .filter(|name| match show {
                    Show::All => true,
                    Show::Changed => changed(name),
                    Show::LowContrast => low(name),
                })
                .filter(|name| query.is_empty() || group_hit || name.to_lowercase().contains(&query))
                .collect();
            (!tokens.is_empty()).then_some((group, tokens))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use wormhole_core::theme::tokens::{
        Area, METRIC_TOKENS, SyntaxOverride, ThemeFile, ThemeTokens, UI_TOKENS, builtin_tokens,
        parse_theme_file, resolve,
    };

    use super::{
        Show, changes, copy_name, draft_for, export_dir, export_file_name, export_json, group_of, group_size,
        groups, is_metric, number, override_of, set_override, visible,
    };

    /// Resolves a draft against the built-in themes only (no user themes dir).
    fn builtins_only(file: &ThemeFile) -> Result<ThemeTokens, String> {
        resolve(file, &|_| None)
    }

    fn tokyo_night() -> ThemeTokens {
        builtin_tokens().into_iter().find(|t| t.name == "Tokyo Night").expect("Tokyo Night is built in")
    }

    #[test]
    fn a_draft_starts_out_as_the_theme_it_extends() {
        let base = builtin_tokens().into_iter().next().expect("a built-in theme");
        let draft = draft_for(&base, builtins_only);
        assert_eq!(draft.extends.as_deref(), Some(base.name.as_str()));
        assert_eq!(draft.name, Some(format!("{} Custom", base.name)));
        assert!(draft.ui.is_empty() && draft.terminal.ansi.is_empty());
        assert_eq!(builtins_only(&draft).expect("resolves").terminal, base.terminal);
    }

    #[test]
    fn a_theme_that_cannot_be_extended_by_name_is_copied() {
        let mut base = builtin_tokens().into_iter().next().expect("a built-in theme");
        base.name = "Somewhere else".into();
        let draft = draft_for(&base, builtins_only);
        assert_eq!(draft.extends, None);
        assert_eq!(draft.terminal.foreground.as_deref(), Some(base.terminal.foreground.to_hex().as_str()));
        assert_eq!(draft.terminal.ansi.len(), 16);
        assert_eq!(builtins_only(&draft).expect("resolves").terminal, base.terminal);
    }

    #[test]
    fn a_copied_palette_keeps_cursor_text_and_selection_foreground() {
        let mut base = tokyo_night();
        base.name = "Somewhere else".into();
        base.terminal.cursor_text = Some(wormhole_core::theme::tokens::Color::from_rgb_u32(0x112233));
        base.terminal.selection_foreground = Some(wormhole_core::theme::tokens::Color::from_rgb_u32(0x445566));
        let draft = draft_for(&base, builtins_only);
        assert_eq!(draft.terminal.cursor_text.as_deref(), Some("#112233"));
        assert_eq!(draft.terminal.selection_foreground.as_deref(), Some("#445566"));
        assert_eq!(builtins_only(&draft).expect("resolves").terminal, base.terminal);
        // Left unset, they stay unset and keep following.
        base.terminal.cursor_text = None;
        base.terminal.selection_foreground = None;
        let draft = draft_for(&base, builtins_only);
        assert_eq!((&draft.terminal.cursor_text, &draft.terminal.selection_foreground), (&None, &None));
    }

    #[test]
    fn colors_are_set_read_and_reset() {
        let mut draft = ThemeFile::default();
        assert_eq!(override_of(&draft, "primary"), None);
        set_override(&mut draft, "primary", Some("#ff8800".into())).expect("sets");
        assert_eq!(override_of(&draft, "primary").as_deref(), Some("#ff8800"));
        set_override(&mut draft, "primary", None).expect("resets");
        assert_eq!(override_of(&draft, "primary"), None);
        assert!(draft.ui.is_empty());
    }

    #[test]
    fn terminal_colors_and_ansi_slots_have_their_own_homes() {
        let mut draft = ThemeFile::default();
        set_override(&mut draft, "terminal.background", Some("#101010".into())).expect("sets");
        set_override(&mut draft, "terminal.ansi.5", Some("#aa00aa".into())).expect("sets");
        assert_eq!(override_of(&draft, "terminal.background").as_deref(), Some("#101010"));
        assert_eq!(override_of(&draft, "terminal.ansi.5").as_deref(), Some("#aa00aa"));
        assert_eq!(override_of(&draft, "terminal.ansi.6"), None);
        assert!(draft.ui.is_empty(), "terminal colors are not UI tokens");
        set_override(&mut draft, "terminal.ansi.5", None).expect("resets");
        assert_eq!(override_of(&draft, "terminal.ansi.5"), None);
    }

    #[test]
    fn syntax_colors_keep_the_style_beside_them() {
        let mut draft = parse_theme_file(r#"{"syntax": {"keyword": {"bold": true}}}"#).expect("parses");
        assert_eq!(override_of(&draft, "syntax.keyword"), None);
        set_override(&mut draft, "syntax.keyword", Some("#ff00ff".into())).expect("sets");
        assert_eq!(override_of(&draft, "syntax.keyword").as_deref(), Some("#ff00ff"));
        assert_eq!(
            draft.syntax["keyword"],
            SyntaxOverride::Style { color: Some("#ff00ff".into()), bold: Some(true), italic: None }
        );
        set_override(&mut draft, "syntax.keyword", None).expect("resets");
        assert_eq!(override_of(&draft, "syntax.keyword"), None);
        assert!(draft.syntax.contains_key("keyword"), "the bold stays");
        set_override(&mut draft, "syntax.string", Some("#00ff00".into())).expect("sets");
        assert_eq!(draft.syntax["string"], SyntaxOverride::Color("#00ff00".into()));
        assert!(builtins_only(&draft).is_ok());
    }

    #[test]
    fn metrics_take_pixels_with_or_without_a_unit() {
        let mut draft = ThemeFile::default();
        assert!(is_metric("metrics.radius"));
        assert!(!is_metric("primary"));
        set_override(&mut draft, "metrics.radius", Some("6px".into())).expect("sets");
        assert_eq!(override_of(&draft, "metrics.radius").as_deref(), Some("6"));
        set_override(&mut draft, "metrics.radius", Some(" 8 ".into())).expect("sets");
        assert_eq!(override_of(&draft, "metrics.radius").as_deref(), Some("8"));
        let err = set_override(&mut draft, "metrics.radius", Some("round".into())).expect_err("not a number");
        assert!(err.contains("metrics.radius") && err.contains("round"), "{err}");
        assert!(set_override(&mut draft, "metrics.radius", Some("-2".into())).is_err(), "not negative");
        // A failed edit leaves the draft alone.
        assert_eq!(override_of(&draft, "metrics.radius").as_deref(), Some("8"));
        set_override(&mut draft, "metrics.radius", None).expect("resets");
        assert_eq!(override_of(&draft, "metrics.radius"), None);
        assert_eq!(number(6.5), "6.5");
    }

    #[test]
    fn changes_count_what_differs_from_the_start() {
        let initial = draft_for(&tokyo_night(), builtins_only);
        let mut draft = initial.clone();
        assert_eq!(changes(&initial, &draft), 0);
        set_override(&mut draft, "primary", Some("#3d59a1".into())).expect("sets");
        set_override(&mut draft, "metrics.radius_chip", Some("6".into())).expect("sets");
        set_override(&mut draft, "terminal.ansi.2", Some("#00ff00".into())).expect("sets");
        assert_eq!(changes(&initial, &draft), 3);
        // Back to where it started is no change, and the name alone isn't one.
        set_override(&mut draft, "terminal.ansi.2", None).expect("resets");
        draft.name = Some("Elsewhere".into());
        assert_eq!(changes(&initial, &draft), 2);
    }

    /// Export writes the draft itself: what it overrides, expressions as typed, and nothing it
    /// only inherits ("What Export writes").
    #[test]
    fn export_writes_only_the_overrides() {
        let mut draft = draft_for(&tokyo_night(), builtins_only);
        let typed = "readable(text.muted, sidebar.row.selected, 4.5)";
        set_override(&mut draft, "sidebar.row.selected.muted", Some(typed.into())).expect("sets");
        set_override(&mut draft, "metrics.radius_chip", Some("6".into())).expect("sets");
        let json = export_json(&draft);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(
            value,
            serde_json::json!({
                "name": "Tokyo Night Custom",
                "extends": "Tokyo Night",
                "ui": {"sidebar.row.selected.muted": typed},
                "metrics": {"radius_chip": 6},
            })
        );
        assert!(json.contains("\"radius_chip\": 6\n"), "a whole number reads 6, not 6.0: {json}");
        // It reads back as the same theme.
        let again = parse_theme_file(&json).expect("parses");
        assert_eq!(again, draft);
        assert_eq!(
            builtins_only(&again).expect("resolves"),
            builtins_only(&draft).expect("resolves"),
            "the export is the theme"
        );
    }

    #[test]
    fn names_for_a_copy_and_an_export() {
        assert_eq!(copy_name("Dusk", |_| false), "Dusk copy");
        assert_eq!(copy_name("Dusk", |n| n == "Dusk copy"), "Dusk copy 2");
        assert_eq!(copy_name("Dusk", |n| n == "Dusk copy" || n == "Dusk copy 2"), "Dusk copy 3");
        assert_eq!(export_file_name("Tokyo Night Custom"), "tokyo-night-custom.json");
        assert_eq!(export_file_name("  Dusk / 2 "), "dusk-2.json");
        assert_eq!(export_file_name("…"), "theme.json");
    }

    /// Export asks where every time, and starts in the folder it went to last.
    #[test]
    fn export_starts_in_the_last_folder_used() {
        let home = || Some(PathBuf::from("/Users/me"));
        let exists = |dir: &Path| dir == Path::new("/Users/me/Documents/themes");
        // Nothing remembered yet: the home folder.
        assert_eq!(export_dir(None, home(), exists), Path::new("/Users/me"));
        // The folder of the last export.
        assert_eq!(
            export_dir(Some("/Users/me/Documents/themes"), home(), exists),
            Path::new("/Users/me/Documents/themes")
        );
        // That folder has gone, or the record is garbage: home again, not an error.
        assert_eq!(export_dir(Some("/Volumes/gone/themes"), home(), exists), Path::new("/Users/me"));
        assert_eq!(export_dir(Some("themes"), home(), |_| true), Path::new("/Users/me"));
        assert_eq!(export_dir(Some(""), home(), |_| true), Path::new("/Users/me"));
        // No home either.
        assert_eq!(export_dir(None, None, exists), Path::new("/"));
    }

    #[test]
    fn the_list_has_every_token_once_in_its_groups() {
        let listed: Vec<&str> = groups().iter().flat_map(|g| g.tokens.iter().map(String::as_str)).collect();
        for info in UI_TOKENS.iter().chain(METRIC_TOKENS) {
            assert_eq!(listed.iter().filter(|n| **n == info.name).count(), 1, "{}", info.name);
        }
        assert_eq!(group_of("sidebar.row.selected"), Some("Chrome"));
        assert_eq!(group_of("metrics.radius_chip"), Some("Shape"));
        assert_eq!(group_of("terminal.ansi.4"), Some("Terminal ANSI"));
        let syntax = groups().iter().find(|g| g.name == "Syntax").expect("a Syntax group");
        assert!(syntax.tokens.contains(&"syntax.keyword".to_string()));
        assert!(!syntax.tokens.contains(&"syntax.keyword.control".to_string()), "it follows keyword");
        assert_eq!(groups().last().map(|g| g.name), Some("Syntax"));
    }

    #[test]
    fn the_list_filters_by_show_and_text() {
        let nothing = |_: &str| false;
        let all = visible(Show::All, None, "", &nothing, &nothing);
        assert_eq!(all.len(), groups().len());

        let changed = |name: &str| name == "primary" || name == "metrics.radius_chip";
        let shown = visible(Show::Changed, None, "", &changed, &nothing);
        let names: Vec<(&str, Vec<&str>)> = shown.iter().map(|(g, t)| (g.name, t.clone())).collect();
        assert_eq!(names, [("Primary", vec!["primary"]), ("Shape", vec!["metrics.radius_chip"])]);

        let low = |name: &str| name == "text.link";
        let shown = visible(Show::LowContrast, None, "", &nothing, &low);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].1, ["text.link"]);

        // Text matches a token's name in any case, or its whole group.
        let shown = visible(Show::All, None, "ROW.SEL", &nothing, &nothing);
        assert!(shown.iter().all(|(_, t)| t.iter().all(|n| n.contains("row.sel"))));
        assert!(shown.iter().any(|(g, _)| g.name == "Chrome"));
        let shown = visible(Show::All, None, "banners", &nothing, &nothing);
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].1.len(), 8);
        assert!(visible(Show::Changed, None, "border", &changed, &nothing).is_empty());
    }

    /// The gallery's area filter narrows the list to what that part's components draw with, and
    /// combines with Changed / Low contrast and the text (each keeps only what the others keep).
    #[test]
    fn the_list_keeps_to_the_area() {
        let nothing = |_: &str| false;
        let sidebar = visible(Show::All, Some(Area::Sidebar), "", &nothing, &nothing);
        let names: Vec<&str> = sidebar.iter().flat_map(|(_, t)| t.iter().copied()).collect();
        assert!(
            names.contains(&"sidebar.row.selected") && names.contains(&"metrics.radius_chip"),
            "{names:?}"
        );
        assert!(!names.contains(&"resize.handle.hover") && !names.contains(&"terminal.ansi.4"), "{names:?}");
        assert!(!names.contains(&"diff.added.background"), "the diff is the panel's");
        assert!(sidebar.iter().all(|(g, _)| g.name != "Terminal ANSI" && g.name != "Syntax"));
        // A band counts what the area has of its group.
        let chrome = groups().iter().find(|g| g.name == "Chrome").expect("a Chrome group");
        let listed = sidebar.iter().find(|(g, _)| g.name == "Chrome").map(|(_, t)| t.len());
        assert_eq!(listed, Some(group_size(chrome, Some(Area::Sidebar))));
        assert!(group_size(chrome, Some(Area::Sidebar)) < group_size(chrome, None));
        assert_eq!(group_size(chrome, None), chrome.tokens.len());

        let changed = |name: &str| name == "primary" || name == "resize.handle.hover";
        let shown = visible(Show::Changed, Some(Area::Sidebar), "", &changed, &nothing);
        let names: Vec<(&str, Vec<&str>)> = shown.iter().map(|(g, t)| (g.name, t.clone())).collect();
        assert_eq!(names, [("Primary", vec!["primary"])], "the resize handle is the ambient part's");
        assert!(visible(Show::All, Some(Area::Ambient), "sidebar.row", &nothing, &nothing).is_empty());
        assert_eq!(visible(Show::All, None, "", &nothing, &nothing).len(), groups().len());
    }
}
