//! What a hostile program cannot make the terminal hold: the caps on strings, pictures and queues,
//! and the host's say in them.
//!
//! The engine has a built-in ceiling for each (the constants the modules own: `IMAGE_BUDGET_BYTES`,
//! `MAX_IMAGES`, the OSC string sizes, …). A host that has no use for a picture of 100 MiB, or
//! that wants a title no longer than its window can show, lowers the cap with [`Limits`]; it can
//! never raise one above the ceiling, so the guarantees in `docs/security.md` hold whatever the
//! host asks. The caps are read where a string ends or an allocation is made, never per byte.

use super::graphics::{IMAGE_BUDGET_BYTES, MAX_IMAGES};
use super::{CWD_MAX, EVENTS_MAX, TITLE_MAX, Terminal, osc, reports};

/// Most bytes of a hyperlink's target. An OSC string is at most 8 KiB anyway; the cap exists so a
/// host can lower it.
const LINK_URI_MAX: usize = 8 * 1024;
/// Most bytes of dropped history lines held for a host that does not take them (see
/// [`Builder::archive_history`](crate::Builder::archive_history)). A line costs 16 bytes a column.
const ARCHIVE_BACKLOG_MAX: usize = 64 << 20;

/// The caps a host can lower: how much a program's sequences can make the terminal hold.
///
/// [`Default`] is the built-in ceiling of every value, which is what a terminal uses unless told
/// otherwise. A value above its ceiling is cut to it (see [`clamped`](Self::clamped)), so a host
/// can only make the terminal stricter, and a limit added in a later version starts at its ceiling
/// for a host that does not set it. The struct is `#[non_exhaustive]`: start from
/// [`Limits::default`] and set the fields.
///
/// What a string over its cap does is the same everywhere: it is dropped whole, never cut (a
/// title and a folder are the exceptions: they are cut at a character).
///
/// ```
/// use wormhole_vt::{Limits, Terminal};
///
/// let mut limits = Limits::default();
/// limits.image_bytes = 16 << 20; // pictures: 16 MiB of pixels, not 128
/// limits.clipboard = 64 * 1024;  // OSC 52: 64 KiB
/// limits.events = u32::MAX as usize; // above the ceiling: cut to it
/// let term = Terminal::builder().limits(limits).build();
/// assert_eq!(term.limits().image_bytes, 16 << 20);
/// assert_eq!(term.limits().events, Limits::default().events);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// Bytes the pictures of one screen may take together, as RGBA once decoded (ceiling:
    /// [`IMAGE_BUDGET_BYTES`](crate::image::IMAGE_BUDGET_BYTES), 128 MiB). A picture over it is
    /// refused; the least recently used ones leave to make room for a new one. Lowered while
    /// pictures are held, the surplus is evicted at once (each as an
    /// [`Event::ImageFree`](crate::Event::ImageFree)).
    pub image_bytes: u64,
    /// Pictures one screen keeps (ceiling: [`MAX_IMAGES`](crate::image::MAX_IMAGES), 1,024).
    /// Lowering it evicts like `image_bytes`.
    pub image_count: usize,
    /// Bytes of pictures waiting in the event queue for the host to decode, in all (ceiling:
    /// 256 MiB). Past it new pictures are refused.
    pub image_uploads: usize,
    /// Bytes one picture's data may be once decoded from base64: a kitty transmission, however many
    /// chunks, or one iTerm2 file (ceiling: 32 MiB).
    pub image_payload: usize,
    /// Bytes of a Sixel's data (ceiling: 4 MiB).
    pub sixel: usize,
    /// Bytes of an OSC string that is small by nature: titles, folders, colours, hyperlinks,
    /// notifications, shell marks, program status (ceiling: 8 KiB, the number and its `;`
    /// included).
    pub osc: usize,
    /// Bytes of an OSC 52 string, the clipboard (ceiling: 8 MiB, `52;c;` and the base64 text
    /// included).
    pub clipboard: usize,
    /// Bytes of an OSC 1337 string that carries a picture (ceiling: 64 MiB of base64).
    pub osc_image: usize,
    /// Bytes of an OSC 1337 string that is not a picture, which reaches the host as
    /// [`Event::Iterm`](crate::Event::Iterm) (ceiling: 64 KiB).
    pub osc_iterm: usize,
    /// Bytes of a window title or icon name kept, cut at a character (ceiling: 4,096).
    pub title: usize,
    /// Bytes of the folder OSC 7 reports kept, cut at a character (ceiling: 4,096).
    pub cwd: usize,
    /// Bytes of a hyperlink's target; a longer one is ignored like one with a control character
    /// (ceiling: 8 KiB).
    pub link_uri: usize,
    /// Events the host may leave undrained: past it new ones are dropped, except pictures' pixels
    /// and their release, resets and dropped history (ceiling: 16,384).
    pub events: usize,
    /// Reply bytes the host may leave uncollected: past it new replies are dropped whole (ceiling:
    /// 1 MiB).
    pub replies: usize,
    /// Bytes of dropped history lines (16 a column, and the line's side data) held for a host
    /// that does not take its events, when [`Builder::archive_history`](crate::Builder::archive_history)
    /// is on; past it the newest lines are counted, not kept (ceiling: 64 MiB).
    pub archive_backlog: usize,
}

impl Limits {
    /// The built-in ceilings.
    const CEILING: Limits = Limits {
        image_bytes: IMAGE_BUDGET_BYTES,
        image_count: MAX_IMAGES,
        image_uploads: super::graphics::QUEUED_BYTES_MAX,
        image_payload: super::graphics::PAYLOAD_MAX,
        sixel: super::dispatch::dcs::SIXEL_MAX,
        osc: osc::SMALL,
        clipboard: osc::CLIPBOARD,
        osc_image: osc::IMAGE,
        osc_iterm: osc::ITERM_OTHER,
        title: TITLE_MAX,
        cwd: CWD_MAX,
        link_uri: LINK_URI_MAX,
        events: EVENTS_MAX,
        replies: reports::REPLIES_MAX,
        archive_backlog: ARCHIVE_BACKLOG_MAX,
    };

    /// These limits with every value above its ceiling cut to it. [`Builder::limits`] and
    /// [`Terminal::set_limits`] do this themselves; it is public to show what they will use.
    ///
    /// [`Builder::limits`]: crate::Builder::limits
    pub fn clamped(self) -> Limits {
        let c = Limits::CEILING;
        Limits {
            image_bytes: self.image_bytes.min(c.image_bytes),
            image_count: self.image_count.min(c.image_count),
            image_uploads: self.image_uploads.min(c.image_uploads),
            image_payload: self.image_payload.min(c.image_payload),
            sixel: self.sixel.min(c.sixel),
            osc: self.osc.min(c.osc),
            clipboard: self.clipboard.min(c.clipboard),
            osc_image: self.osc_image.min(c.osc_image),
            osc_iterm: self.osc_iterm.min(c.osc_iterm),
            title: self.title.min(c.title),
            cwd: self.cwd.min(c.cwd),
            link_uri: self.link_uri.min(c.link_uri),
            events: self.events.min(c.events),
            replies: self.replies.min(c.replies),
            archive_backlog: self.archive_backlog.min(c.archive_backlog),
        }
    }
}

impl Default for Limits {
    /// The built-in ceilings: what the engine allows when the host says nothing.
    fn default() -> Limits {
        Limits::CEILING
    }
}

impl Terminal {
    /// The limits in force: what the host set, cut to the ceilings.
    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Change the caps (each value cut to its ceiling, see [`Limits`]). They apply from the next
    /// sequence: a string being read keeps the cap it started with, queued events and replies stay
    /// where they are, a title already kept is not cut again. Pictures are the exception: ones
    /// held over the new `image_bytes` or `image_count` are evicted now, least recently used
    /// first (those nothing is placed from before the others), each announced by an
    /// [`Event::ImageFree`](crate::Event::ImageFree).
    pub fn set_limits(&mut self, limits: Limits) {
        self.limits = limits.clamped();
        self.apply_limits();
    }

    /// Hand the limits to the parts that keep their own copy.
    pub(super) fn apply_limits(&mut self) {
        let limits = self.limits;
        self.osc.set_caps(osc::Caps {
            small: limits.osc,
            clipboard: limits.clipboard,
            image: limits.osc_image,
            other: limits.osc_iterm,
        });
        self.dcs.set_sixel_max(limits.sixel);
        self.enforce_image_limits();
        self.set_archive_cap(limits.archive_backlog);
    }
}
