//! Reading and writing a theme file's own values by token name: the one place that knows where
//! a token lives in the file (a UI token, a terminal slot, a palette index, a syntax colour, a
//! metric), and what differs between two files. Pure, so it is tested without a window.

use std::collections::{BTreeMap, BTreeSet};

use wormhole_core::theme::tokens::{METRIC_TOKENS, SyntaxOverride, TerminalFile, ThemeFile, ansi_index};

/// The terminal colours a file names by a plain key, with their slots: one list for reading,
/// writing and listing them, so a colour added to the file's terminal block is added here once.
macro_rules! terminal_keys {
    ($($key:literal => $field:ident),* $(,)?) => {
        const TERMINAL_KEYS: &[&str] = &[$($key),*];

        fn terminal_get<'a>(terminal: &'a TerminalFile, key: &str) -> Option<&'a Option<String>> {
            match key { $($key => Some(&terminal.$field),)* _ => None }
        }

        fn terminal_slot<'a>(terminal: &'a mut TerminalFile, key: &str) -> Option<&'a mut Option<String>> {
            match key { $($key => Some(&mut terminal.$field),)* _ => None }
        }
    };
}

terminal_keys! {
    "foreground" => foreground,
    "background" => background,
    "cursor" => cursor,
    "cursor_text" => cursor_text,
    "selection_background" => selection_background,
    "selection_foreground" => selection_foreground,
}

/// Whether `name` is a metric (`metrics.radius_chip`) rather than a color.
pub(in crate::studio) fn is_metric(name: &str) -> bool {
    name.starts_with("metrics.")
}

/// A metric as the token editor shows and takes it: pixels without a unit.
pub(in crate::studio) fn number(value: f32) -> String {
    format!("{value}")
}

/// The file's own value for a token (not the extended or derived one).
pub(in crate::studio) fn override_of(file: &ThemeFile, name: &str) -> Option<String> {
    if is_metric(name) {
        return file.metrics.get(name).map(number);
    }
    if let Some(key) = name.strip_prefix("terminal.") {
        if let Some(index) = key.strip_prefix("ansi.") {
            // A hand-written file may call the slot by name (`blue`, `bright_white`).
            let slot = index.parse::<usize>().ok().filter(|i| *i < 16)?;
            return file
                .terminal
                .ansi
                .iter()
                .find(|(k, _)| ansi_index(k) == Some(slot))
                .map(|(_, v)| v.clone());
        }
        return terminal_get(&file.terminal, key).cloned().flatten();
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        return match file.syntax.get(key)? {
            SyntaxOverride::Color(color) => Some(color.clone()),
            SyntaxOverride::Style { color, .. } => color.clone(),
        };
    }
    file.ui.get(name).cloned()
}

/// Set (or with `None`, reset) a token in the file's own values.
pub(in crate::studio) fn set_override(
    file: &mut ThemeFile,
    name: &str,
    value: Option<String>,
) -> Result<(), String> {
    if is_metric(name) {
        let value = match value {
            Some(v) => Some(
                v.trim_end_matches("px")
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| wormhole_text::keys::studio_not_pixels(name, &v))?,
            ),
            None => None,
        };
        file.metrics.set(name, value);
        return Ok(());
    }
    if let Some(key) = name.strip_prefix("terminal.") {
        if let Some(index) = key.strip_prefix("ansi.") {
            let Some(slot) = index.parse::<usize>().ok().filter(|i| *i < 16) else {
                return Err(format!("{name}: no such palette slot")); // text: error (logged, not shown)
            };
            // Under whatever name the file gives it: two keys for one slot would fight.
            file.terminal.ansi.retain(|k, _| ansi_index(k) != Some(slot));
            if let Some(v) = value {
                file.terminal.ansi.insert(slot.to_string(), v);
            }
            return Ok(());
        }
        if let Some(slot) = terminal_slot(&mut file.terminal, key) {
            *slot = value;
            return Ok(());
        }
    }
    if let Some(key) = name.strip_prefix("syntax.") {
        set_syntax_color(file, key, value);
        return Ok(());
    }
    match value {
        Some(v) => file.ui.insert(name.to_string(), v),
        None => file.ui.remove(name),
    };
    Ok(())
}

/// A syntax key's color, keeping the bold and italic a theme may have set next to it.
fn set_syntax_color(file: &mut ThemeFile, key: &str, value: Option<String>) {
    let style = match file.syntax.remove(key) {
        Some(SyntaxOverride::Style { bold, italic, .. }) if bold.is_some() || italic.is_some() => {
            Some(SyntaxOverride::Style { color: value, bold, italic })
        }
        _ => value.map(SyntaxOverride::Color),
    };
    if let Some(style) = style {
        file.syntax.insert(key.to_string(), style);
    }
}

/// Every value the file sets itself, by token name (bold and italic as their own entries).
fn own_values(file: &ThemeFile) -> BTreeMap<String, String> {
    let mut values: BTreeMap<String, String> = file.ui.clone();
    for key in TERMINAL_KEYS {
        if let Some(value) = terminal_get(&file.terminal, key).cloned().flatten() {
            values.insert(format!("terminal.{key}"), value);
        }
    }
    for (key, value) in &file.terminal.ansi {
        let slot = ansi_index(key).map_or_else(|| key.clone(), |i| i.to_string());
        values.insert(format!("terminal.ansi.{slot}"), value.clone());
    }
    for info in METRIC_TOKENS {
        if let Some(value) = file.metrics.get(info.name) {
            values.insert(info.name.to_string(), number(value));
        }
    }
    for (key, style) in &file.syntax {
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

/// The tokens `file` sets differently from `before` (added, changed or removed), in the shown
/// appearances too; a bold or italic is named by its syntax token.
pub(in crate::studio) fn changed_tokens(before: &ThemeFile, file: &ThemeFile) -> BTreeSet<String> {
    let (before, after) = (all_values(before), all_values(file));
    let names: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    names
        .into_iter()
        .filter(|name| before.get(*name) != after.get(*name))
        .map(|name| {
            let token = name.rsplit_once('/').map_or(name.as_str(), |(_, t)| t);
            token.split(':').next().unwrap_or(token).to_string()
        })
        .collect()
}

/// Every value of the top and of each appearance, the appearance's as `name/token`.
fn all_values(file: &ThemeFile) -> BTreeMap<String, String> {
    let mut values = own_values(file);
    for variant in &file.variants {
        let prefix = format!("{}/", variant.name);
        values.extend(own_values(&variant.to_file()).into_iter().map(|(k, v)| (format!("{prefix}{k}"), v)));
    }
    values
}

/// How many values `file` sets differently from `saved` (added, changed or removed): what
/// closing without saving would lose.
pub(in crate::studio) fn changes(saved: &ThemeFile, file: &ThemeFile) -> usize {
    let with_variants = |file: &ThemeFile| {
        let mut values = own_values(file);
        for variant in &file.variants {
            let prefix = format!("{}/", variant.name);
            values
                .extend(own_values(&variant.to_file()).into_iter().map(|(k, v)| (format!("{prefix}{k}"), v)));
        }
        values
    };
    let (before, after) = (with_variants(saved), with_variants(file));
    let names: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    names.into_iter().filter(|name| before.get(*name) != after.get(*name)).count()
}

#[cfg(test)]
mod tests {
    use wormhole_core::theme::tokens::{
        SyntaxOverride, ThemeFile, ThemeTokens, builtin_tokens, parse_theme_file, resolve,
    };

    use super::{changes, is_metric, number, override_of, set_override};
    use crate::studio::draft::draft_for;

    fn builtins_only(file: &ThemeFile) -> Result<ThemeTokens, String> {
        resolve(file, &|_| None)
    }

    fn tokyo_night() -> ThemeTokens {
        builtin_tokens().into_iter().find(|t| t.name == "Tokyo Night").expect("Tokyo Night is built in")
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
    fn an_ansi_slot_named_by_color_is_read_and_replaced_not_duplicated() {
        let mut draft =
            parse_theme_file(r##"{"terminal": {"ansi": {"blue": "#0000aa", "bright_white": "#ffffff"}}}"##)
                .expect("parses");
        assert_eq!(override_of(&draft, "terminal.ansi.4").as_deref(), Some("#0000aa"));
        assert_eq!(override_of(&draft, "terminal.ansi.15").as_deref(), Some("#ffffff"));
        set_override(&mut draft, "terminal.ansi.4", Some("#1111ff".into())).expect("sets");
        assert_eq!(draft.terminal.ansi.len(), 2, "one key for the slot: {:?}", draft.terminal.ansi);
        assert_eq!(override_of(&draft, "terminal.ansi.4").as_deref(), Some("#1111ff"));
        set_override(&mut draft, "terminal.ansi.15", None).expect("resets");
        assert_eq!(override_of(&draft, "terminal.ansi.15"), None);
        assert_eq!(draft.terminal.ansi.len(), 1);
        // Counted under the slot, whatever the file calls it.
        let saved = parse_theme_file(r##"{"terminal": {"ansi": {"blue": "#0000aa"}}}"##).expect("parses");
        let mut now = saved.clone();
        set_override(&mut now, "terminal.ansi.4", Some("#0000aa".into())).expect("sets");
        assert_eq!(changes(&saved, &now), 0);
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
}
