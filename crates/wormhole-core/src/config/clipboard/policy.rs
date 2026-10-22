//! The two settings: `clipboard_read` and `clipboard_write` in `config.json`.

use serde::{Deserialize, Serialize};

/// What a program asking to read the clipboard gets (`clipboard_read`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardRead {
    /// A bar over the terminal asks; nothing is given before the person says so.
    #[default]
    Ask,
    /// Every program in every terminal may read it.
    Allow,
    /// No program gets it, and nothing is asked.
    Deny,
}

impl ClipboardRead {
    /// In the order the settings page lists them.
    pub const ALL: [ClipboardRead; 3] = [ClipboardRead::Ask, ClipboardRead::Allow, ClipboardRead::Deny];
}

/// Whether a program may set the clipboard (`clipboard_write`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardWrite {
    /// It may, and the terminal says so with a short notice.
    #[default]
    Allow,
    /// It may not.
    Deny,
}

impl ClipboardWrite {
    /// In the order the settings page lists them.
    pub const ALL: [ClipboardWrite; 2] = [ClipboardWrite::Allow, ClipboardWrite::Deny];
}

#[cfg(test)]
mod tests {
    use super::{ClipboardRead, ClipboardWrite};
    use crate::config::{APP_SETTINGS, PROJECT_SETTINGS, Settings};
    use crate::jsonc;

    #[test]
    fn reading_asks_and_writing_is_allowed_until_the_person_says_otherwise() {
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert_eq!(unset.clipboard_read, ClipboardRead::Ask);
        assert_eq!(unset.clipboard_write, ClipboardWrite::Allow);
    }

    #[test]
    fn the_settings_read_their_names() {
        let read = |json: &str| -> Settings { jsonc::parse(json).expect("valid jsonc") };
        let set = read(r#"{ "clipboard_read": "deny", "clipboard_write": "deny" }"#);
        assert_eq!((set.clipboard_read, set.clipboard_write), (ClipboardRead::Deny, ClipboardWrite::Deny));
        assert_eq!(read(r#"{ "clipboard_read": "allow" }"#).clipboard_read, ClipboardRead::Allow);
        assert!(jsonc::parse::<Settings>(r#"{ "clipboard_read": "sometimes" }"#).is_err());
        let json = serde_json::to_value(set).expect("serializes");
        assert_eq!(json["clipboard_read"], "deny");
        assert_eq!(json["clipboard_write"], "deny");
    }

    #[test]
    fn a_project_cannot_open_the_clipboard() {
        for key in ["clipboard_read", "clipboard_write"] {
            assert!(APP_SETTINGS.contains(&key) && !PROJECT_SETTINGS.contains(&key), "{key}");
        }
    }
}
