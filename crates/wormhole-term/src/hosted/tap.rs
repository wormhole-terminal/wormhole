//! What the PTY thread and the connections share on a hosted terminal's host side: the feed that
//! parses and forwards, the tap that hears the program's events, and the OSC 52 tickets.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wormhole_vt::types::TermMode;
use wormhole_wire as wire;

use super::event::{HostEvent, window_op_to_wire};
use super::memory::Memory;
use super::stream::{Stream, Wake};
use crate::engine::OwnFeed;
use crate::events::{Heard, Reply, Tap, TextFormatter};
use crate::osc::{Mark, ProgressState, PromptKind};
use crate::pty::Feed;
use crate::screen::Screen;

/// How long a program waits for the clipboard before it is told there is nothing in it.
pub const CLIPBOARD_TIMEOUT: Duration = Duration::from_secs(2);

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

struct PendingLoad {
    ticket: u32,
    format: TextFormatter,
    reply: Reply,
    deadline: Instant,
}

#[derive(Default)]
struct Clipboard {
    next: u32,
    pending: Vec<PendingLoad>,
}

/// The shared state of one hosted terminal's host side.
pub(super) struct Core {
    pub stream: Arc<Stream>,
    /// The stream offset at the end of the slice being parsed: what the events of that slice
    /// happened "at".
    slice_end: AtomicU64,
    /// The parser and the terminal are between sequences (set under the screen's lock).
    pub ready: AtomicBool,
    pub history_lines: AtomicU32,
    pub screen_rows: AtomicU32,
    pub alt_screen: AtomicBool,
    /// Bumped on every slice parsed: "host alive but the parser stuck" shows as a counter that
    /// stops.
    pub progress: AtomicU64,
    pub epoch: AtomicU32,
    /// Input bytes the master has taken in all.
    pub input_written: AtomicU64,
    /// The reader thread is over (the program exited and was reported, or was hung up).
    pub finished: AtomicBool,
    pub exit: Mutex<Option<wire::ExitInfo>>,
    /// What a client that attaches later is told (title, folder, reports). Updated under the
    /// screen's lock, as the events are heard; a leaf lock.
    memory: Mutex<Memory>,
    /// A controller is attached to answer the clipboard.
    controller: AtomicBool,
    clipboard: Mutex<Clipboard>,
    /// Tells whoever supervises the host that something it waits for happened.
    wake: Wake,
}

impl Core {
    pub fn new(wake: Wake, rows: u16) -> Self {
        Self {
            stream: Arc::new(Stream::default()),
            slice_end: AtomicU64::new(0),
            ready: AtomicBool::new(true),
            history_lines: AtomicU32::new(0),
            screen_rows: AtomicU32::new(u32::from(rows)),
            alt_screen: AtomicBool::new(false),
            progress: AtomicU64::new(0),
            epoch: AtomicU32::new(0),
            input_written: AtomicU64::new(0),
            finished: AtomicBool::new(false),
            exit: Mutex::new(None),
            memory: Mutex::new(Memory::default()),
            controller: AtomicBool::new(false),
            clipboard: Mutex::new(Clipboard::default()),
            wake,
        }
    }

    pub fn wake(&self) {
        (self.wake)();
    }

    fn memory(&self) -> MutexGuard<'_, Memory> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The events that tell a late client the title, folder and reports (see [`Memory`]).
    pub fn remembered(&self) -> Vec<HostEvent> {
        self.memory().events()
    }

    fn clipboard(&self) -> MutexGuard<'_, Clipboard> {
        self.clipboard.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What the screen looks like to the counters, after a slice.
    pub fn publish(&self, screen: &Screen, ready: bool) {
        self.history_lines.store(screen.history_size().min(u32::MAX as usize) as u32, Ordering::Release);
        self.screen_rows.store(screen.screen_lines() as u32, Ordering::Release);
        self.alt_screen.store(screen.mode().contains(TermMode::ALT_SCREEN), Ordering::Release);
        self.ready.store(ready, Ordering::Release);
        self.progress.fetch_add(1, Ordering::AcqRel);
    }

    pub fn finish(&self) {
        self.finished.store(true, Ordering::Release);
        self.cancel_clipboard();
        self.wake();
    }

    // ---- the clipboard (OSC 52 read) ----

    pub fn set_controller_attached(&self, attached: bool) {
        self.controller.store(attached, Ordering::Release);
        if !attached {
            self.cancel_clipboard();
        }
    }

    fn clipboard_load(&self, format: &TextFormatter, reply: &Reply, at: u64) {
        if !self.controller.load(Ordering::Acquire) {
            reply.send(format("").into_bytes());
            return;
        }
        let ticket = {
            let mut clipboard = self.clipboard();
            let ticket = clipboard.next;
            clipboard.next = clipboard.next.wrapping_add(1);
            clipboard.pending.push(PendingLoad {
                ticket,
                format: format.clone(),
                reply: reply.clone(),
                deadline: Instant::now() + CLIPBOARD_TIMEOUT,
            });
            ticket
        };
        let selection = selection_of(format);
        self.stream.push_event(at, HostEvent::ClipboardLoad { ticket, selection });
        // The supervisor has a deadline to keep.
        self.wake();
    }

    /// The controller's answer to `ClipboardLoad{ticket}`; a ticket answered or expired already is
    /// ignored. Empty data is "nothing to give".
    pub fn clipboard_reply(&self, ticket: u32, data: &[u8]) {
        let found = {
            let mut clipboard = self.clipboard();
            clipboard.pending.iter().position(|p| p.ticket == ticket).map(|at| clipboard.pending.remove(at))
        };
        if let Some(pending) = found {
            let text = String::from_utf8_lossy(data);
            pending.reply.send((pending.format)(&text).into_bytes());
        }
    }

    /// Answer "empty" to every load whose time is up; when the next one is due.
    pub fn clipboard_expire(&self, now: Instant) -> Option<Instant> {
        let (expired, next) = {
            let mut clipboard = self.clipboard();
            let (expired, rest): (Vec<_>, Vec<_>) =
                std::mem::take(&mut clipboard.pending).into_iter().partition(|p| p.deadline <= now);
            clipboard.pending = rest;
            (expired, clipboard.pending.iter().map(|p| p.deadline).min())
        };
        for pending in expired {
            pending.reply.send((pending.format)("").into_bytes());
        }
        next
    }

    /// Answer "empty" to every load (the controller is gone).
    pub fn cancel_clipboard(&self) {
        let all = std::mem::take(&mut self.clipboard().pending);
        for pending in all {
            pending.reply.send((pending.format)("").into_bytes());
        }
    }

    fn exited(&self, code: Option<i32>, signal: Option<i32>) {
        let info = wire::ExitInfo {
            // A program that was killed has no code; one the system could not tell about has -1.
            code: code.unwrap_or(if signal.is_some() { 0 } else { -1 }),
            signal: signal.map_or(0, |s| s.clamp(0, 255) as u8),
            at_ms: now_ms(),
        };
        *self.exit.lock().unwrap_or_else(PoisonError::into_inner) = Some(info);
        // As a local terminal's `Shared` ends what a program reported with its process.
        self.memory().program_over();
        self.stream.push_event(self.stream.total(), HostEvent::Exited(info));
        self.wake();
    }
}

/// The selection letter an OSC 52 read asked for, read back from the formatter's reply
/// (`ESC ] 52 ; <selection> ; <data> <end>`); `c` when it cannot tell.
fn selection_of(format: &TextFormatter) -> u8 {
    let probe = format("");
    probe
        .strip_prefix("\x1b]52;")
        .and_then(|rest| rest.split(';').next())
        .and_then(|selection| selection.bytes().next())
        .unwrap_or(b'c')
}

fn mark_event(mark: Mark, prompt: u64) -> Option<HostEvent> {
    let (kind, exit_code) = match mark {
        Mark::PromptStart(PromptKind::Initial) => (wire::MarkKind::PromptStart, None),
        // The extra prompts around a command are marks on the grid (the repaint has them) and
        // change nothing a client acts on.
        Mark::PromptStart(_) => return None,
        Mark::CommandStart => (wire::MarkKind::CommandStart, None),
        Mark::OutputStart => (wire::MarkKind::OutputStart, None),
        Mark::Finished(code) => (wire::MarkKind::CommandEnd, code),
    };
    Some(HostEvent::Mark(wire::Mark { kind, block: prompt, exit_code, at_ms: now_ms() }))
}

impl Tap for Core {
    fn heard(&self, what: Heard<'_>) {
        let at = self.slice_end.load(Ordering::Acquire);
        let event = match what {
            Heard::Title(title) => {
                self.memory().set_title(title);
                HostEvent::Title(title.to_owned())
            }
            Heard::ResetTitle => {
                self.memory().clear_title();
                HostEvent::ResetTitle
            }
            Heard::Bell => HostEvent::Bell,
            // A path too long for the wire's strings is no folder worth showing.
            Heard::Cwd(path) => match path.to_str().filter(|p| p.len() <= 4096) {
                Some(path) => {
                    self.memory().set_cwd(path);
                    HostEvent::Cwd(path.to_owned())
                }
                None => return,
            },
            Heard::Mark(mark, prompt) => {
                // A new prompt: whatever program was reporting is over.
                if mark == Mark::PromptStart(PromptKind::Initial) {
                    self.memory().program_over();
                }
                match mark_event(mark, prompt) {
                    Some(event) => event,
                    None => return,
                }
            }
            Heard::Notify(notification) => HostEvent::Notify {
                source: 0,
                title: notification.title.clone(),
                body: notification.body.clone(),
            },
            Heard::Progress(progress) => match progress {
                None => HostEvent::Progress { state: wire::ProgressState::Remove, percent: 0 },
                Some(p) => HostEvent::Progress {
                    state: match p.state {
                        ProgressState::Normal => wire::ProgressState::Set,
                        ProgressState::Error => wire::ProgressState::Error,
                        ProgressState::Indeterminate => wire::ProgressState::Indeterminate,
                        ProgressState::Paused => wire::ProgressState::Paused,
                    },
                    percent: p.percent,
                },
            },
            Heard::ProgramStatus(body) => match std::str::from_utf8(body) {
                Ok(body) => {
                    self.memory().report(body);
                    HostEvent::ProgramStatus(body.to_owned())
                }
                Err(_) => return,
            },
            Heard::ClipboardStore(text) => {
                HostEvent::ClipboardStore { selection: b'c', data: text.as_bytes().to_vec() }
            }
            Heard::ClipboardLoad(format, reply) => return self.clipboard_load(format, reply, at),
            Heard::WindowOp(op) => {
                let (op, a, b) = window_op_to_wire(op);
                HostEvent::WindowOp { op, a, b }
            }
            Heard::FullReset => {
                self.memory().full_reset();
                HostEvent::FullReset
            }
            Heard::Exited { code, signal } => return self.exited(code, signal),
        };
        self.stream.push_event(at, event);
    }
}

/// What the PTY thread parses with: the engine's own feed, and then the same bytes into the
/// stream the clients read. Both happen in one critical section of the screen's lock, so a
/// repaint made under it is exactly at the stream offset the cursor registered then.
pub(super) struct HostedFeed {
    inner: OwnFeed,
    core: Arc<Core>,
    /// Bytes parsed so far.
    offset: u64,
}

impl HostedFeed {
    pub fn new(inner: OwnFeed, core: Arc<Core>) -> Self {
        Self { inner, core, offset: 0 }
    }
}

impl Feed<Screen> for HostedFeed {
    fn feed(&mut self, screen: &mut Screen, bytes: &[u8]) {
        self.offset += bytes.len() as u64;
        self.core.slice_end.store(self.offset, Ordering::Release);
        self.inner.feed(screen, bytes);
        self.core.publish(screen, self.inner.repaint_ready(screen));
        self.core.stream.append(bytes);
    }

    fn deadline(&self) -> Option<Instant> {
        self.inner.deadline()
    }

    fn expire(&mut self, screen: &mut Screen) {
        self.inner.expire(screen);
    }
}

impl Drop for HostedFeed {
    /// The reader thread owns the feed and drops it when it ends, whether the program exited or
    /// the terminal was closed and hung up: the one moment both cases have in common.
    fn drop(&mut self) {
        self.core.finish();
    }
}
