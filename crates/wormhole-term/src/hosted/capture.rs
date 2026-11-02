//! What a checkpoint takes of a running hosted terminal: its primary screen as VT bytes.

use super::Reader;
use crate::checkpoint::Limits;
use crate::checkpoint::vt::primary_screen;

/// The primary screen of `reader`'s terminal (see `checkpoint::vt`); `None` while a full-screen program shows
/// the alternate one, and for a screen with nothing on it. It looks at the screen in slices, each under the
/// screen's lock, so it is for the checkpoint's own thread and never the PTY's.
pub fn capture(reader: &Reader, limits: Limits) -> Option<Vec<u8>> {
    primary_screen(reader, limits)
}
