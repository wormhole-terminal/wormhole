//! Pinned folders: the list in `config.json` (`files_pinned`) that the path menu shows. One list for
//! every window, kept by the settings store, which saves it.

use std::path::{Path, PathBuf};

use gpui_kit::*;

use super::Files;
use crate::settings::{settings, update_settings};

impl Files {
    /// The pinned folders, in the order they were pinned.
    pub(super) fn pinned(cx: &App) -> Vec<PathBuf> {
        settings(cx).pinned_folders()
    }

    /// Pins `folder`, or takes it off the pins.
    pub(super) fn set_pinned(&mut self, folder: &Path, pinned: bool, cx: &mut Context<Self>) {
        if update_pins(folder, pinned, cx) {
            cx.notify();
        }
    }
}

/// Changes the pins and tells every window; whether they changed.
fn update_pins(folder: &Path, pinned: bool, cx: &mut App) -> bool {
    if settings(cx).is_pinned(folder) == pinned {
        return false;
    }
    update_settings(cx, |settings| {
        settings.set_pinned(folder, pinned);
    });
    true
}
