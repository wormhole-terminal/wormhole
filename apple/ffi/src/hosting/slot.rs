//! What a `Terminal` knows of the session host its program runs in, and what only a hosted terminal does.

use std::sync::atomic::{AtomicBool, Ordering};

use wormhole_term::Pty;

use super::TerminalHosting;
use crate::terminal::Terminal;

/// The host a terminal runs in.
pub(crate) struct HostSlot {
    id: String,
    /// Let go of (`Terminal.detach`): closing the terminal afterwards leaves the program running and its
    /// saved row in place.
    detached: AtomicBool,
    /// Called when the terminal is closed while still attached, which ends its program: the saved row
    /// naming the host goes with it.
    on_end: Box<dyn Fn() + Send + Sync>,
}

impl HostSlot {
    pub(crate) fn new(id: String, on_end: impl Fn() + Send + Sync + 'static) -> Self {
        Self { id, detached: AtomicBool::new(false), on_end: Box::new(on_end) }
    }

    /// The terminal is being closed: if that ends the program, forget which host ran it.
    pub(crate) fn closing(&self) {
        if !self.detached.load(Ordering::Acquire) {
            (self.on_end)();
        }
    }
}

#[uniffi::export]
impl Terminal {
    /// The session host this terminal runs in; `None` for one in the app's own process.
    pub fn host_id(&self) -> Option<String> {
        self.host.as_ref().map(|slot| slot.id.clone())
    }

    /// Where the program runs now.
    pub fn hosting(&self) -> TerminalHosting {
        let Some(slot) = &self.host else { return TerminalHosting::Local };
        let host_id = slot.id.clone();
        let lost = self.with_pty(Pty::host_lost).unwrap_or(false);
        if lost {
            TerminalHosting::Lost { host_id }
        } else if slot.detached.load(Ordering::Acquire) {
            TerminalHosting::Detached { host_id }
        } else {
            TerminalHosting::Persistent { host_id }
        }
    }

    /// Let go of the session host and leave the program running in it for the next attach; closing or
    /// dropping the terminal afterwards does not end it. What the app does for every hosted terminal as it
    /// quits (`HostedTerminals.finish`). Whether there was a host to let go of.
    pub fn detach(&self) -> bool {
        let Some(slot) = &self.host else { return false };
        let detached = self.with_pty(Pty::detach).unwrap_or(false);
        if detached {
            slot.detached.store(true, Ordering::Release);
        }
        detached
    }

    /// On screen (fed a repaint, then the stream) or not (headless: the screen keeps what it last showed).
    /// Nothing for a terminal in the app's own process.
    pub fn set_visible(&self, visible: bool) {
        self.with_pty(|pty| pty.set_visible(visible));
    }
}
