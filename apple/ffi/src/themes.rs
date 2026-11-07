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
