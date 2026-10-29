//! The first save of a theme in the older format writes the new one: the theme its `extends` chain
//! leads to is copied into the file's `base`, its own values stay on top, and it gets an `id`.
//! Nothing on screen may change, so the converted file is resolved next to the original first and
//! nothing is written unless the two come out the same.

use std::path::Path;

use wormhole_core::theme::tokens::ThemeFile;

use super::StudioWindow;
use super::themes::Themes;

/// The file a save writes.
pub(super) struct ToWrite {
    pub(super) file: ThemeFile,
    /// It was in the older format and is converted by this save.
    pub(super) converted: bool,
}

/// `file` converted to the new format with `id`, if it resolves to the very same theme afterwards.
pub(super) fn converted_safely(themes: &Themes, file: &ThemeFile, id: &str) -> Result<ThemeFile, String> {
    let converted = (themes.convert)(file, id)?;
    let before = (themes.resolve)(file)?;
    let after = (themes.resolve)(&converted)?;
    if before != after {
        return Err("the converted theme does not look the same".to_string()); // text: error (logged, not shown)
    }
    Ok(converted)
}

impl StudioWindow {
    /// What saving writes over the theme's file at `path`: the file as edited, converted first when it
    /// is still in the older format. The error is what to tell the person.
    pub(super) fn file_to_write(&self, path: &Path) -> Result<ToWrite, String> {
        if self.doc.file.version.is_some() {
            return Ok(ToWrite { file: self.doc.file.clone(), converted: false });
        }
        let id = (self.themes.new_id)(self.doc.name(), Some(path));
        match converted_safely(&self.themes, &self.doc.file, &id) {
            Ok(file) => Ok(ToWrite { file, converted: true }),
            Err(err) => {
                tracing::error!("converting {} to the new theme format failed: {err}", path.display());
                Err(wormhole_text::text(wormhole_text::keys::STUDIO_CONVERT_FAILED).to_string())
            }
        }
    }
}
