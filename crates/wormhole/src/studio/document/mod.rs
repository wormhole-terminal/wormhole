//! What the Studio edits: a theme file and the state it was last saved in. Pure, with no GPUI,
//! so it is tested without a window. `layers` reads and writes the file's values by token name.

pub(super) mod layers;

use wormhole_core::theme::tokens::ThemeFile;

pub(super) use layers::{changes, is_metric, number, override_of, set_override};

/// The theme under edit.
pub(super) struct Document {
    /// The file as edited now.
    pub(super) file: ThemeFile,
    /// The file as last read or saved: what a change, and Revert to Saved, compare with.
    pub(super) saved: ThemeFile,
}

impl Document {
    pub(super) fn new(file: ThemeFile) -> Self {
        Self { saved: file.clone(), file }
    }

    /// Whether the file has edits that are not saved. Compared with the saved file, so undoing
    /// back to it clears it.
    pub(super) fn dirty(&self) -> bool {
        self.file != self.saved
    }

    /// The file as it stands is now what is saved.
    pub(super) fn mark_saved(&mut self) {
        self.saved = self.file.clone();
    }

    #[allow(dead_code, reason = "Revert to Saved, a few steps on")]
    /// The saved file is the file again: the edits are given up.
    pub(super) fn revert(&mut self) {
        self.file = self.saved.clone();
    }

    #[allow(dead_code, reason = "the title row and the menu, a few steps on")]
    /// The file's display name, empty when it has none.
    pub(super) fn name(&self) -> &str {
        self.file.name.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_is_dirty_until_it_is_back_at_the_saved_file() {
        let mut doc = Document::new(ThemeFile::default());
        assert!(!doc.dirty());
        doc.file.ui.insert("primary".into(), "#fff".into());
        assert!(doc.dirty());
        doc.mark_saved();
        assert!(!doc.dirty());
        doc.file.ui.clear();
        assert!(doc.dirty());
        doc.revert();
        assert!(!doc.dirty());
        assert!(doc.file.ui.contains_key("primary"));
    }
}
