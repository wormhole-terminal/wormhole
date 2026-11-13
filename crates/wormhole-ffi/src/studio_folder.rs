//! The Theme Studio's themes folder through core's rules: which files are themes, a capped read, a save that keeps
//! the file it replaces as a saved version first (`theme::saved_versions`, thinned by `keep_policy`), and the saved
//! versions of a theme. The same functions the GPUI Studio's `themes.rs` is written over; the Swift side keeps no
//! copy of the thinning, the naming or the caps.

use std::path::{Path, PathBuf};

use wormhole_core::storage::files::{self, LinkPolicy};
use wormhole_core::theme::registry::{UserThemes, fingerprint};
use wormhole_core::theme::saved_versions::{SavedVersion, keep_version_now, list_versions, read_version};
use wormhole_core::theme::tokens::{MAX_THEME_BYTES, read_theme_file, slug};

use crate::error::FfiError;

fn theme_error(reason: String) -> FfiError {
    FfiError::Theme { reason }
}

/// One earlier save of a theme.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StudioSavedVersion {
    pub path: String,
    /// Seconds since 1970 (UTC).
    pub saved: i64,
    /// The hand-written original: never thinned out.
    pub pinned: bool,
}

fn is_theme_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case("json") || e.eq_ignore_ascii_case("jsonc"))
}

/// The theme files of the folder (`.json` and `.jsonc`, hidden ones skipped), sorted by path. Lists and stats only.
#[uniffi::export]
pub fn studio_folder_files(themes_dir: String) -> Vec<String> {
    fingerprint(Path::new(&themes_dir))
        .into_iter()
        .map(|(path, _, _)| path)
        .filter(|path| is_theme_file(path))
        .map(|path| path.to_string_lossy().into_owned())
        .collect()
}

/// A theme file's text, read at core's cap for a theme.
#[uniffi::export]
pub fn studio_folder_read(path: String) -> Result<String, FfiError> {
    files::read_to_string_capped(Path::new(&path), MAX_THEME_BYTES as u64, LinkPolicy::Follow)
        .map_err(|e| theme_error(e.to_string()))
}

/// The id the saves of the theme at `path` are kept under: its slug, else the file's own id, else one made from its name.
fn version_id(themes: &Path, path: &Path, text: &str) -> String {
    let own = UserThemes::read(themes)
        .themes
        .iter()
        .find(|t| t.path == path && t.family.is_none())
        .map(|t| t.slug.clone());
    own.or_else(|| {
        let file = read_theme_file(text).ok()?;
        Some(file.id.clone().unwrap_or_else(|| slug::derive_id(file.name.as_deref().unwrap_or("theme"))))
    })
    .unwrap_or_else(|| slug::derive_id("theme"))
}

/// Writes the theme atomically at `path`. The file it replaces is kept as a saved version first; a version that
/// cannot be kept is logged and does not stop the save, the person's edit matters more than the history.
#[uniffi::export]
pub fn studio_folder_write(themes_dir: String, path: String, text: String) -> Result<(), FfiError> {
    let (themes, path) = (Path::new(&themes_dir), PathBuf::from(path));
    files::ensure_private_dir(themes).map_err(|e| theme_error(e.to_string()))?;
    if let Ok(old) = files::read_to_string_capped(&path, MAX_THEME_BYTES as u64, LinkPolicy::Follow) {
        if let Err(err) = keep_version_now(themes, &version_id(themes, &path, &old), &old) {
            tracing::warn!("keeping the earlier {}: {err}", path.display());
        }
    }
    files::write_atomic(&path, text.as_bytes(), LinkPolicy::Follow).map_err(|e| theme_error(e.to_string()))
}

/// The saved versions of the theme at `path`, newest first.
#[uniffi::export]
pub fn studio_folder_versions(themes_dir: String, path: String) -> Vec<StudioSavedVersion> {
    let themes = Path::new(&themes_dir);
    let Ok(text) = files::read_to_string_capped(Path::new(&path), MAX_THEME_BYTES as u64, LinkPolicy::Follow)
    else {
        return Vec::new();
    };
    list_versions(themes, &version_id(themes, Path::new(&path), &text))
        .into_iter()
        .map(|v| StudioSavedVersion {
            path: v.path.to_string_lossy().into_owned(),
            saved: v.saved,
            pinned: v.pinned,
        })
        .collect()
}

/// The text of a saved version (capped like a theme, inside its own folder).
#[uniffi::export]
pub fn studio_folder_read_version(path: String) -> Result<String, FfiError> {
    read_version(&SavedVersion { path: PathBuf::from(path), saved: 0, pinned: false }).map_err(theme_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_keeps_what_it_replaced_and_hidden_files_are_not_themes() {
        let dir = tempfile::tempdir().expect("scratch");
        let themes = dir.path().to_string_lossy().into_owned();
        let file = dir.path().join("Dusk.json").to_string_lossy().into_owned();
        studio_folder_write(themes.clone(), file.clone(), "{ \"name\": \"Dusk\" }".into()).expect("first");
        studio_folder_write(themes.clone(), file.clone(), "{ \"name\": \"Dusk 2\" }".into()).expect("second");
        assert_eq!(studio_folder_files(themes.clone()), vec![file.clone()]);
        let versions = studio_folder_versions(themes, file.clone());
        assert_eq!(versions.len(), 1, "the first text was kept once");
        assert!(versions[0].pinned, "a hand-written first file is the pinned original");
        assert_eq!(
            studio_folder_read_version(versions[0].path.clone()).expect("reads"),
            "{ \"name\": \"Dusk\" }"
        );
        assert_eq!(studio_folder_read(file).expect("reads"), "{ \"name\": \"Dusk 2\" }");
    }
}
