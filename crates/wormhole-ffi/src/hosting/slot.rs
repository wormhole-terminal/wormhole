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
    /// The host ends and the terminal stays (`Terminal.end_keeping_row`: Quit and End All, Restart): its saved row, with
    /// its notes, tasks and text, is kept and no longer names the host (core `hosts::persisted_host`'s `ending`).
    kept: AtomicBool,
    /// Called when the terminal is closed on purpose while still attached, which ends its program: the saved row
    /// naming the host goes with it.
    on_end: Box<dyn Fn() + Send + Sync>,
}

impl HostSlot {
    pub(crate) fn new(id: String, on_end: impl Fn() + Send + Sync + 'static) -> Self {
        Self { id, detached: AtomicBool::new(false), kept: AtomicBool::new(false), on_end: Box::new(on_end) }
    }

    /// The terminal is being closed: if the person closed it and that ends the program, forget its row.
    pub(crate) fn closing(&self) {
        if !self.detached.load(Ordering::Acquire) && !self.kept.load(Ordering::Acquire) {
            (self.on_end)();
        }
    }
}

impl Terminal {
    /// The host the terminal's saved row names (the GPUI app's `Session::persisted_host`): its own, unless the host was
    /// lost or is ending while the terminal stays.
    pub(crate) fn persisted_host(&self) -> Option<String> {
        let slot = self.host.as_ref()?;
        let lost = self.with_pty(Pty::host_lost).unwrap_or(false);
        wormhole_core::ipc::hosts::persisted_host(Some(&slot.id), lost, slot.kept.load(Ordering::Acquire))
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

    /// End the program as `close` does, without forgetting the terminal: it stays in its window (Restart starts a new
    /// program in its place) or comes back at the next launch (Quit and End All), its saved row kept with its notes,
    /// tasks and text and naming no host. `close` is for a terminal the person closed. For a terminal in the app's own
    /// process the two are the same.
    pub fn end_keeping_row(&self) {
        if let Some(slot) = &self.host {
            slot.kept.store(true, Ordering::Release);
        }
        self.close();
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

    /// Seconds the host has not answered, once it has not for long enough, while attached (`HostResponsiveness` says when
    /// it changes): the bar offers Wait or End. `None` for a host that answers, one let go of, and a terminal in the app.
    pub fn host_not_responding(&self) -> Option<f64> {
        let stats = self.with_pty(Pty::link_stats).flatten()?;
        let attached = !stats.detached && !stats.ended && !stats.lost;
        stats.not_responding_for.filter(|_| attached).map(|silent| silent.as_secs_f64())
    }

    /// How the connection to the host stands, for the Session Hosts window; `None` for a terminal in the app's own process and
    /// for one whose connection is not up.
    pub fn host_link(&self) -> Option<crate::aux_backends::HostLinkRecord> {
        self.host.as_ref()?;
        self.with_pty(Pty::link_stats).flatten().map(Into::into)
    }

    /// On screen (fed a repaint, then the stream) or not (headless: the screen keeps what it last showed).
    /// Nothing for a terminal in the app's own process.
    pub fn set_visible(&self, visible: bool) {
        self.with_pty(|pty| pty.set_visible(visible));
    }
}
