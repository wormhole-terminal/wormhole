//! Terminal events on their way from the PTY thread to the UI.
//!
//! The engine sends a wakeup per read burst: tens to hundreds a second while a
//! program streams output. Each one would be a cross-thread wake of the UI. Here
//! they collapse into one [`PtyEvent::Output`] in flight at a time (the next is
//! sent only after the UI took the previous one), and a revision counter tells
//! any reader whether output arrived since it last looked.
//!
//! Replies to terminal queries (cursor position, device attributes, text area
//! size) are written back on the PTY thread, in parse order, so a program that
//! blocks on one never waits for a busy UI thread. The few the UI answers (a
//! color, the clipboard) keep their place in that order: replies after one wait
//! for it. Programs ask for the background color and then the cursor position,
//! and take the cursor reply arriving first as "no color support"; the color
//! reply, landing after they quit, was typed at the shell's prompt.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Instant;

use super::WindowSize;
use super::blocks::CommandLog;
use super::images::{DECODES_MAX, ImageStore};
use super::osc::{Mark, Notification, Osc, Progress, ProgressState, PromptKind};
use super::procinfo;
use super::program_status::{self, ProgramStatus};
use wormhole_vt::term::WindowOp;

/// Turns clipboard text into the escape sequence a program expects.
pub type TextFormatter = Arc<dyn Fn(&str) -> String + Sync + Send + 'static>;

/// What the UI hears from a terminal.
#[derive(Clone)]
pub enum PtyEvent {
    /// New content since the UI took the last `Output`.
    Output,
    Title(String),
    ResetTitle,
    Bell,
    /// OSC 52: put text on the clipboard.
    ClipboardStore(String),
    /// OSC 52 read; answer with the clipboard text run through the formatter.
    ClipboardLoad(TextFormatter, Reply),
    /// OSC 7: the shell said where it is (a local folder that exists); `Pty::cwd` has it.
    Cwd,
    /// OSC 133: the shell's prompt, command and output marks.
    Mark(Mark),
    /// OSC 9 / 99 / 777: a program's own notification.
    Notify(Notification),
    /// What programs report about themselves (OSC 7501) changed since the UI took the last one:
    /// a report, a full reset, or a prompt dropping what only the running program could say.
    /// Read it with `Pty::program_status`. Like `Output`, one is in flight at a time, so a
    /// program reporting as fast as it can writes costs the UI one look per turn.
    ProgramStatus,
    /// A program asked the window to do something and `allow_window_ops` is on: move, resize,
    /// minimise, raise, full screen (XTWINOPS), or switch to 80 or 132 columns (DECCOLM). The
    /// sizes in cells have taken effect in the engine already; the UI makes the window match and
    /// then calls [`Pty::sync_size`](super::Pty::sync_size).
    WindowOp(WindowOp),
    /// The shell exited (sent once), with its exit code when the system reported one.
    Exited(Option<i32>),
}

impl std::fmt::Debug for PtyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyEvent::Output => f.write_str("Output"),
            PtyEvent::Title(title) => write!(f, "Title({title})"),
            PtyEvent::ResetTitle => f.write_str("ResetTitle"),
            PtyEvent::Bell => f.write_str("Bell"),
            PtyEvent::ClipboardStore(_) => f.write_str("ClipboardStore"),
            PtyEvent::ClipboardLoad(..) => f.write_str("ClipboardLoad"),
            PtyEvent::Cwd => f.write_str("Cwd"),
            PtyEvent::Mark(mark) => write!(f, "Mark({mark:?})"),
            PtyEvent::Notify(notification) => write!(f, "Notify({notification:?})"),
            PtyEvent::ProgramStatus => f.write_str("ProgramStatus"),
            PtyEvent::WindowOp(op) => write!(f, "WindowOp({op:?})"),
            PtyEvent::Exited(code) => write!(f, "Exited({code:?})"),
        }
    }
}

/// Where the UI's answer to a query goes: into the place the query holds among the replies.
/// Dropped unanswered (the view is gone), it gives that place up, so the replies behind it go out.
#[derive(Clone)]
pub struct Reply(Arc<ReplySlot>);

struct ReplySlot {
    ticket: u64,
    shared: Arc<Shared>,
}

impl Reply {
    pub fn send(&self, bytes: Vec<u8>) {
        self.0.shared.fill_reply(self.0.ticket, bytes);
    }
}

impl Drop for ReplySlot {
    fn drop(&mut self) {
        // A no-op once answered: the slot is gone.
        self.shared.fill_reply(self.ticket, Vec::new());
    }
}

/// Query replies in the order the program asked for them.
#[derive(Default)]
struct Replies {
    next_ticket: u64,
    queue: VecDeque<Slot>,
}

enum Slot {
    Ready(Vec<u8>),
    /// The UI's answer to this ticket goes here.
    Waiting(u64),
}

impl Replies {
    /// A reply from the PTY thread: out now, unless a held one is ahead of it.
    fn push(&mut self, bytes: Vec<u8>) -> Option<Vec<u8>> {
        if self.queue.is_empty() {
            return Some(bytes);
        }
        self.queue.push_back(Slot::Ready(bytes));
        None
    }

    /// Hold a place for a reply the UI gives later.
    fn hold(&mut self) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.queue.push_back(Slot::Waiting(ticket));
        ticket
    }

    /// The UI's answer for `ticket`; what can go out now, in order. An unknown ticket (already
    /// answered) changes nothing.
    fn fill(&mut self, ticket: u64, bytes: Vec<u8>) -> Vec<Vec<u8>> {
        let Some(slot) = self.queue.iter_mut().find(|slot| matches!(slot, Slot::Waiting(t) if *t == ticket))
        else {
            return Vec::new();
        };
        *slot = Slot::Ready(bytes);
        let mut out = Vec::new();
        while let Some(Slot::Ready(_)) = self.queue.front() {
            if let Some(Slot::Ready(bytes)) = self.queue.pop_front() {
                out.push(bytes);
            }
        }
        out
    }
}

/// Sends bytes to the program, from any thread.
pub(super) type Writer = Box<dyn Fn(Vec<u8>) + Send + Sync>;

/// State shared by the PTY thread (writer) and the UI (reader).
#[derive(Default)]
pub(super) struct Shared {
    /// Bumped on every wakeup: "did this terminal print since X?".
    revision: AtomicU64,
    /// An `Output` is queued and the UI hasn't taken it yet.
    output_pending: AtomicBool,
    exited: AtomicBool,
    /// The grid size in cells and pixels, packed, for size queries.
    size: AtomicU64,
    /// Where query replies go (the PTY's input); set once the PTY exists.
    writer: OnceLock<Writer>,
    /// Replies waiting behind one the UI hasn't given yet.
    replies: Mutex<Replies>,
    /// OSC 7: the folder the shell last reported, until `Pty::cwd` takes it.
    cwd_report: Mutex<Option<PathBuf>>,
    /// OSC 133: where the shell is. 0 before any mark, 1 at its prompt, 2 running a command.
    prompt: AtomicU8,
    /// OSC 133 `D`: the last exit code, as `1 << 32 | code`; 0 before any.
    exit_code: AtomicU64,
    /// OSC 133: when each command started and how it ended, by prompt (the blocks' states).
    commands: Mutex<CommandLog>,
    /// OSC 9;4: the progress, as `(state + 1) << 8 | percent`; 0 when there is none.
    progress: AtomicU32,
    /// OSC 7501: what programs report, applied here on the PTY thread in the order the bytes came
    /// (so a prompt after a report drops it, and the UI never holds a queue of them).
    program_status: Mutex<ProgramStatus>,
    /// A `ProgramStatus` event is queued and the UI hasn't taken it yet.
    program_status_pending: AtomicBool,
    /// The inline images, and how many are being decoded right now.
    images: Mutex<ImageStore>,
    decoding: AtomicUsize,
}

const PROMPT_NONE: u8 = 0;
const PROMPT_AT: u8 = 1;
const PROMPT_RUNNING: u8 = 2;

impl Shared {
    pub(super) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    pub(super) fn exited(&self) -> bool {
        self.exited.load(Ordering::Acquire)
    }

    fn report_cwd(&self, path: PathBuf) {
        *self.cwd_report.lock().unwrap_or_else(PoisonError::into_inner) = Some(path);
    }

    /// The folder the shell reported since the last call, if any.
    pub(super) fn take_cwd_report(&self) -> Option<PathBuf> {
        self.cwd_report.lock().unwrap_or_else(PoisonError::into_inner).take()
    }

    fn mark(&self, mark: Mark, prompt: u64) {
        match mark {
            Mark::PromptStart(_) | Mark::CommandStart => self.prompt.store(PROMPT_AT, Ordering::Release),
            Mark::OutputStart => self.prompt.store(PROMPT_RUNNING, Ordering::Release),
            Mark::Finished(code) => {
                self.prompt.store(PROMPT_AT, Ordering::Release);
                // A `D` without a code (an empty line) leaves the last command's code.
                if let Some(code) = code {
                    self.exit_code.store(1 << 32 | u64::from(code as u32), Ordering::Release);
                }
            }
        }
        self.commands().mark(mark, prompt, Instant::now());
    }

    /// The commands' log (the lock is held for a lookup, never across a wait).
    pub(super) fn commands(&self) -> MutexGuard<'_, CommandLog> {
        self.commands.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn program_status(&self) -> MutexGuard<'_, ProgramStatus> {
        self.program_status.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The inline images (the lock is held for a lookup or an insert, never across a decode).
    pub(super) fn images(&self) -> MutexGuard<'_, ImageStore> {
        self.images.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A decode is about to start on a thread of its own; `false` when [`DECODES_MAX`] are being
    /// decoded already, in which case it must not start. Pair a `true` with [`Self::end_decode`].
    pub(super) fn begin_decode(&self) -> bool {
        if self.decoding.fetch_add(1, Ordering::AcqRel) >= DECODES_MAX {
            self.decoding.fetch_sub(1, Ordering::AcqRel);
            return false;
        }
        true
    }

    pub(super) fn end_decode(&self) {
        self.decoding.fetch_sub(1, Ordering::AcqRel);
    }

    /// Answer a program (the same way a query reply goes: on the PTY thread, in order).
    pub(super) fn write_reply(&self, bytes: Vec<u8>) {
        self.reply(bytes);
    }

    /// Whether the shell is at its prompt, as its marks say; `None` before any mark.
    pub(super) fn at_prompt(&self) -> Option<bool> {
        match self.prompt.load(Ordering::Acquire) {
            PROMPT_NONE => None,
            state => Some(state == PROMPT_AT),
        }
    }

    pub(super) fn last_exit_code(&self) -> Option<i32> {
        let packed = self.exit_code.load(Ordering::Acquire);
        (packed != 0).then_some(packed as u32 as i32)
    }

    fn set_progress(&self, progress: Option<Progress>) {
        let packed = progress.map_or(0, |p| {
            let state = match p.state {
                ProgressState::Normal => 1,
                ProgressState::Error => 2,
                ProgressState::Indeterminate => 3,
                ProgressState::Paused => 4,
            };
            state << 8 | u32::from(p.percent)
        });
        self.progress.store(packed, Ordering::Release);
    }

    pub(super) fn progress(&self) -> Option<Progress> {
        let packed = self.progress.load(Ordering::Acquire);
        let state = match packed >> 8 {
            0 => return None,
            1 => ProgressState::Normal,
            2 => ProgressState::Error,
            3 => ProgressState::Indeterminate,
            _ => ProgressState::Paused,
        };
        Some(Progress { state, percent: (packed & 0xff) as u8 })
    }

    pub(super) fn set_writer(&self, writer: Writer) {
        let _ = self.writer.set(writer);
    }

    pub(super) fn set_size(&self, size: WindowSize) {
        let packed = u64::from(size.num_lines)
            | u64::from(size.num_cols) << 16
            | u64::from(size.cell_width) << 32
            | u64::from(size.cell_height) << 48;
        self.size.store(packed, Ordering::Release);
    }

    pub(super) fn size(&self) -> WindowSize {
        let packed = self.size.load(Ordering::Acquire);
        WindowSize {
            num_lines: packed as u16,
            num_cols: (packed >> 16) as u16,
            cell_width: (packed >> 32) as u16,
            cell_height: (packed >> 48) as u16,
        }
    }

    /// The lock is held while sending, so replies go out in the order they were queued.
    fn replies(&self) -> MutexGuard<'_, Replies> {
        self.replies.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn reply(&self, bytes: Vec<u8>) {
        let mut replies = self.replies();
        if let Some(bytes) = replies.push(bytes) {
            self.send_reply(bytes);
        }
    }

    /// Hold a place among the replies for one the UI gives.
    pub(super) fn hold_reply(self: &Arc<Self>) -> Reply {
        let ticket = self.replies().hold();
        Reply(Arc::new(ReplySlot { ticket, shared: self.clone() }))
    }

    fn fill_reply(&self, ticket: u64, bytes: Vec<u8>) {
        let mut replies = self.replies();
        for bytes in replies.fill(ticket, bytes) {
            self.send_reply(bytes);
        }
    }

    fn send_reply(&self, bytes: Vec<u8>) {
        if let Some(writer) = self.writer.get()
            && !bytes.is_empty()
        {
            writer(bytes);
        }
    }
}

/// Receives the engine's events on the PTY thread (while the terminal is locked, so it must never
/// lock it again); `engine` turns the engine's own events into calls on this.
#[derive(Clone)]
pub(crate) struct EventProxy {
    tx: async_channel::Sender<PtyEvent>,
    shared: Arc<Shared>,
}

impl EventProxy {
    pub(super) fn new() -> (Self, PtyEvents) {
        let (tx, rx) = async_channel::unbounded();
        let shared = Arc::new(Shared::default());
        (Self { tx, shared: shared.clone() }, PtyEvents { rx, shared })
    }

    pub(super) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    pub(super) fn send(&self, event: PtyEvent) {
        let _ = self.tx.try_send(event);
    }

    /// The program printed (the engine sends one per read burst): bump the revision and, unless
    /// the UI hasn't taken the last one yet, say so.
    pub(super) fn wakeup(&self) {
        self.shared.revision.fetch_add(1, Ordering::AcqRel);
        if !self.shared.output_pending.swap(true, Ordering::AcqRel) {
            self.send(PtyEvent::Output);
        }
    }

    /// The shell is gone: say so once.
    pub(super) fn exited(&self, code: Option<i32>) {
        if !self.shared.exited.swap(true, Ordering::AcqRel) {
            // The process the programs' work ran in is over (the UI shows the exit itself).
            self.shared.program_status().on_process_exit();
            self.send(PtyEvent::Exited(code));
        }
    }

    /// What programs report changed: one event until the UI has looked.
    fn program_status_changed(&self) {
        if !self.shared.program_status_pending.swap(true, Ordering::AcqRel) {
            self.send(PtyEvent::ProgramStatus);
        }
    }

    /// What the host made of the engine's events (a folder, a mark, a notification, a status), on
    /// the PTY thread and never touching the terminal's lock.
    pub(super) fn osc(&self, osc: Osc) {
        match osc {
            // A folder on another machine (ssh) or one that isn't here is no folder to show;
            // polling the shell's process stays the fallback for those.
            Osc::Cwd { host, path } => {
                if procinfo::is_this_host(&host) && path.is_dir() {
                    self.shared.report_cwd(path);
                    self.send(PtyEvent::Cwd);
                }
            }
            Osc::Mark(mark, prompt) => {
                self.shared.mark(mark, prompt);
                self.send(PtyEvent::Mark(mark));
                // A new prompt: whatever program was reporting is over.
                if mark == Mark::PromptStart(PromptKind::Initial) && self.shared.program_status().on_prompt()
                {
                    self.program_status_changed();
                }
            }
            Osc::Notify(notification) => self.send(PtyEvent::Notify(notification)),
            // Nothing shows this yet. Once something does, it must stop mapping it to a status
            // after the first OSC 7501 report, until the next full reset: a mapped 9;4 would
            // replace a real report's kind and msg.
            Osc::Progress(progress) => self.shared.set_progress(progress),
            // Answered here, on the PTY thread while the read's events are acted on, so the reply
            // goes out ahead of the engine's own replies from the same read: the reply to the
            // primary device attributes a program sends right after the query must not come first,
            // or it takes the query as "not supported".
            Osc::ProgramStatusQuery => self.shared.write_reply(program_status::QUERY_REPLY.to_vec()),
            Osc::ProgramStatus(report) => {
                if self.shared.program_status().apply(report) {
                    self.program_status_changed();
                }
            }
            Osc::FullReset => {
                if self.shared.program_status().reset() {
                    self.program_status_changed();
                }
            }
        }
    }
}

/// The UI's end: taking an `Output` re-arms the next one.
pub struct PtyEvents {
    rx: async_channel::Receiver<PtyEvent>,
    shared: Arc<Shared>,
}

impl PtyEvents {
    /// The next event, or `None` once the terminal is gone.
    pub async fn recv(&self) -> Option<PtyEvent> {
        let event = self.rx.recv().await.ok()?;
        Some(self.taken(event))
    }

    /// An event that is already queued, without waiting.
    pub fn try_recv(&self) -> Option<PtyEvent> {
        self.rx.try_recv().ok().map(|event| self.taken(event))
    }

    /// Events waiting to be taken.
    pub fn queued(&self) -> usize {
        self.rx.len()
    }

    fn taken(&self, event: PtyEvent) -> PtyEvent {
        // Cleared before the UI reads the grid, so output that lands after this
        // point sends a fresh `Output` instead of being missed.
        match event {
            PtyEvent::Output => self.shared.output_pending.store(false, Ordering::Release),
            PtyEvent::ProgramStatus => self.shared.program_status_pending.store(false, Ordering::Release),
            _ => {}
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::Replies;

    #[test]
    fn replies_wait_behind_one_the_ui_has_not_given() {
        let mut replies = Replies::default();
        assert_eq!(replies.push(b"a".to_vec()), Some(b"a".to_vec()), "nothing held: out now");
        let color = replies.hold();
        assert_eq!(replies.push(b"cursor".to_vec()), None, "waits for the color");
        let clipboard = replies.hold();
        assert_eq!(replies.push(b"attributes".to_vec()), None);
        assert!(replies.fill(clipboard, b"clip".to_vec()).is_empty(), "the color is still ahead");
        assert_eq!(
            replies.fill(color, b"rgb".to_vec()),
            [b"rgb".to_vec(), b"cursor".to_vec(), b"clip".to_vec(), b"attributes".to_vec()]
        );
        assert!(replies.fill(color, Vec::new()).is_empty(), "answered already");
        assert_eq!(replies.push(b"b".to_vec()), Some(b"b".to_vec()));
    }
}
