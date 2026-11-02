//! What a checkpoint takes of a running hosted terminal, in one go: the screen as VT bytes, the size, the
//! folder and the title.

use super::vt::{Limits, primary_screen};
use crate::hosted::Reader;

/// A terminal as a checkpoint sees it at one moment.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Capture {
    pub cols: u16,
    pub rows: u16,
    /// The folder the shell reported with OSC 7, else the one the system says the program is in.
    pub cwd: Option<String>,
    pub title: Option<String>,
    /// The primary screen (see `vt`); `None` while a full-screen program shows the alternate one, and for a
    /// screen with nothing on it.
    pub screen: Option<Vec<u8>>,
}

/// Read `reader`'s terminal. Holds the screen's lock while it reads the rows, so it is for the checkpoint's
/// own thread and never the PTY's.
pub fn capture(reader: &Reader, limits: Limits) -> Capture {
    let (title, folder) = reader.title_and_folder();
    let cwd = folder.or_else(|| {
        let path = crate::sys::process::cwd(reader.pid())?;
        path.to_str().map(str::to_owned)
    });
    let (cols, rows, screen) = reader.with_screen(|screen| {
        (
            u16::try_from(screen.columns()).unwrap_or(u16::MAX),
            u16::try_from(screen.screen_lines()).unwrap_or(u16::MAX),
            primary_screen(screen, limits),
        )
    });
    Capture { cols, rows, cwd, title, screen }
}
