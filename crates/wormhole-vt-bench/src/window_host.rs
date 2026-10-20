//! The host side of the engine for the headless suite runs (`esctest-host`, `vttest-host`): what the
//! app does with the engine's events, on a fake window.
//!
//! The window is a text area of cells plus a frame (the sidebar and the title bar), on a screen
//! of fixed size, like the app's window around a pane: the geometry the reports describe is
//! computed from it, and the window operations move and size it the way the app's would.

use wormhole_vt::window::{Extent, Fullscreen, Maximize, WindowGeometry, WindowOp};
use wormhole_vt::{COLOR_COUNT, Event, NamedColor, Policy, Rgb, Terminal};

/// A cell in pixels.
const CELL: (u32, u32) = (7, 17);
/// What is around the text area in the window: the sidebar, the title bar.
const FRAME: (u32, u32) = (240, 60);
/// The screen.
const SCREEN: (u32, u32) = (1920, 1080);
/// What the menu bar and the Dock take of the screen's height: a maximised window stops short of
/// both, a full screen window of the menu bar's only (see `most`).
const BARS: u32 = 100;
/// The menu bar's part of `BARS`.
const MENU_BAR: u32 = 38;

/// The theme: xterm's own default colors (black on white), which esctest2 assumes when it
/// resets the dynamic colors, and xterm's palette.
fn theme() -> Box<[Rgb; COLOR_COUNT]> {
    const ANSI: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (0xcd, 0, 0),
        (0, 0xcd, 0),
        (0xcd, 0xcd, 0),
        (0, 0, 0xee),
        (0xcd, 0, 0xcd),
        (0, 0xcd, 0xcd),
        (0xe5, 0xe5, 0xe5),
        (0x7f, 0x7f, 0x7f),
        (0xff, 0, 0),
        (0, 0xff, 0),
        (0xff, 0xff, 0),
        (0x5c, 0x5c, 0xff),
        (0xff, 0, 0xff),
        (0, 0xff, 0xff),
        (0xff, 0xff, 0xff),
    ];
    let mut palette = Box::new([Rgb::default(); COLOR_COUNT]);
    for (i, slot) in palette.iter_mut().enumerate().take(256) {
        let (r, g, b) = match i {
            0..=15 => ANSI[i],
            16..=231 => {
                let n = (i - 16) as u8;
                let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                (level(n / 36), level(n / 6 % 6), level(n % 6))
            }
            _ => {
                let v = 8 + 10 * (i - 232) as u8;
                (v, v, v)
            }
        };
        *slot = Rgb { r, g, b };
    }
    palette[NamedColor::Foreground as usize] = Rgb { r: 0, g: 0, b: 0 };
    palette[NamedColor::Background as usize] = Rgb { r: 255, g: 255, b: 255 };
    palette[NamedColor::Cursor as usize] = Rgb { r: 0, g: 0, b: 0 };
    palette
}

/// A terminal and what plays the app around it.
pub struct Host {
    term: Terminal,
    clipboard: String,
    window_ops: bool,
    /// The window's top left corner.
    at: (i32, i32),
    iconified: bool,
    fullscreen: bool,
    /// The size before a maximise, to go back to.
    restore: Option<(usize, usize)>,
}

impl Host {
    pub fn new(cols: usize, rows: usize, window_ops: bool, checksum: bool) -> Host {
        let mut policy = Policy::default();
        policy.screen_checksum = checksum;
        policy.window_ops = window_ops;
        // esctest2 reads the title back (XTWINOPS 20 and 21), so the reports go with the operations.
        policy.title_report = window_ops;
        let term = Terminal::builder()
            .size(cols, rows)
            .scrollback(1000)
            .identity("Wormhole", env!("CARGO_PKG_VERSION"))
            .cell_size(CELL.0 as u16, CELL.1 as u16)
            .palette(&theme())
            .policy(policy)
            .build();
        let mut host = Host {
            term,
            clipboard: String::new(),
            window_ops,
            at: (100, 80),
            iconified: false,
            fullscreen: false,
            restore: None,
        };
        host.publish_window();
        host
    }

    /// The cells of a text area that fills `height` of the screen.
    fn cells(height: u32) -> (usize, usize) {
        (((SCREEN.0 - FRAME.0) / CELL.0) as usize, ((height - FRAME.1) / CELL.1) as usize)
    }

    /// The most cells the text area can have in a maximised window, which is the most a window
    /// can be: the screen less the bars and the frame.
    fn usable(&self) -> (usize, usize) {
        Self::cells(SCREEN.1 - BARS)
    }

    /// The most cells the text area can have, as the app reports it (XTWINOPS 19) and lets a
    /// resize reach: in full screen the whole screen, else what a maximised window gets, as the
    /// app does.
    fn most(&self) -> (usize, usize) {
        let full = Self::cells(SCREEN.1 - MENU_BAR);
        if self.fullscreen {
            return full;
        }
        self.usable()
    }

    /// Tell the engine where the window is.
    fn publish_window(&mut self) {
        let (cols, rows) = self.size();
        let (max_cols, max_rows) = self.most();
        self.term.set_window(WindowGeometry {
            x: self.at.0,
            y: self.at.1,
            width: cols as u32 * CELL.0 + FRAME.0,
            height: rows as u32 * CELL.1 + FRAME.1,
            screen_width: SCREEN.0,
            // The part of the screen a window can use.
            screen_height: SCREEN.1 - BARS,
            max_cols: max_cols as u16,
            max_rows: max_rows as u16,
            iconified: self.iconified,
        });
    }

    fn size(&self) -> (usize, usize) {
        (self.term.cols(), self.term.rows())
    }

    /// Resize the window so the text area is this size, as far as the screen allows: a window
    /// stays clear of the bars, full screen has none.
    fn set_size(&mut self, cols: usize, rows: usize) {
        let (max_cols, max_rows) = if self.fullscreen { self.most() } else { self.usable() };
        let (cols, rows) = (cols.clamp(2, max_cols), rows.clamp(1, max_rows));
        self.term.resize(cols, rows);
        // The window grows to the left and down; it must stay on the screen.
        let width = cols as u32 * CELL.0 + FRAME.0;
        let height = rows as u32 * CELL.1 + FRAME.1;
        self.at.0 = self.at.0.clamp(0, SCREEN.0.saturating_sub(width) as i32);
        self.at.1 = self.at.1.clamp(0, SCREEN.1.saturating_sub(height) as i32);
    }

    fn extent(extent: Extent, now: usize, most: usize) -> usize {
        match extent {
            Extent::Keep => now,
            Extent::Largest => most,
            Extent::Size(n) => n as usize,
        }
    }

    fn window_op(&mut self, op: WindowOp) {
        if !self.window_ops {
            return;
        }
        let (cols, rows) = self.size();
        let (usable_cols, usable_rows) = self.usable();
        match op {
            WindowOp::Iconify => self.iconified = true,
            WindowOp::Deiconify => self.iconified = false,
            WindowOp::Raise | WindowOp::Lower => {}
            WindowOp::Move { x, y } => {
                let width = cols as u32 * CELL.0 + FRAME.0;
                let height = rows as u32 * CELL.1 + FRAME.1;
                self.at.0 = x.clamp(0, SCREEN.0.saturating_sub(width) as i32);
                self.at.1 = y.clamp(0, SCREEN.1.saturating_sub(height) as i32);
            }
            // The window is the screen in full screen mode: a resize is left alone, as in the app.
            WindowOp::ResizeCells { .. } if self.fullscreen => {}
            WindowOp::ResizeCells { rows: height, cols: width } => {
                self.restore = None;
                self.set_size(
                    Self::extent(width, cols, usable_cols),
                    Self::extent(height, rows, usable_rows),
                );
            }
            WindowOp::Maximize(how) => {
                let restore = *self.restore.get_or_insert((cols, rows));
                match how {
                    Maximize::Restore => {
                        self.restore = None;
                        self.set_size(restore.0, restore.1);
                    }
                    Maximize::Both => self.set_size(usable_cols, usable_rows),
                    Maximize::Vertical => self.set_size(cols, usable_rows),
                    Maximize::Horizontal => self.set_size(usable_cols, rows),
                }
            }
            WindowOp::Fullscreen(how) => {
                let enter = match how {
                    Fullscreen::Enter => true,
                    Fullscreen::Exit => false,
                    Fullscreen::Toggle => !self.fullscreen,
                };
                if enter && !self.fullscreen {
                    self.restore = Some((cols, rows));
                    self.fullscreen = true;
                    let (cols, rows) = self.most();
                    self.set_size(cols, rows);
                } else if !enter && self.fullscreen {
                    self.fullscreen = false;
                    if let Some((cols, rows)) = self.restore.take() {
                        self.set_size(cols, rows);
                    }
                }
            }
            _ => {}
        }
    }

    /// The terminal, to read its screen.
    pub fn term(&self) -> &Terminal {
        &self.term
    }

    /// Feed what the program wrote; returns what to write back.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.term.feed(bytes);
        let events: Vec<Event> = self.term.events().collect();
        let mut out = Vec::new();
        let mut sent = 0;
        for event in events {
            match event {
                Event::ClipboardStore { text, .. } => self.clipboard = text,
                Event::ClipboardLoad { selection, terminator, replies_before } => {
                    let replies = self.term.replies();
                    let until = replies_before.min(replies.len());
                    out.extend_from_slice(&replies[sent..until]);
                    sent = until;
                    let text = self.clipboard.clone();
                    // The engine queues the clipboard's answer after the replies it already holds.
                    let mut answer = Terminal::builder().size(2, 1).scrollback(0).build();
                    answer.clipboard_reply(&selection, &text, terminator);
                    out.extend(answer.take_replies());
                }
                Event::WindowOp(op) => self.window_op(op),
                // DECCOLM: the window is made to match when the app allows window operations.
                Event::ColumnsRequested(cols) => {
                    if self.window_ops {
                        let rows = self.size().1;
                        self.set_size(cols, rows);
                    } else {
                        let (cols, rows) = (80, 25);
                        self.term.resize(cols, rows);
                    }
                }
                _ => {}
            }
        }
        self.publish_window();
        out.extend_from_slice(&self.term.replies()[sent.min(self.term.replies().len())..]);
        self.term.clear_replies();
        out
    }
}
