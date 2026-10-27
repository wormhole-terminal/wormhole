//! The look page's offer to bring over the settings of the terminal the person used before: what was
//! found, the way to take it, and afterwards what came over. All its words are catalog messages
//! (`settings-import-*`), the same ones Settings → Import and the intro window's step use.

use wormhole_text::keys::{
    SETTINGS_IMPORT_WELCOME_TOUR, settings_import_onboarding_all, settings_import_onboarding_summary,
    settings_import_use_mine, settings_import_welcome_found,
};
use wormhole_text::{text, whole};

use super::super::ansi::{Style, fit};
use super::super::art::MARGIN;
use super::super::keys::Key;
use super::super::model::{Import, Welcome};
use super::{ACCENT, DIM, GOOD, Page, TITLE, Target};

/// The rows the offer needs under the look page's other lines: a blank, what was found, and the way
/// to take it (or what came over). None when nothing was found.
pub(super) fn rows(w: &Welcome) -> usize {
    if w.import().is_some() { 3 } else { 0 }
}

impl Page {
    /// The offer's lines from `row` (the blank before them is the caller's). A tour says where the
    /// settings can be brought over instead of offering to; once they are, the line says what came.
    pub(super) fn import_offer(&mut self, w: &Welcome, row: usize) {
        let Some(import) = w.import() else { return };
        let width = self.text_width();
        match import {
            Import::Done(_, done) => {
                let said = if done.left == 0 {
                    settings_import_onboarding_all(whole(done.brought))
                } else {
                    settings_import_onboarding_summary(whole(done.brought), &done.left.to_string())
                };
                self.shown(row, MARGIN, &[("✓", GOOD), (" ", Style::PLAIN), (&fit(&said, width - 2), GOOD)]);
            }
            Import::Offered(offer) | Import::Asked(offer) => {
                let name = text(offer.source.name_key());
                let found = settings_import_welcome_found(name);
                self.shown(row, MARGIN, &[(&fit(&found, width), Style::PLAIN)]);
                if !w.live() {
                    self.shown(row + 1, MARGIN, &[(text(SETTINGS_IMPORT_WELCOME_TOUR), DIM)]);
                } else if matches!(import, Import::Offered(_)) {
                    // The key, then what it does, as the footer's hints are written.
                    let label = fit(&settings_import_use_mine(name), width.saturating_sub(3));
                    let end =
                        self.shown(row + 1, MARGIN, &[("i", TITLE), ("  ", Style::PLAIN), (&label, ACCENT)]);
                    self.hit(row + 1, MARGIN, end, Target::Key(Key::Char('i')));
                }
            }
        }
    }
}
