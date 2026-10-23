//! The sizes and settings a terminal is started with: the grid's size in cells and pixels, and how the
//! engine behind it is set up. Below the engine, the PTY layer, the hosted terminals and the handle,
//! which all take them.

use crate::types;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSize {
    pub cols: u16,
    pub rows: u16,
    pub cell_width: f32,
    pub cell_height: f32,
}

impl GridSize {
    pub(crate) fn window_size(&self) -> WindowSize {
        WindowSize {
            num_lines: self.rows,
            num_cols: self.cols,
            cell_width: self.cell_width as u16,
            cell_height: self.cell_height as u16,
        }
    }
}

/// A grid's size in cells and in pixels: what a program that asks for the text area's size is told
/// and what an image is fitted to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowSize {
    pub num_lines: u16,
    pub num_cols: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// How to start the engine behind a terminal: what the screen is given before the program's first
/// byte. Where the program comes from, and its folder and environment, are the [`Launch`].
#[derive(Clone, Copy, Debug, Default)]
pub struct StartOptions<'a> {
    /// Previous session text, replayed dimmed before the shell starts.
    pub restore: Option<&'a str>,
    pub scrollback_lines: usize,
    /// Answer DECRQCRA, a program's request for the checksum of a rectangle of the screen
    /// (`allow_screen_checksum`); the engine ignores it otherwise.
    pub allow_screen_checksum: bool,
    /// Let the program act on the window (XTWINOPS: move, resize, minimise, …; DECCOLM changing the
    /// width) (`allow_window_ops`); the engine ignores both otherwise.
    pub allow_window_ops: bool,
    /// Answer the program's request for the window title and icon name (XTWINOPS 20 and 21)
    /// (`allow_title_report`); the engine ignores it otherwise.
    pub allow_title_report: bool,
    /// The theme's colors, which the engine answers a program's color queries (OSC 4, 10, 11, 12)
    /// with from its first byte; without them it answers with xterm's until
    /// [`Pty::set_default_colors`] is called, which a terminal that was never drawn has not yet.
    pub colors: Option<&'a [types::Rgb; types::COLOR_COUNT]>,
    /// Whether the theme is light or dark, which the engine answers a program's `CSI ? 996 n` with
    /// from its first byte; unanswered until [`Pty::set_color_scheme`] says.
    pub color_scheme: Option<types::ColorScheme>,
}
