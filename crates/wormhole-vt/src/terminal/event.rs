//! What the terminal tells its host besides the screen: the events a program's sequences cause.
//!
//! Events are queued in the order the sequences came and taken with [`Terminal::events`]
//! (the queue keeps its capacity, so a bell or a mark allocates nothing; the events that carry
//! text or bytes own them). Replies to the program are not events: they are bytes for the PTY, see
//! [`Terminal::replies`].
//!
//! [`Terminal`]: crate::terminal::Terminal
//! [`Terminal::events`]: crate::terminal::Terminal::events
//! [`Terminal::replies`]: crate::terminal::Terminal::replies

use super::graphics::ImageUpload;
use crate::grid::DroppedHistory;
use crate::types::ImageHandle;

/// How an OSC string ended: a reply ends the same way (xterm), and OSC 7501 limits the length of
/// the whole sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminator {
    /// BEL (0x07).
    Bel,
    /// ST (`ESC \`).
    St,
}

impl Terminator {
    /// The bytes that end a reply the same way: BEL, or `ESC \`.
    pub fn as_str(self) -> &'static str {
        match self {
            Terminator::Bel => "\x07",
            Terminator::St => "\x1b\\",
        }
    }
}

/// How far along a task is (OSC 9;4, ConEmu's progress report, which Windows Terminal and
/// Ghostty speak too).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgressState {
    /// Going well; the percentage says how far.
    Normal,
    /// Failed or in error; the percentage is where it stopped.
    Error,
    /// Working with no way to say how far.
    Indeterminate,
    /// Waiting.
    Paused,
}

/// A program's progress: the state and a percentage (0 when the state has none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// How the task is doing.
    pub state: ProgressState,
    /// 0 to 100; 0 when the state has no percentage.
    pub percent: u8,
}

/// Which prompt an OSC 133 `A` mark starts (its `k=` option): the one a command is typed at, or
/// one of the extra prompts around it, which belong to the same command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PromptKind {
    /// The prompt a command is typed at: a new block starts.
    Initial,
    /// A later line of a multi-line command (`k=s`).
    Secondary,
    /// A right-aligned prompt (`k=r`).
    Right,
    /// A continuation line (`k=c`).
    Continuation,
}

/// The shell's marks (OSC 133, FinalTerm's semantic prompts). The terminal has already written
/// them into the grid (every cell printed afterwards is a prompt, command or output cell of the
/// block); the event is for whoever keeps a log of the commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShellMark {
    /// `A`: a prompt starts.
    PromptStart(PromptKind),
    /// `B`: the prompt ended, the command line follows.
    CommandStart,
    /// `C`: the command runs, its output follows.
    OutputStart,
    /// `D[;code]`: the command finished.
    Finished(Option<i32>),
}

/// Something a program's sequences ask of the host. See the module docs.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// BEL.
    Bell,
    /// OSC 0 or 2, or a title popped off the stack: read the new one with
    /// [`Terminal::title`](crate::terminal::Terminal::title) (`None` when it was reset: an empty title).
    Title,
    /// OSC 1: the icon name.
    IconName(String),
    /// OSC 7 (or ConEmu's OSC 9;9): the shell said where it is. Every report is kept, not just
    /// the last of a read ([`Terminal::cwd_reports`](crate::terminal::Terminal::cwd_reports)): the host
    /// decides which are folders it can show, and one it refuses (another machine's, over ssh)
    /// must not hide an earlier one it takes. [`Terminal::cwd`](crate::terminal::Terminal::cwd) is the latest.
    Cwd,
    /// OSC 9 (iTerm2's, `title` empty) or OSC 777 `notify`. Raw: the host cleans the text.
    Notification {
        /// Empty for OSC 9, which has only a body.
        title: String,
        /// What the program said.
        body: String,
    },
    /// OSC 99 (kitty's notifications): the metadata and the payload, raw. The host assembles
    /// chunks.
    KittyNotification {
        /// The `key=value` pairs before the `;`, joined by `:`.
        metadata: String,
        /// The text after it.
        payload: String,
    },
    /// OSC 9;4. `None` removes the progress.
    Progress(Option<Progress>),
    /// OSC 52 set: put `text` on the clipboard `selection` names (`c`, `p`, `s`, `0`..`7`, as
    /// sent; `s0`, xterm's default, when the program named none).
    ClipboardStore {
        /// Which clipboard, as sent.
        selection: String,
        /// What to copy (base64 decoded; invalid UTF-8 as U+FFFD).
        text: String,
    },
    /// OSC 52 query: reply with [`Terminal::clipboard_reply`](crate::terminal::Terminal::clipboard_reply).
    ClipboardLoad {
        /// Which clipboard, as sent.
        selection: String,
        /// How the request ended (the reply ends the same way).
        terminator: Terminator,
        /// How many bytes of [`Terminal::replies`](crate::terminal::Terminal::replies) the program was owed
        /// before this request: the host sends those first, then holds a place for the clipboard's
        /// answer, so the program reads its answers in the order it asked.
        replies_before: usize,
    },
    /// OSC 133 (the grid has them as well).
    Mark(ShellMark),
    /// OSC 7501, the [program status protocol](https://mitchellh.com/writing/program-status-osc7501):
    /// what a program reports about itself, raw (everything after `7501;`), and how it ended (the
    /// protocol limits the length including the terminator).
    ProgramStatus {
        /// Everything after `7501;`.
        payload: Vec<u8>,
        /// How it ended.
        terminator: Terminator,
    },
    /// OSC 1337: the parts of iTerm2's protocol that are not inline images (marks, the current
    /// directory, `SetUserVar`, …), raw, everything after `1337;`. At most 64 KiB of them; a
    /// longer one is dropped.
    Iterm(Vec<u8>),
    /// An OSC string the host asked for with a [`Passthrough`](crate::passthrough::Passthrough):
    /// `OSC number ; payload`, whole, raw (the payload is what a program printed: check it).
    ///
    /// To answer, write the reply after the first `replies_before` bytes of
    /// [`Terminal::replies`](crate::terminal::Terminal::replies) that the program was owed before
    /// this sequence went out (the way an OSC 52 read is answered), ended as the request was
    /// ([`Terminator::as_str`]), so the program reads its answers in the order it asked.
    Osc {
        /// The command number.
        number: u32,
        /// Everything after the `;` (empty without one), at most the limit asked for.
        payload: Vec<u8>,
        /// How the string ended.
        terminator: Terminator,
        /// How many bytes of the replies were queued before this sequence.
        replies_before: usize,
    },
    /// A DCS string the host asked for with a [`Passthrough`](crate::passthrough::Passthrough):
    /// `DCS params intermediates final data ST`, whole. See [`Event::Osc`] on answering.
    Dcs {
        /// The numbers before the intermediates (an empty one reads 0).
        params: Vec<u16>,
        /// The intermediate bytes, 0x20..=0x2F.
        intermediates: Vec<u8>,
        /// The final byte.
        final_byte: u8,
        /// The data between the header and the string terminator, at most the limit asked for.
        payload: Vec<u8>,
        /// How many bytes of the replies were queued before this sequence.
        replies_before: usize,
    },
    /// An APC string the host asked for with a [`Passthrough`](crate::passthrough::Passthrough):
    /// `APC payload ST`, whole, the first byte (the one that was asked for) included. See
    /// [`Event::Osc`] on answering.
    Apc {
        /// The whole string, at most the limit asked for.
        payload: Vec<u8>,
        /// How many bytes of the replies were queued before this sequence.
        replies_before: usize,
    },
    /// OSC 22: a program asked for a mouse pointer shape. The name is a CSS `cursor` keyword
    /// (`pointer`, `text`, `crosshair`, `ew-resize`, …; the X11 names xterm programs use are
    /// translated, anything else is dropped), or empty for "back to the host's own pointer".
    PointerShape(String),
    /// iTerm2's `OSC 1337 ; SetUserVar=name=base64`, decoded: a program's named value for the
    /// host (WezTerm-aware tools set the running command, the host name, …). The name is at most
    /// 256 bytes of printable text, the value at most 64 KiB; a pair that does not fit is dropped.
    /// At most 1 MiB of values wait in the queue for a host that does not take the events.
    UserVar {
        /// The variable's name.
        name: String,
        /// Its value, base64 decoded.
        value: Vec<u8>,
    },
    /// A picture to decode and keep (kitty's graphics protocol, Sixel, iTerm2's inline images).
    /// The host answers with [`Terminal::image_decoded`](crate::terminal::Terminal::image_decoded).
    ImageData(ImageUpload),
    /// A picture is gone (deleted, replaced, evicted, or its screen was left): the host drops the
    /// pixels it kept under the handle.
    ImageFree(ImageHandle),
    /// DECCOLM changed the width of the screen (mode 40 allows it): the host should resize the
    /// window to this many columns, or take the terminal back to its own size with
    /// [`Terminal::resize`](crate::terminal::Terminal::resize).
    ColumnsRequested(usize),
    /// A program asked the window to do something (and the host allowed it, see
    /// [`Policy::window_ops`](crate::terminal::Policy::window_ops)). The sizes in cells have taken effect in the engine
    /// already; the host makes the window match, or calls [`Terminal::resize`](crate::terminal::Terminal::resize)
    /// with the size it has.
    WindowOp(super::WindowOp),
    /// A mode the host acts on changed: the mouse tracking or encoding, the alternate screen,
    /// bracketed paste, focus reports, the application cursor keys and keypad, DECBKM, the kitty
    /// keyboard flags, reverse video, the cursor's visibility, shape or blink, synchronized output,
    /// mode 2031 or the DEC locator. Read the new state with
    /// [`Terminal::modes`](crate::terminal::Terminal::modes) and
    /// [`Terminal::cursor`](crate::terminal::Terminal::cursor).
    ///
    /// At most once for each call of [`Terminal::events`](crate::terminal::Terminal::events), after
    /// the other events: the engine compares the state with the state the last call saw, so a mode
    /// set and reset again in between is no change, any number of feeds is one event, and
    /// nothing is done while a program prints.
    ModesChanged,
    /// The colors programs set over the host's palette changed (OSC 4, 5, 10 to 19 and their
    /// resets 104, 105 and 110 to 119, or RIS): read them with
    /// [`Terminal::colors`](crate::terminal::Terminal::colors). At most once for each call of
    /// [`Terminal::events`](crate::terminal::Terminal::events), like `ModesChanged`. The
    /// palette the host gave with [`Terminal::set_default_colors`](crate::terminal::Terminal::set_default_colors)
    /// is not counted.
    ColorsChanged,
    /// RIS: the terminal was reset to its initial state.
    FullReset,
    /// History lines left the engine, oldest first: the host asked for them with
    /// [`Terminal::set_archive_history`](crate::terminal::Terminal::set_archive_history) and keeps them if it
    /// wants more history than the engine holds. Never lost without a trace: when the host left too
    /// many undrained, [`DroppedHistory::skipped`] counts the ones that were not kept. Events of
    /// this kind come after the others of the same [`events`](crate::terminal::Terminal::events) call, in the
    /// order the lines left, and are not subject to the queue's cap (the backlog has its own,
    /// [`Limits::archive_backlog`](crate::terminal::Limits::archive_backlog)).
    HistoryDropped(DroppedHistory),
}
