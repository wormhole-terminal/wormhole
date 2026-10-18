//! What a hosted terminal's replica is made with (again on every repaint), and the sizes and
//! colors as the wire carries them.

use wormhole_wire as wire;

use crate::screen::Screen;
use crate::types::{COLOR_COUNT, NamedColor, Rgb};
use crate::{GridSize, StartOptions};

/// What the replica is made with, again on every repaint.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ReplicaOptions {
    pub scrollback_lines: usize,
    pub allow_screen_checksum: bool,
    pub allow_window_ops: bool,
}

impl ReplicaOptions {
    pub(crate) fn of(options: &StartOptions<'_>) -> Self {
        Self {
            scrollback_lines: options.scrollback_lines,
            allow_screen_checksum: options.allow_screen_checksum,
            allow_window_ops: options.allow_window_ops,
        }
    }

    /// A fresh screen of `size` with these options.
    pub(crate) fn screen(&self, size: GridSize) -> Screen {
        let mut screen = Screen::new(size, self.scrollback_lines);
        screen.set_screen_checksum(self.allow_screen_checksum);
        screen.set_window_ops(self.allow_window_ops);
        screen
    }
}

/// The grid size of a wire size (cells in 1/16 pixel).
pub(crate) fn grid_of(size: wire::Size) -> GridSize {
    GridSize {
        cols: size.cols.max(1),
        rows: size.rows.max(1),
        cell_width: f32::from(size.cell_w) / 16.0,
        cell_height: f32::from(size.cell_h) / 16.0,
    }
}

/// The wire size of a grid size.
pub(crate) fn wire_size(size: GridSize) -> wire::Size {
    let sixteenths = |px: f32| (px * 16.0).round().clamp(0.0, f32::from(u16::MAX)) as u16;
    wire::Size {
        cols: size.cols,
        rows: size.rows,
        cell_w: sixteenths(size.cell_width),
        cell_h: sixteenths(size.cell_height),
    }
}

/// The colors a host answers color queries with, from a full palette.
pub(crate) fn wire_colors(palette: &[Rgb; COLOR_COUNT]) -> wire::Colors<'static> {
    let rgb = |c: Rgb| [c.r, c.g, c.b];
    wire::Colors {
        fg: rgb(palette[NamedColor::Foreground as usize]),
        bg: rgb(palette[NamedColor::Background as usize]),
        cursor: Some(rgb(palette[NamedColor::Cursor as usize])),
        palette: wire::List::Owned(palette.iter().take(wire::MAX_PALETTE).map(|c| rgb(*c)).collect()),
    }
}
