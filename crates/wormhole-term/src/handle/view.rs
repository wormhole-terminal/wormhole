//! What the screen shows and how it is scrolled: its snapshot, the cursor, the colours, the inline images and the engine's idea of the theme.

use crate::Pty;
use wormhole_vt::{Colors, Line, Modes, Scroll};

use crate::{Backend, ScreenSnapshot, find, images, minimap, types};

impl Pty {
    pub fn mode(&self) -> Modes {
        self.term.lock().mode()
    }

    /// Bring `out` up to date with the screen and `above` rows of history over it.
    pub fn snapshot(&self, out: &mut ScreenSnapshot, above: usize) {
        out.update(&mut self.term.lock(), above);
        if !out.block_spans().is_empty() {
            let log = self.shared.commands();
            out.set_block_states(|id| log.state(id));
        }
    }

    /// Bring the minimap's copy up to date for a minimap `fits` lines tall (read again only when
    /// the output or the view changed).
    pub fn minimap(&self, out: &mut minimap::MinimapSnapshot, fits: usize) {
        let revision = self.output_revision();
        out.update(&self.term.lock(), fits, revision);
    }

    /// The cursor's row on the screen as shown, unless the view is scrolled away from it.
    pub fn cursor_row(&self) -> Option<usize> {
        let term = self.term.lock();
        let row = term.grid_cursor().line.0 + term.display_offset() as i32;
        usize::try_from(row).ok().filter(|&row| row < term.screen_lines())
    }

    /// Colors programs set at runtime (OSC 4/10/11/12), by palette index.
    pub fn colors(&self) -> Colors {
        self.term.lock().colors()
    }

    /// Scroll the view through the history.
    pub fn scroll(&self, scroll: Scroll) {
        self.term.lock().scroll_display(scroll);
    }

    /// How many lines the view is scrolled up into the history.
    pub fn display_offset(&self) -> usize {
        self.term.lock().display_offset()
    }

    /// Scroll the view so `line` (a grid line: 0 is the live screen's top) is in it: a line out
    /// of view comes to the middle of the screen; one in view leaves the view where it is.
    pub fn reveal(&self, line: Line) {
        let mut term = self.term.lock();
        let offset = term.display_offset();
        let wanted = find::reveal_offset(line, term.screen_lines(), offset, term.history_size());
        if let Some(wanted) = wanted {
            term.scroll_display(Scroll::Delta(wanted as i32 - offset as i32));
        }
    }

    /// Scroll the view so `line` is its top row, as far as the history allows (a line on the live
    /// screen brings the view back to it).
    pub fn scroll_line_to_top(&self, line: Line) {
        let mut term = self.term.lock();
        let offset = term.display_offset() as i32;
        let wanted = (-line.0).clamp(0, term.history_size() as i32);
        if wanted != offset {
            term.scroll_display(Scroll::Delta(wanted - offset));
        }
    }

    /// Erase the history (the screen stays).
    pub fn clear_history(&self) {
        self.term.lock().clear_history();
    }

    /// The grid's last line (the live screen's bottom row).
    pub fn bottommost_line(&self) -> Line {
        self.term.lock().bottommost_line()
    }

    /// Give the engine the theme's colors, for the programs that ask what they are (OSC 4, 10,
    /// 11, 12): it answers those itself.
    pub fn set_default_colors(&self, palette: &[types::Rgb; types::COLOR_COUNT]) {
        self.term.lock().set_default_colors(palette);
        // The host's engine is the one that answers.
        if let Backend::Hosted(host) = &self.io {
            host.set_colors(palette);
        }
    }

    /// Tell the engine whether the theme is light or dark, for the programs that ask
    /// (`CSI ? 996 n`) and the ones that asked to hear of changes (mode 2031): they are told now.
    pub fn set_color_scheme(&self, scheme: types::ColorScheme) {
        let replies = self.term.lock().set_color_scheme(scheme);
        match &self.io {
            Backend::Local(_) => {
                if !replies.is_empty() && self.is_alive() {
                    self.shared.reply(replies);
                }
            }
            // The host's engine answers and tells the program; the replica's report would be a
            // second one.
            Backend::Hosted(host) => host.set_color_scheme(scheme),
        }
    }

    /// The inline images (`images`): what the placeholder cells in a snapshot draw. The lock is
    /// for a frame's lookups; nothing waits on it.
    pub fn images(&self) -> std::sync::MutexGuard<'_, images::ImageStore> {
        self.shared.images()
    }

    /// The virtual placement (kitty's `U=1`) the placeholder cells with this picture id and
    /// placement id belong to. Pictures placed in the grid are drawn from
    /// [`ScreenSnapshot::placements`]; these are the ones drawn from placeholder cells.
    pub fn virtual_placement(&self, image_id: u32, placement_id: u32) -> Option<images::Tiles> {
        let placement = self.term.lock().virtual_placement(image_id, placement_id)?;
        Some(images::Tiles {
            key: images::key_of(placement.image),
            cols: placement.cols,
            rows: placement.rows,
        })
    }
}
