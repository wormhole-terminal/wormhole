//! What a program may do to the window (xterm's window operations, `allow_window_ops`) and what it may ask of the
//! window's place and size (always): the GPUI app's `window_ops.rs` and `terminal_view/window_geometry.rs`, as sums
//! Swift asks for and carries out with AppKit.
//!
//! A terminal is one pane of a window with a sidebar, a panel and a title row around it, and perhaps other panes
//! beside it, so the operations are read as being about the pane that asked: "resize to 40 rows" makes the window
//! as much larger or smaller as this pane's text area needs to get 40 rows (the frame around the pane stays as it
//! is), "the screen in pixels" is the screen's usable area (a window cannot be over the menu bar or the Dock), "the
//! screen in characters" is the most the pane's text area could have in this window. Every rectangle is in points,
//! from the top left corner of the screen the window is on (Swift turns AppKit's bottom-left frames round).
//!
//! The engine has already taken the sizes in cells when it allowed the operation (`wormhole_vt::Policy::window_ops`);
//! once the window has had its say, `Terminal::sync_size` brings the engine back to the pane's size.

use wormhole_term::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};

use crate::terminal::Terminal;

/// A rectangle in points from the top left of the window's screen.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The pane a program's terminal is in: its cell, its text area in cells, and what is around the text area in the
/// window (the sidebar, the title row, the panel, the other panes of a split), in points.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct PaneSizeRecord {
    pub cell_width: f32,
    pub cell_height: f32,
    pub cols: u32,
    pub rows: u32,
    pub frame_width: f32,
    pub frame_height: f32,
}

/// Where the window is: its frame, the whole screen and the part of it a window can use.
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct WindowPlace {
    pub window: ScreenRect,
    pub screen: ScreenRect,
    pub usable: ScreenRect,
    pub fullscreen: bool,
}

/// How far a resize goes along one direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ExtentRecord {
    Keep,
    Largest,
    Size { cells: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MaximizeRecord {
    Restore,
    Both,
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FullscreenRecord {
    Exit,
    Enter,
    Toggle,
}

/// What a program asked the window to do (`TermEvent::WindowOp`, only when `allow_window_ops` is on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum WindowOpRecord {
    Deiconify,
    Iconify,
    /// The window's top left corner to `x`, `y` of its screen.
    Move {
        x: i32,
        y: i32,
    },
    Raise,
    Lower,
    /// The pane's text area to this many cells.
    ResizeCells {
        cols: ExtentRecord,
        rows: ExtentRecord,
    },
    Maximize {
        how: MaximizeRecord,
    },
    Fullscreen {
        how: FullscreenRecord,
    },
}

impl WindowOpRecord {
    /// The engine's operation as Swift carries it out; `None` for one a newer engine adds that a window cannot do.
    pub(crate) fn of(op: WindowOp) -> Option<Self> {
        let extent = |extent: Extent| match extent {
            Extent::Keep => ExtentRecord::Keep,
            Extent::Largest => ExtentRecord::Largest,
            Extent::Size(cells) => ExtentRecord::Size { cells },
        };
        Some(match op {
            WindowOp::Deiconify => Self::Deiconify,
            WindowOp::Iconify => Self::Iconify,
            WindowOp::Move { x, y } => Self::Move { x, y },
            WindowOp::Raise => Self::Raise,
            WindowOp::Lower => Self::Lower,
            WindowOp::ResizeCells { rows, cols } => {
                Self::ResizeCells { cols: extent(cols), rows: extent(rows) }
            }
            WindowOp::Maximize(how) => Self::Maximize {
                how: match how {
                    Maximize::Restore => MaximizeRecord::Restore,
                    Maximize::Both => MaximizeRecord::Both,
                    Maximize::Vertical => MaximizeRecord::Vertical,
                    Maximize::Horizontal => MaximizeRecord::Horizontal,
                },
            },
            WindowOp::Fullscreen(how) => Self::Fullscreen {
                how: match how {
                    Fullscreen::Exit => FullscreenRecord::Exit,
                    Fullscreen::Enter => FullscreenRecord::Enter,
                    Fullscreen::Toggle => FullscreenRecord::Toggle,
                },
            },
            _ => return None,
        })
    }
}

/// Where a move or a resize puts the window, and the pane's size in cells it asked for (`None`: whatever the window
/// ends up with).
#[derive(Debug, Clone, Copy, PartialEq, uniffi::Record)]
pub struct WindowFrameGoal {
    pub frame: ScreenRect,
    pub cols: Option<u32>,
    pub rows: Option<u32>,
}

/// The frame `op` asks of a window at `place` around `pane`: a move kept inside the usable area, a resize to cells
/// (none in full screen) or along one direction (XTWINOPS 9;2 and 9;3) moved first so that all of it is on the
/// screen. `None` for an operation that is not a frame (iconify, raise, maximise both ways, full screen) or that
/// changes nothing.
#[uniffi::export]
pub fn window_op_frame(
    op: WindowOpRecord,
    place: WindowPlace,
    pane: PaneSizeRecord,
) -> Option<WindowFrameGoal> {
    let window = place.window;
    match op {
        WindowOpRecord::Move { x, y } => {
            let at = |wanted: i32, from: f32, space: f32, size: f32| {
                (wanted as f32).clamp(from, from + (space - size).max(0.0))
            };
            let usable = place.usable;
            let frame = ScreenRect {
                x: at(x, usable.x, usable.width, window.width),
                y: at(y, usable.y, usable.height, window.height),
                ..window
            };
            Some(WindowFrameGoal { frame, cols: None, rows: None })
        }
        WindowOpRecord::ResizeCells { cols, rows } if !place.fullscreen => {
            let [(most_cols, most_rows), _] = reach(&place, &pane);
            let target = |extent: ExtentRecord, now: u32, max: u32| match extent {
                ExtentRecord::Keep => now,
                ExtentRecord::Largest => max,
                ExtentRecord::Size { cells } => cells.clamp(1, max.max(now)),
            };
            let cols = target(cols, pane.cols, u32::from(most_cols));
            let rows = target(rows, pane.rows, u32::from(most_rows));
            if (cols, rows) == (pane.cols, pane.rows) {
                return None;
            }
            // Half a cell over, so that the points AppKit rounds the window to do not cost a cell.
            let width = pane.frame_width + (cols as f32 + 0.5) * pane.cell_width;
            let height = pane.frame_height + (rows as f32 + 0.5) * pane.cell_height;
            Some(WindowFrameGoal {
                frame: fit_on_screen(&place, width, height),
                cols: Some(cols),
                rows: Some(rows),
            })
        }
        WindowOpRecord::Maximize { how: how @ (MaximizeRecord::Vertical | MaximizeRecord::Horizontal) } => {
            let width = if how == MaximizeRecord::Horizontal { place.usable.width } else { window.width };
            let height = if how == MaximizeRecord::Vertical { place.usable.height } else { window.height };
            Some(WindowFrameGoal { frame: fit_on_screen(&place, width, height), cols: None, rows: None })
        }
        _ => None,
    }
}

/// What a program is told of the window (XTWINOPS 11, 13, 14, 15, 19): its place and size, the screen's usable area,
/// the most cells the pane could have (maximised, or the whole screen in full screen) and whether it is in the Dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct WindowGeometryRecord {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub max_cols: u16,
    pub max_rows: u16,
    pub iconified: bool,
}

/// The geometry of the window at `place` around `pane` (`window_geometry.rs` `geometry`).
#[uniffi::export]
pub fn window_geometry(place: WindowPlace, pane: PaneSizeRecord, iconified: bool) -> WindowGeometryRecord {
    let [usable_cells, full_cells] = reach(&place, &pane);
    let (max_cols, max_rows) = if place.fullscreen { full_cells } else { usable_cells };
    let window = place.window;
    WindowGeometryRecord {
        x: window.x.round() as i32,
        y: window.y.round() as i32,
        width: window.width.round().max(0.0) as u32,
        height: window.height.round().max(0.0) as u32,
        screen_width: place.usable.width.round().max(0.0) as u32,
        screen_height: place.usable.height.round().max(0.0) as u32,
        max_cols,
        max_rows,
        iconified,
    }
}

#[uniffi::export]
impl Terminal {
    /// Tell the engine where the window is, for the reports a program may ask for; the latest is kept. Skip it while a
    /// resize a program asked for has not been followed yet: the engine already has the size it asked for.
    pub fn set_window(&self, geometry: WindowGeometryRecord) {
        let WindowGeometryRecord {
            x,
            y,
            width,
            height,
            screen_width,
            screen_height,
            max_cols,
            max_rows,
            iconified,
        } = geometry;
        self.with_pty(|pty| {
            pty.set_window(WindowGeometry {
                x,
                y,
                width,
                height,
                screen_width,
                screen_height,
                max_cols,
                max_rows,
                iconified,
            })
        });
    }

    /// Bring the engine back to the size this terminal has, after the window followed (or could not follow) a size a
    /// program asked for.
    pub fn sync_size(&self) {
        self.with_pty(wormhole_term::Pty::sync_size);
    }
}

/// The most cells the pane's text area could have, as `(cols, rows)`: in the screen's usable area (what a maximised
/// window gets) and in full screen (over the Dock, below the menu bar's strip), never fewer than it has now.
fn reach(place: &WindowPlace, pane: &PaneSizeRecord) -> [(u16, u16); 2] {
    let most = |space: f32, frame: f32, cell: f32, now: u32| {
        let cells = ((space - frame) / cell.max(1.0)).floor().max(0.0) as u32;
        cells.max(now).min(u32::from(u16::MAX)) as u16
    };
    let below_bar = place.screen.height - place.usable.y;
    [
        (
            most(place.usable.width, pane.frame_width, pane.cell_width, pane.cols),
            most(place.usable.height, pane.frame_height, pane.cell_height, pane.rows),
        ),
        (
            most(place.screen.width, pane.frame_width, pane.cell_width, pane.cols),
            most(below_bar, pane.frame_height, pane.cell_height, pane.rows),
        ),
    ]
}

/// The window, about to be `width` by `height`, moved so that all of it is on the screen: below the menu bar, over
/// the Dock rather than off the screen.
fn fit_on_screen(place: &WindowPlace, width: f32, height: f32) -> ScreenRect {
    let at = |now: f32, low: f32, end: f32, extent: f32| now.clamp(low, (end - extent).max(low));
    let (screen, usable) = (place.screen, place.usable);
    ScreenRect {
        x: at(place.window.x, usable.x, screen.x + screen.width, width),
        y: at(place.window.y, usable.y, screen.y + screen.height, height),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: ScreenRect = ScreenRect { x: 0.0, y: 0.0, width: 1512.0, height: 982.0 };
    /// Below a 33-point menu bar, above a 70-point Dock.
    const USABLE: ScreenRect = ScreenRect { x: 0.0, y: 33.0, width: 1512.0, height: 879.0 };
    const PANE: PaneSizeRecord = PaneSizeRecord {
        cell_width: 8.0,
        cell_height: 17.0,
        cols: 80,
        rows: 24,
        frame_width: 300.0,
        frame_height: 60.0,
    };

    fn place(x: f32, y: f32, width: f32, height: f32) -> WindowPlace {
        WindowPlace {
            window: ScreenRect { x, y, width, height },
            screen: SCREEN,
            usable: USABLE,
            fullscreen: false,
        }
    }

    #[test]
    fn a_resize_in_cells_grows_the_window_by_what_the_pane_needs_and_keeps_it_on_screen() {
        let op =
            WindowOpRecord::ResizeCells { cols: ExtentRecord::Size { cells: 100 }, rows: ExtentRecord::Keep };
        let goal = window_op_frame(op, place(100.0, 100.0, 944.0, 468.0), PANE).expect("a frame");
        assert_eq!((goal.cols, goal.rows), (Some(100), Some(24)));
        assert_eq!(goal.frame.width, 300.0 + 100.5 * 8.0);
        assert_eq!(goal.frame.height, 60.0 + 24.5 * 17.0);
        // Near the right edge: moved left so the whole window is on the screen.
        let goal = window_op_frame(op, place(1200.0, 100.0, 944.0, 468.0), PANE).expect("a frame");
        assert_eq!(goal.frame.x + goal.frame.width, SCREEN.width);
        // More than fits: as many as the usable area holds.
        let huge = WindowOpRecord::ResizeCells {
            cols: ExtentRecord::Size { cells: 9999 },
            rows: ExtentRecord::Largest,
        };
        let goal = window_op_frame(huge, place(0.0, 33.0, 944.0, 468.0), PANE).expect("a frame");
        assert_eq!(goal.cols, Some(((1512.0 - 300.0) / 8.0) as u32));
        assert_eq!(goal.rows, Some(((879.0 - 60.0) / 17.0) as u32));
        // The same size, or full screen: nothing.
        let same =
            WindowOpRecord::ResizeCells { cols: ExtentRecord::Size { cells: 80 }, rows: ExtentRecord::Keep };
        assert_eq!(window_op_frame(same, place(0.0, 33.0, 944.0, 468.0), PANE), None);
        let mut full = place(0.0, 0.0, 1512.0, 982.0);
        full.fullscreen = true;
        assert_eq!(window_op_frame(op, full, PANE), None);
    }

    #[test]
    fn a_move_stays_in_the_usable_area() {
        let goal =
            window_op_frame(WindowOpRecord::Move { x: 5000, y: 0 }, place(10.0, 40.0, 800.0, 500.0), PANE);
        let frame = goal.expect("a frame").frame;
        assert_eq!(
            (frame.x, frame.y, frame.width),
            (1512.0 - 800.0, 33.0, 800.0),
            "off the right, under the menu bar"
        );
    }

    #[test]
    fn the_geometry_says_the_window_the_usable_screen_and_the_most_cells() {
        let geometry = window_geometry(place(10.4, 50.6, 944.0, 468.0), PANE, false);
        assert_eq!((geometry.x, geometry.y, geometry.width, geometry.height), (10, 51, 944, 468));
        assert_eq!((geometry.screen_width, geometry.screen_height), (1512, 879));
        assert_eq!((geometry.max_cols, geometry.max_rows), (151, 48));
        let mut full = place(0.0, 0.0, 1512.0, 982.0);
        full.fullscreen = true;
        assert_eq!(window_geometry(full, PANE, false).max_rows, ((982.0 - 33.0 - 60.0) / 17.0) as u16);
        assert!(window_geometry(place(0.0, 0.0, 1.0, 1.0), PANE, true).iconified);
    }
}
