//! The `Terminal` object: a program in a PTY on the real engine, as Swift holds it.
//!
//! Threads: the PTY's reader thread and the event thread are Rust's and run on their own. Swift
//! calls in from the main thread (`frame`, `write`, `resize`); the one lock they share with the
//! reader is the engine's fair lock, taken inside `Pty::snapshot` for a copy of the damaged rows.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Instant;

use wormhole_core::terminal::{SpawnOptions, spawn};
use wormhole_term::input::keys::{self, KeyInput};
use wormhole_term::types::Scroll;
use wormhole_term::{GridSize, Pty, ScreenSnapshot};

use crate::error::FfiError;
use crate::events::{EventQueue, TermEvent, TerminalListener, spawn_receiver};
use crate::{frame, isolation};

/// How much history a terminal keeps (the app's default is the same order).
const SCROLLBACK_LINES: usize = 10_000;

/// A terminal: a program in a PTY, its screen and its events.
#[derive(uniffi::Object)]
pub struct Terminal {
    /// `None` after `close`. Write-locked only by `resize` and `close`.
    pty: RwLock<Option<Pty>>,
    /// The copy `frame` reads the screen into; the engine's damage is consumed by it, so there
    /// is exactly one.
    snapshot: Mutex<ScreenSnapshot>,
    events: Arc<EventQueue>,
    timing: Timing,
}

/// Nanoseconds spent in the last and in all `frame` calls, by part.
#[derive(Default)]
struct Timing {
    frames: AtomicU64,
    last_snapshot_ns: AtomicU64,
    last_pack_ns: AtomicU64,
    total_snapshot_ns: AtomicU64,
    total_pack_ns: AtomicU64,
    last_bytes: AtomicU64,
}

/// A key press, named as the app's key table names keys ("enter", "left", "f5", "a").
#[derive(Debug, Clone, uniffi::Record)]
pub struct KeyPress {
    pub key: String,
    /// What the key types with the layout and modifiers; `None` when it types nothing (Control held).
    pub text: Option<String>,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    /// Cmd: the application's own shortcut, nothing is sent.
    pub platform: bool,
    /// Option is Meta: Option+key sends ESC and the key.
    pub option_as_meta: bool,
}

/// Where a frame's time went on the Rust side of the seam (the Swift side measures the whole call).
#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct FrameTiming {
    pub frames: u64,
    pub last_snapshot_ns: u64,
    pub last_pack_ns: u64,
    pub total_snapshot_ns: u64,
    pub total_pack_ns: u64,
    pub last_bytes: u64,
}

#[uniffi::export]
impl Terminal {
    /// Start `program` with `args` in a PTY of `cols` x `rows` cells (`cell_width` x `cell_height`
    /// points, which the program may ask for). Refuses unless the process is an isolated instance
    /// of the experiment. Blocks briefly (it spawns): call it off the main thread in a real app.
    #[uniffi::constructor]
    pub fn start(
        program: String,
        args: Vec<String>,
        cols: u16,
        rows: u16,
        cell_width: f32,
        cell_height: f32,
        listener: Arc<dyn TerminalListener>,
    ) -> Result<Arc<Self>, FfiError> {
        isolation::require()?;
        let name = program.rsplit('/').next().unwrap_or(&program).to_string();
        let options = SpawnOptions {
            tab_id: "swift-experiment".into(),
            scrollback_lines: SCROLLBACK_LINES,
            shell: Some(wormhole_core::terminal::shells::ShellCommand { program, args, name }),
            ..SpawnOptions::default()
        };
        let size = GridSize { cols, rows, cell_width, cell_height };
        let (pty, pty_events) =
            spawn(size, &options).map_err(|e| FfiError::Start { reason: format!("{e:#}") })?;
        let events = EventQueue::new(listener);
        spawn_receiver(pty_events, events.clone()).map_err(|e| FfiError::Start { reason: e.to_string() })?;
        Ok(Arc::new(Self {
            pty: RwLock::new(Some(pty)),
            snapshot: Mutex::new(ScreenSnapshot::default()),
            events,
            timing: Timing::default(),
        }))
    }

    /// The rows that changed since the last call, packed (`frame.rs` has the layout); every row
    /// the first time and after a resize. Empty after `close`.
    pub fn frame(&self) -> Vec<u8> {
        self.pack_frame(false)
    }

    /// Every row, whatever changed: for a renderer that lost its picture (a new surface).
    pub fn full_frame(&self) -> Vec<u8> {
        self.pack_frame(true)
    }

    /// Everything queued since the last drain; re-arms the wake.
    pub fn drain_events(&self) -> Vec<TermEvent> {
        self.events.drain()
    }

    /// Typed or pasted bytes for the program (the view goes back to the live screen).
    pub fn write(&self, bytes: Vec<u8>) {
        if let Some(pty) = self.pty.read().unwrap_or_else(PoisonError::into_inner).as_ref() {
            pty.write_input(bytes);
        }
    }

    /// A key press (see [`KeyPress`]). Whether anything was sent.
    pub fn send_key(&self, press: KeyPress) -> bool {
        let guard = self.pty.read().unwrap_or_else(PoisonError::into_inner);
        let Some(pty) = guard.as_ref() else { return false };
        let input = KeyInput {
            key: &press.key,
            key_char: press.text.as_deref(),
            shift: press.shift,
            alt: press.alt,
            control: press.control,
            platform: press.platform,
            option_as_meta: press.option_as_meta,
        };
        matches!(pty.send_key(|modes| keys::encode(&input, modes)), wormhole_term::KeySent::Sent { .. })
    }

    /// The grid's new size; the program is told through the PTY and the engine reflows.
    pub fn resize(&self, cols: u16, rows: u16, cell_width: f32, cell_height: f32) {
        if let Some(pty) = self.pty.write().unwrap_or_else(PoisonError::into_inner).as_mut() {
            pty.resize(GridSize { cols, rows, cell_width, cell_height });
        }
    }

    /// Scroll the view through the history, `lines` up (positive) or down (negative); the next
    /// `frame` is complete. Typing returns the view to the live screen.
    pub fn scroll(&self, lines: i32) {
        if let Some(pty) = self.pty.read().unwrap_or_else(PoisonError::into_inner).as_ref() {
            pty.scroll(Scroll::Delta(lines));
        }
    }

    /// Back to the live screen.
    pub fn scroll_to_bottom(&self) {
        if let Some(pty) = self.pty.read().unwrap_or_else(PoisonError::into_inner).as_ref() {
            pty.scroll(Scroll::Bottom);
        }
    }

    /// The visible screen as plain text.
    pub fn screen_text(&self) -> String {
        self.pty
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(Pty::screen_text)
            .unwrap_or_default()
    }

    /// Whether the program still runs.
    pub fn is_alive(&self) -> bool {
        self.pty.read().unwrap_or_else(PoisonError::into_inner).as_ref().is_some_and(Pty::is_alive)
    }

    /// End the program (hangup, with the engine's grace period) and drop the PTY. Explicit: a Swift
    /// `deinit` only frees.
    pub fn close(&self) {
        let pty = self.pty.write().unwrap_or_else(PoisonError::into_inner).take();
        drop(pty);
    }

    /// The Rust side's share of the frames so far.
    pub fn frame_timing(&self) -> FrameTiming {
        let t = &self.timing;
        FrameTiming {
            frames: t.frames.load(Ordering::Relaxed),
            last_snapshot_ns: t.last_snapshot_ns.load(Ordering::Relaxed),
            last_pack_ns: t.last_pack_ns.load(Ordering::Relaxed),
            total_snapshot_ns: t.total_snapshot_ns.load(Ordering::Relaxed),
            total_pack_ns: t.total_pack_ns.load(Ordering::Relaxed),
            last_bytes: t.last_bytes.load(Ordering::Relaxed),
        }
    }
}

impl Terminal {
    fn pack_frame(&self, all: bool) -> Vec<u8> {
        let guard = self.pty.read().unwrap_or_else(PoisonError::into_inner);
        let Some(pty) = guard.as_ref() else { return Vec::new() };
        let mut snapshot = self.snapshot.lock().unwrap_or_else(PoisonError::into_inner);

        let started = Instant::now();
        pty.snapshot(&mut snapshot);
        let snapshotted = Instant::now();
        let bytes = frame::pack(&snapshot, all);
        let packed = Instant::now();

        let t = &self.timing;
        let (snapshot_ns, pack_ns) = (nanos(snapshotted - started), nanos(packed - snapshotted));
        t.frames.fetch_add(1, Ordering::Relaxed);
        t.last_snapshot_ns.store(snapshot_ns, Ordering::Relaxed);
        t.last_pack_ns.store(pack_ns, Ordering::Relaxed);
        t.total_snapshot_ns.fetch_add(snapshot_ns, Ordering::Relaxed);
        t.total_pack_ns.fetch_add(pack_ns, Ordering::Relaxed);
        t.last_bytes.store(bytes.len() as u64, Ordering::Relaxed);
        bytes
    }
}

fn nanos(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
