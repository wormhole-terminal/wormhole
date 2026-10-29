//! The key names of a version 1 file, and what they were in version 0.
//!
//! One rule for every name (decision th-13): a dot is a level, an underscore joins the words
//! of one name. The names inside the resolver and in a version 0 file are the old ones
//! (`terminal.cursor_text`); this table is the one place both are written down. The reader
//! and the writer of version 1, the check that turns an old name into a hint, and the docs
//! (`themes.md`) are all made from it.

/// A key whose name changed in version 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyRename {
    /// The block it sits in: `terminal` or `metrics`.
    pub block: &'static str,
    /// Its name in version 0 (and inside the resolver).
    pub old: &'static str,
    /// Its name in version 1.
    pub new: &'static str,
}

const fn rename(block: &'static str, old: &'static str, new: &'static str) -> KeyRename {
    KeyRename { block, old, new }
}

/// Every key version 1 spells differently. Keys not listed (`terminal.background`, `radius`,
/// `spacing`, every `ui` token, every syntax key) are unchanged. The `terminal.ansi` object
/// of version 0 becomes `ansi.0` ... `ansi.15` keys (see [`ANSI_RULE`]).
pub const KEY_TABLE: &[KeyRename] = &[
    rename("terminal", "cursor_text", "cursor.text"),
    rename("terminal", "selection_background", "selection.background"),
    rename("terminal", "selection_foreground", "selection.foreground"),
    rename("metrics", "radius_sm", "radius.sm"),
    rename("metrics", "radius_chip", "radius.chip"),
    rename("metrics", "radius_lg", "radius.lg"),
    rename("metrics", "border_width", "border.width"),
    rename("metrics", "divider_width", "divider.width"),
    rename("metrics", "focus_ring_width", "focus_ring.width"),
    rename("metrics", "ui_font_family", "ui_font.family"),
    rename("metrics", "ui_font_size", "ui_font.size"),
];

/// How the nested `ansi` object of version 0 is spelled in version 1.
pub const ANSI_RULE: &str = r#""ansi": { "4": ... } or { "blue": ... }  becomes  "ansi.4": ..."#;

/// Terminal keys of version 1 in the order a file lists them (the 16 `ansi.N` follow).
pub(super) const TERMINAL_ORDER: &[&str] =
    &["background", "foreground", "cursor", "cursor.text", "selection.background", "selection.foreground"];

/// Metric keys of version 1 in the order a file lists them.
pub(super) const METRIC_ORDER: &[&str] = &[
    "radius",
    "radius.sm",
    "radius.chip",
    "radius.lg",
    "spacing",
    "border.width",
    "divider.width",
    "focus_ring.width",
    "ui_font.family",
    "ui_font.size",
];

/// The version 1 key of `old` in `block`, if it changed.
pub fn new_key(block: &str, old: &str) -> Option<&'static str> {
    KEY_TABLE.iter().find(|r| r.block == block && r.old == old).map(|r| r.new)
}

/// The version 0 key of `new` in `block`, if it changed.
pub fn old_key(block: &str, new: &str) -> Option<&'static str> {
    KEY_TABLE.iter().find(|r| r.block == block && r.new == new).map(|r| r.old)
}

/// Every terminal key a version 1 file may use, `ansi.0` to `ansi.15` included.
pub(super) fn terminal_keys() -> Vec<String> {
    TERMINAL_ORDER.iter().map(|k| (*k).to_string()).chain((0..16).map(|i| format!("ansi.{i}"))).collect()
}

/// The resolver's name of the metric a version 1 file calls `new`.
pub(super) fn metric_internal(new: &str) -> Option<&'static str> {
    METRIC_ORDER.iter().find(|k| **k == new).map(|k| old_key("metrics", k).unwrap_or(k))
}

/// The version 1 name of a color node the resolver calls `old` (`terminal.cursor_text`).
pub(super) fn new_node(old: &str) -> Option<String> {
    let key = old.strip_prefix("terminal.")?;
    new_key("terminal", key).map(|new| format!("terminal.{new}"))
}

/// The resolver's name for a version 1 color node (`terminal.cursor.text`).
pub(super) fn old_node(new: &str) -> Option<String> {
    let key = new.strip_prefix("terminal.")?;
    old_key("terminal", key).map(|old| format!("terminal.{old}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_follows_its_own_rule() {
        for r in KEY_TABLE {
            assert!(!r.old.contains('.'), "{r:?}: old names have no levels");
            assert_eq!(r.new.replace('.', "_"), r.old, "{r:?}: a level was a word joiner");
            assert!(r.new.contains('.'), "{r:?}");
        }
        let mut news: Vec<_> = KEY_TABLE.iter().map(|r| (r.block, r.new)).collect();
        news.sort_unstable();
        news.dedup();
        assert_eq!(news.len(), KEY_TABLE.len());
    }

    #[test]
    fn nodes_map_both_ways() {
        assert_eq!(new_node("terminal.cursor_text").as_deref(), Some("terminal.cursor.text"));
        assert_eq!(
            old_node("terminal.selection.background").as_deref(),
            Some("terminal.selection_background")
        );
        assert_eq!(new_node("terminal.background"), None);
        assert_eq!(new_node("primary"), None);
        assert_eq!(new_key("metrics", "focus_ring_width"), Some("focus_ring.width"));
        assert_eq!(old_key("metrics", "ui_font.size"), Some("ui_font_size"));
    }
}
