//! The Import page's list: the terminals that have settings on this Mac and what each would bring over
//! (`import::scan`). Core reads each source's files read-only, capped, and executes nothing; this lists
//! them. Reviewing and applying a proposal is a later step (`import::apply` over the ticks).
//!
//! The scan reads a home folder. `home` names it (a test's fixture, or the experiment's own folder);
//! `None` is this Mac's real home, which is what the product page wants and an experiment does not.

use std::path::PathBuf;

use wormhole_core::import::{self, Env, Found};
use wormhole_text::keys::settings_import_count;
use wormhole_text::{text, whole};

/// One terminal found.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsImportRow {
    /// Core's stable name for the source (`ghostty`, `iterm2`).
    pub id: String,
    /// The terminal's name as the interface writes it.
    pub app: String,
    /// "13 settings", in the catalogs' words.
    pub summary: String,
    /// How many settings it could bring over.
    pub settings: u32,
    /// How many it read that have no Wormhole setting to go in.
    pub skipped: u32,
    /// The files that were read, the settings file first.
    pub files: Vec<String>,
}

fn row(found: &Found) -> SettingsImportRow {
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    SettingsImportRow {
        id: found.source.key().to_string(),
        app: text(found.source.name_key()).to_string(),
        summary: settings_import_count(whole(found.proposal.values.len())),
        settings: count(found.proposal.values.len()),
        skipped: count(found.proposal.skipped.len()),
        files: found.files.iter().map(|file| file.to_string_lossy().into_owned()).collect(),
    }
}

pub(crate) fn rows_in(env: &Env) -> Vec<SettingsImportRow> {
    import::scan(env).iter().map(row).collect()
}

/// The terminals with settings under `home` (this Mac's real home for `None`), the newest first. Reads
/// files: call it off the main thread, and only when the page is open.
#[uniffi::export]
pub fn settings_import_rows(home: Option<String>) -> Vec<SettingsImportRow> {
    let env = match home {
        Some(home) => Some(Env::new(PathBuf::from(home))),
        None => Env::from_process(),
    };
    env.map(|env| rows_in(&env)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use wormhole_core::import::Env;

    use super::rows_in;

    #[test]
    fn a_terminal_with_a_settings_file_is_listed_with_what_it_brings() {
        let home = tempfile::tempdir().expect("temp home");
        let dir = home.path().join(".config/ghostty");
        std::fs::create_dir_all(&dir).expect("folder");
        crate::settings::tests::write_file(&dir.join("config"), "font-size = 15\nbackground-opacity = 0.9\n");

        let rows = rows_in(&Env::new(home.path().to_path_buf()));
        let ghostty = rows.iter().find(|row| row.id == "ghostty").expect("found");
        assert_eq!(ghostty.app, "Ghostty");
        assert!(ghostty.settings >= 2, "{ghostty:?}");
        assert!(ghostty.summary.contains("settings"), "{}", ghostty.summary);
        assert!(ghostty.files[0].ends_with("ghostty/config"));
    }

    #[test]
    fn an_empty_home_lists_nothing() {
        let home = tempfile::tempdir().expect("temp home");
        assert!(rows_in(&Env::new(home.path().to_path_buf())).is_empty());
    }
}
