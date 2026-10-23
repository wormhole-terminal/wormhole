//! The colour a folder wears, kept by name or hex under its path in `folder_colors`.

use std::path::{Path, PathBuf};

use super::Settings;
use crate::project::ProjectColorChoice;
use crate::theme::folder_color::FolderColor;

impl Settings {
    /// The colour `folder` wears, if it has one this build knows: a name, or a hex colour the
    /// person picked.
    pub fn folder_color_choice(&self, folder: &Path) -> Option<ProjectColorChoice> {
        ProjectColorChoice::parse(self.folder_colors.get(&folder_color_key(folder))?).ok()
    }

    /// [`Self::folder_color_choice`], for a name only.
    pub fn folder_color(&self, folder: &Path) -> Option<FolderColor> {
        self.folder_color_choice(folder)?.named()
    }

    /// Colour `folder`, or (`None`) take its colour away. Returns whether anything changed.
    pub fn set_folder_color_choice(&mut self, folder: &Path, color: Option<ProjectColorChoice>) -> bool {
        let key = folder_color_key(folder);
        match color {
            Some(color) => {
                let value = color.to_value();
                self.folder_colors.insert(key, value.clone()).as_deref() != Some(value.as_str())
            }
            None => self.folder_colors.remove(&key).is_some(),
        }
    }

    /// [`Self::set_folder_color_choice`], for a name.
    pub fn set_folder_color(&mut self, folder: &Path, color: Option<FolderColor>) -> bool {
        self.set_folder_color_choice(folder, color.map(ProjectColorChoice::Named))
    }
}

/// How a folder is named in `folder_colors`: its path without a trailing slash or `.` parts, so
/// the terminal's folder and one typed by hand find the same entry.
pub(super) fn folder_color_key(folder: &Path) -> String {
    folder.components().collect::<PathBuf>().to_string_lossy().into_owned()
}
