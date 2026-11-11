//! Theme resolution across the seam: `wormhole-core`'s loader and token rules, asked for one theme at
//! one look, answered as JSON in the shape `themes.json` has (`theme_dump`). Read-only: it never
//! writes the themes folder or any setting.

use std::path::Path;

use crate::error::FfiError;
use crate::theme_resolve::{self, Request};

/// One theme resolved: `theme` is a name, slug or path (what `settings.theme` holds), `themes_dir` the
/// folder of user theme files, `primary` a `#rrggbb` or empty, `branch` `""`, `"primary"` or `#rrggbb`.
#[uniffi::export]
pub fn resolve_theme(
    theme: String,
    themes_dir: Option<String>,
    primary: String,
    branch: String,
    increase_contrast: bool,
    opacity: f32,
    blur: bool,
) -> Result<String, FfiError> {
    let dir = themes_dir.as_deref().map(Path::new);
    theme_resolve::resolve(&Request {
        theme: &theme,
        project: None,
        themes_dir: dir,
        primary: &primary,
        branch: &branch,
        increase_contrast,
        opacity,
        blur,
    })
    .map_err(|reason| FfiError::Theme { reason })
}

/// The user's theme files in `themes_dir`: a JSON array of `{id, name, appearance, error}`.
#[uniffi::export]
pub fn list_user_themes(themes_dir: String) -> String {
    theme_resolve::list(Path::new(&themes_dir))
}

/// A project's `theme.json` text resolved as the theme in effect (`ProjectService::theme_overlay`), with the
/// person's primary and branch colours over it and the window's look, as `resolve_theme` answers.
#[uniffi::export]
pub fn resolve_theme_json(
    json: String,
    themes_dir: Option<String>,
    primary: String,
    branch: String,
    increase_contrast: bool,
    opacity: f32,
    blur: bool,
) -> Result<String, FfiError> {
    let dir = themes_dir.as_deref().map(Path::new);
    theme_resolve::resolve(&Request {
        theme: "",
        project: Some(&json),
        themes_dir: dir,
        primary: &primary,
        branch: &branch,
        increase_contrast,
        opacity,
        blur,
    })
    .map_err(|reason| FfiError::Theme { reason })
}

/// A string that changes when a theme file a pick may come from changes: any file of the themes folder (an
/// `extends` may name one) or a pick that is a path. The same stamp means resolving again gives the same tokens
/// (core's `ThemeStamp`), so the app re-resolves only when it moves.
#[uniffi::export]
pub fn theme_files_stamp(themes_dir: String, picks: Vec<String>) -> String {
    let files = wormhole_core::theme::registry::fingerprint(Path::new(&themes_dir));
    let picked: Vec<_> =
        picks.iter().map(|pick| std::fs::metadata(pick).ok().map(|m| (m.len(), m.modified().ok()))).collect();
    format!("{files:?}{picked:?}")
}
