//! Which setting each row of the native window is. The window's catalog (`macos/settings/.../Pages.swift`) keys a
//! row `"<page>:<group>:<row>"`; this table says which entry of the settings index (the id the GPUI window and the
//! palette use) it stands for. A row not listed is not wired: the window shows it and keeps what is set in it
//! until it closes, but nothing is written. The table is `rows.txt`, read once.

use std::sync::LazyLock;

/// (the window's row key, the settings index's entry id), in the window's order.
fn rows() -> &'static [(&'static str, &'static str)] {
    static ROWS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
        include_str!("rows.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split_once(' '))
            .collect()
    });
    &ROWS
}

pub(super) fn all() -> &'static [(&'static str, &'static str)] {
    rows()
}

/// The index id of a row.
pub(super) fn id_of(key: &str) -> Option<&'static str> {
    rows().iter().find(|(row, _)| *row == key).map(|(_, id)| *id)
}

/// The window's row for a sub-value: the row (before `#`, or before the name of a checkbox) a key belongs to.
pub(super) fn base(key: &str) -> &str {
    if let Some((row, _)) = key.split_once('#') {
        return row;
    }
    // "Appearance:2:3Settings": the third number's digits end the row.
    let mut parts = key.splitn(3, ':');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(page), Some(group), Some(rest)) => {
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            &key[..page.len() + group.len() + 2 + digits]
        }
        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_names_a_setting_that_exists_and_once() {
        let mut keys: Vec<_> = rows().iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), rows().len(), "a row key twice");
        for (key, id) in rows() {
            assert!(wormhole_core::config::settings_index::entry(id).is_some(), "{key}: no setting {id}");
        }
    }

    #[test]
    fn a_sub_value_belongs_to_its_row() {
        assert_eq!(base("Fonts:0:0#size"), "Fonts:0:0");
        assert_eq!(base("Appearance:2:3Theme Studio"), "Appearance:2:3");
        assert_eq!(base("Appearance:2:3"), "Appearance:2:3");
        assert_eq!(base("Terminal:0:12"), "Terminal:0:12");
    }
}
