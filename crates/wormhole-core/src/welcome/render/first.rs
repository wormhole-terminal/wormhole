//! The first two pages: the welcome page, with the mark the start animation lands on, and how it looks.

use super::super::ansi::{Style, fit, text_width};
use super::super::art::{self, LOGO, MARGIN, MARK_CELL};
use super::super::keys::Key;
use super::super::model::Welcome;
use super::{
    ACCENT, BACK, DIM, ESC_SKIP, Hint, MARK, Page, ROW_EVERY, TAGLINE, TITLE, TYPE_NAME, TYPE_TEXT, UP_DOWN,
    VALUE_AT, window_start,
};
use crate::intro::Handover;
use crate::keymap::{self, ActionId};

impl Page {
    /// The mark, the name and what it is, then the sidebar's three lines and the way on.
    pub(super) fn welcome(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let next = self.header(w.handover());
        let mut row = next;
        let lines = [
            "your terminals and what their agents are doing".to_string(),
            "which is working, which needs you, which is done".to_string(),
            format!("{} opens another; click one to switch", keymap::glyphs(ActionId::NewTerminal)),
        ];
        for line in lines {
            self.shown(row, MARGIN, &[("←", ACCENT), ("  ", Style::PLAIN), (&line, Style::PLAIN)]);
            row += 1;
        }
        row += 1;
        let label = if w.is_replay() { "Back to Wormhole" } else { "Get started" };
        self.item_row(row, MARGIN, true, &[(label, TITLE)], 0);
        vec![
            ("⏎", if w.is_replay() { "back to Wormhole" } else { "get started" }, Some(Key::Enter)),
            ESC_SKIP,
        ]
    }

    /// The top of the welcome page, which the start animation lands on: the mark's `>` is at
    /// [`MARK_CELL`], or the logo at [`LOGO`]. Returns the row the page continues from.
    fn header(&mut self, handover: Handover) -> usize {
        let (row, col) = MARK_CELL;
        match handover {
            Handover::Logo => {
                // The ship has docked on the chevron: the logo is there, the name types beside it.
                if self.out(self.t, ROW_EVERY, 1) > 0 {
                    art::draw_logo(&mut self.grid, LOGO.row, LOGO.col);
                }
                self.t += ROW_EVERY;
                let text_col = LOGO.col + LOGO.cols + 2;
                self.typed(LOGO.row + 1, text_col, &[("Wormhole", TITLE)], TYPE_NAME);
                self.typed(LOGO.row + 2, text_col, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                LOGO.row + LOGO.rows + 1
            }
            Handover::Cursor => {
                // The ship has docked on the `>`: it is there, the `_` a beat after, then the name.
                self.shown(row, col, &[(">", MARK)]);
                self.shown(row, col + 1, &[("_", MARK)]);
                self.typed(row, col + 3, &[("Wormhole", TITLE)], TYPE_NAME);
                self.typed(row + 1, col + 3, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                row + 3
            }
            Handover::Portal => {
                // No ship: the mark types with the name.
                self.typed(row, col, &[(">_", MARK), (" ", Style::PLAIN), ("Wormhole", TITLE)], TYPE_NAME);
                self.typed(row + 1, col + 3, &[(TAGLINE, Style::PLAIN)], TYPE_TEXT);
                row + 3
            }
        }
    }

    pub(super) fn look(&mut self, w: &Welcome) -> Vec<Hint<'static>> {
        let look = w.look();
        if look.themes.is_empty() {
            let row = self.heading(
                "How should it look?",
                &format!(
                    "Themes, colours and the font are in Settings ({}).",
                    keymap::glyphs(ActionId::Settings)
                ),
            );
            self.shown(row, MARGIN, &[("Nothing changes here: this tour is read-only.", DIM)]);
            return vec![("⏎", "continue", Some(Key::Enter)), BACK, ESC_SKIP];
        }
        let row =
            self.heading("How should it look?", "Themes apply as you move. The rest of it is in Settings.");
        self.shown(row, MARGIN, &[("Theme", DIM)]);
        // The rows below the list: a blank, the colour, the font and the ligatures.
        let below = 4;
        let height = self.list_height(row + 1, below);
        let cursor = w.cursor();
        let first = window_start(cursor, look.themes.len(), height);
        for (line, theme) in look.themes.iter().enumerate().skip(first).take(height) {
            let at = row + 1 + line - first;
            let style = if line == cursor { ACCENT } else { Style::PLAIN };
            let end = self.item_row(at, MARGIN, line == cursor, &[(&theme.name, style)], line);
            if theme.light {
                self.also(at, end + 2, &[("light", DIM)], self.turn);
            }
        }
        let mut at = row + 1 + height.min(look.themes.len()) + 1;
        // The primary colour: a dot in its own colour before each name, the one in use filled and bold.
        self.shown(at, MARGIN, &[("Colour", DIM)]);
        let turn = self.turn;
        let mut col = MARGIN + VALUE_AT;
        for primary in &look.primaries {
            let current = primary.hex == look.primary;
            let dot = Style { fg: primary.ansi, ..Style::PLAIN };
            let name = if current { TITLE } else { Style::PLAIN };
            let parts = [(if current { "●" } else { "○" }, dot), (" ", Style::PLAIN), (&primary.name, name)];
            let width = text_width(&primary.name) + 2;
            if col + width > self.size.cols.saturating_sub(MARGIN) {
                break;
            }
            self.also(at, col, &parts, turn);
            col += width + 2;
        }
        at += 1;
        let font =
            fit(&format!("{} {:.0}", look.font, look.font_size), self.text_width().saturating_sub(VALUE_AT));
        self.shown(at, MARGIN, &[("Font", DIM)]);
        self.also(at, MARGIN + VALUE_AT, &[(&font, Style::PLAIN)], self.turn);
        at += 1;
        self.shown(at, MARGIN, &[("Ligatures", DIM)]);
        let state = if look.ligatures { "on " } else { "off" };
        self.also(
            at,
            MARGIN + VALUE_AT,
            &[(state, TITLE), ("  ", Style::PLAIN), ("=> != >=", DIM)],
            self.turn,
        );
        vec![
            UP_DOWN,
            ("p", "colour", Some(Key::Char('p'))),
            ("l", "ligatures", Some(Key::Char('l'))),
            ("+ -", "size", None),
            ("⏎", "continue", Some(Key::Enter)),
            BACK,
            ESC_SKIP,
        ]
    }
}
