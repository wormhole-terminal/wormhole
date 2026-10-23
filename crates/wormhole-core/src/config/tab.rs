//! The right panel's tabs, as a window saves which one is open.

use serde::{Deserialize, Serialize};

/// A tab of the window's right panel. It is stored in the window's saved layout as its lowercase
/// name (`"changes"`, `"history"`, `"notes"`, `"usage"`, `"project"`), the same strings every build
/// has written, so a layout saved by an older build opens here and one saved here opens there.
///
/// A name this build doesn't know (a tab of a newer build) reads as [`TabId::Changes`], and the rest
/// of the layout survives: an unknown name used to fail the whole layout, and the window opened with
/// the default one.
///
/// The set is closed on purpose: a `&'static str` could not be read back without interning it, and
/// the layout is read before any feature runs. Which tabs exist and which are on is the feature
/// table's business ([`crate::config::FEATURES`]); this type is only their names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TabId {
    History,
    Notes,
    Usage,
    Project,
    // Last because serde wants the catch-all variant there; the panel's order is `ALL`.
    #[default]
    #[serde(other)]
    Changes,
}

impl TabId {
    /// The tab a window falls back to when the one it had is gone (its feature went off, an agent left
    /// the screen, a layout from a newer build). Changes is an always-on feature, so it is always there.
    pub const FALLBACK: Self = Self::Changes;

    /// Every tab, in the order the panel lists them.
    pub const ALL: [TabId; 5] = [Self::Changes, Self::History, Self::Notes, Self::Usage, Self::Project];

    /// The name saved in the layout and typed to devctl's `panel-tab`.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Changes => "changes",
            Self::History => "history",
            Self::Notes => "notes",
            Self::Usage => "usage",
            Self::Project => "project",
        }
    }

    /// The tab's element id in the window, which the tests and devctl find it by.
    pub const fn element_id(self) -> &'static str {
        match self {
            Self::Changes => "tab-changes",
            Self::History => "tab-history",
            Self::Notes => "tab-notes",
            Self::Usage => "tab-usage",
            Self::Project => "tab-project",
        }
    }

    /// The tab with this name, if it is one (unlike reading a layout, which falls back to Changes).
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_saved_strings_are_the_names_older_builds_wrote() {
        let saved: Vec<String> = TabId::ALL.iter().map(|tab| serde_json::to_string(tab).unwrap()).collect();
        assert_eq!(saved, [r#""changes""#, r#""history""#, r#""notes""#, r#""usage""#, r#""project""#]);
    }

    #[test]
    fn every_tab_reads_back_from_its_key_and_from_json() {
        for tab in TabId::ALL {
            assert_eq!(TabId::from_key(tab.key()), Some(tab));
            let json = format!("\"{}\"", tab.key());
            assert_eq!(serde_json::from_str::<TabId>(&json).unwrap(), tab);
            assert_eq!(serde_json::to_string(&tab).unwrap(), json, "writes what it reads");
        }
    }

    #[test]
    fn element_ids_are_the_tab_names() {
        for tab in TabId::ALL {
            assert_eq!(tab.element_id(), format!("tab-{}", tab.key()));
        }
    }

    #[test]
    fn a_tab_of_a_newer_build_reads_as_changes() {
        assert_eq!(serde_json::from_str::<TabId>(r#""terminal-log""#).unwrap(), TabId::Changes);
        assert_eq!(serde_json::from_str::<TabId>(r#""""#).unwrap(), TabId::Changes);
        assert_eq!(TabId::from_key("terminal-log"), None, "devctl is told, a layout is not");
        assert_eq!(TabId::default(), TabId::Changes);
        assert_eq!(TabId::FALLBACK, TabId::default());
    }
}
