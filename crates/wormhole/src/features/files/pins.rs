//! Pinned folders: the list in `config.json` (`files_pinned`) that the path menu shows. They belong to
//! the window's own profile, as everything a window reads does: another profile's window shows and
//! changes its own list. The settings store saves it.

use std::path::{Path, PathBuf};

use gpui_kit::*;

use super::Files;
use crate::settings::{settings_in, update_in};

impl Files {
    /// The pinned folders of this window's profile, in the order they were pinned.
    pub(super) fn pinned(&self, cx: &App) -> Vec<PathBuf> {
        settings_in(&self.profile, cx).pinned_folders()
    }

    /// Pins `folder`, or takes it off the pins.
    pub(super) fn set_pinned(&mut self, folder: &Path, pinned: bool, cx: &mut Context<Self>) {
        if settings_in(&self.profile, cx).is_pinned(folder) == pinned {
            return;
        }
        update_in(&self.profile, cx, |settings| {
            settings.set_pinned(folder, pinned);
        });
        cx.notify();
    }

    /// A folder was renamed: pins of it, and of folders below it, name its new place.
    pub(super) fn move_pins(&mut self, from: &Path, to: &Path, cx: &mut Context<Self>) {
        if self.pinned(cx).iter().any(|pin| pin.starts_with(from)) {
            update_in(&self.profile, cx, |settings| {
                settings.move_pinned(from, to);
            });
        }
    }
}
