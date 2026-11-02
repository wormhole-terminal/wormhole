//! The Changed page's "Default: …" line says the default in the words of the row's own control: a menu's
//! option, a switch's On, "10,000 lines", "80 %", key caps, never a bare number or a name from config.json.
//! The words of a menu or a switch are the control's by construction, so this walks the rows that draw
//! their own control (`Control::Custom`, `Recorder`, `Block`) that write a key.

use wormhole_core::config::settings_index::{ChangedRow, SectionId, Values};

use super::changed::default_words;
use super::registry::{Control, control};
use crate::settings::Settings;

/// Rows whose control shows the stored text itself, so the default is that text: a theme or an icon theme by
/// name, the profiles `config.json` lists (none), the folder it names (none), the flags it sets (none).
const SHOWN_AS_WRITTEN: &[&str] = &["theme", "icon-theme", "shells", "worktree-base", "feature-flags"];

/// Rows whose control shows the number itself, with no unit: the line height slider reads "1.30".
const NUMBER_SHOWN: &[&str] = &["line-height"];

/// The default line of every row that has a control of its own and writes a key.
fn lines() -> Vec<(&'static str, String)> {
    let values = Values::of(&Settings::default());
    let mut lines = Vec::new();
    for section in SectionId::ALL {
        for (group, entry) in section.entries() {
            let Some(setting) = control(entry.id) else { continue };
            if entry.keys.is_empty() || matches!(setting.control, Control::Menu(_) | Control::Toggle { .. }) {
                continue;
            }
            let row = ChangedRow { section, group, entry };
            let words = default_words(&row, setting, &values);
            if !SHOWN_AS_WRITTEN.contains(&entry.id) {
                assert!(
                    setting.default_words.is_some() || matches!(setting.control, Control::Recorder(_)),
                    "{} draws its own control: say its default in that control's words (`Setting::default_words`), \
                     not {words:?}",
                    entry.id
                );
            }
            lines.push((entry.id, words));
        }
    }
    lines
}

#[test]
fn a_row_with_its_own_control_says_its_default_in_that_controls_words() {
    let lines = lines();
    let said = |id: &str| lines.iter().find(|(row, _)| *row == id).map(|(_, words)| words.as_str());
    assert_eq!(said("scrollback"), Some("10,000 lines"), "as its menu says it");
    assert_eq!(said("opacity"), Some("80 %"));
    assert_eq!(said("line-height"), Some("1.30"));
    assert_eq!(said("density"), Some("Regular"));
    assert_eq!(said("font-terminal"), Some("JetBrains Mono, 14 pt"));
    assert_eq!(said("font-code"), Some("Same as terminal, 13 pt"));
    assert_eq!(said("font-interface"), Some("System font, 13 pt"));
    assert_eq!(said("jump-waiting-key"), Some("⌥⌘J"), "key caps, not alt-cmd-j");
}

#[test]
fn no_default_line_is_a_bare_number_or_a_config_string() {
    for (id, words) in lines() {
        // The login shell is named as the Mac names it ("zsh"), which is what the menu shows.
        if id == "default-shell" {
            continue;
        }
        for part in words.split(", ") {
            assert!(
                NUMBER_SHOWN.contains(&id) || part.parse::<f64>().is_err(),
                "{id}: {words:?} has a bare number ({part:?})"
            );
            let identifier = part.chars().any(|c| c.is_ascii_lowercase())
                && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.".contains(c));
            assert!(!identifier || part == "none", "{id}: {words:?} reads like a config string ({part:?})");
        }
    }
}
