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
    /// OSC 7501: what a program reports about itself, raw (everything after `7501;`), and how it
    /// ended (the protocol limits the length including the terminator).
    ProgramStatus {
        /// Everything after `7501;`.
        payload: Vec<u8>,
        /// How it ended.
        terminator: Terminator,
    },
    /// OSC 1337: the parts of iTerm2's protocol that are not inline images (marks, the current
    /// directory, …), raw, everything after `1337;`.
    Iterm(Vec<u8>),
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
    /// RIS: the terminal was reset to its initial state.
    FullReset,
}
