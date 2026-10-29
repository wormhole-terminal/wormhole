//! What the Studio edits: a theme file and the state it was last saved in. Pure, with no GPUI,
//! so it is tested without a window. `layers` reads and writes the file's values by token name.

pub(super) mod history;
pub(super) mod layers;
pub(super) mod subject;

use wormhole_core::theme::tokens::ThemeFile;

use super::themes::Stamp;

pub(super) use layers::{changes, is_metric, number, override_of, set_override};
pub(super) use subject::Subject;

/// The theme under edit.
pub(super) struct Document {
    /// The file as edited now.
    pub(super) file: ThemeFile,
    /// The file as last read or saved: what a change, and Revert to Saved, compare with.
    pub(super) saved: ThemeFile,
    /// What it is and where it lives.
    pub(super) subject: Subject,
    /// The file as it was when read or last written: a write over a different one asks first.
    pub(super) stamp: Option<Stamp>,
}

impl Document {
    pub(super) fn new(file: ThemeFile, subject: Subject, stamp: Option<Stamp>) -> Self {
        Self { saved: file.clone(), file, subject, stamp }
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

    /// The saved file is the file again: the edits are given up.
    pub(super) fn revert(&mut self) {
        self.file = self.saved.clone();
    }

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
        let mut doc = Document::new(ThemeFile::default(), Subject::Untitled { from: String::new() }, None);
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
