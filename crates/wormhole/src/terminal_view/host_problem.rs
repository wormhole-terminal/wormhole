//! A terminal whose session host cannot be shown: it runs but this app could not attach to it (it refused the
//! app, speaks another protocol, is busy or slow), or it stopped answering. The terminal keeps the host's id
//! (the program is in the host) and says why in the bar over its pane (`render/pane_bar.rs`), where the person
//! can try again, wait, open a new terminal in its place, or end the host. This file is the state and the
//! actions; the words of the bar are the catalog's.
//!
//! Ending a host takes seconds (`Kill` over its socket, then a signal) and can fail; the terminal closes only
//! when it worked (`TerminalEvent::HostEnded`, which the window answers by closing that session by its id,
//! shown or not), and says why when it did not.

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::ipc::hosts::{self, Unavailable, UnavailableKind};
use wormhole_term::GridSize;
use wormhole_text::keys::{
    HOSTS_UNAVAILABLE_BUSY, HOSTS_UNAVAILABLE_FAILED, HOSTS_UNAVAILABLE_REFUSED, HOSTS_UNAVAILABLE_TIMED_OUT,
    HOSTS_UNAVAILABLE_TOO_NEW, HOSTS_UNAVAILABLE_TOO_OLD, HOSTS_UNAVAILABLE_UNREACHABLE,
};
use wormhole_text::text;

use super::TerminalView;
use super::hosted::HostFailure;
use super::status::TerminalEvent;

/// How long ending a host waits for it to take `Kill` before it is signalled.
const END_WAIT: Duration = Duration::from_secs(4);
/// How long before an attach that failed in a way that may pass is tried once more, before the bar is shown.
const RETRY_AFTER: Duration = Duration::from_millis(1500);

/// Why a host cannot be shown, in the interface's words.
pub fn unavailable_text(kind: UnavailableKind) -> &'static str {
    text(match kind {
        UnavailableKind::Refused => HOSTS_UNAVAILABLE_REFUSED,
        UnavailableKind::TooOld => HOSTS_UNAVAILABLE_TOO_OLD,
        UnavailableKind::TooNew => HOSTS_UNAVAILABLE_TOO_NEW,
        UnavailableKind::Busy => HOSTS_UNAVAILABLE_BUSY,
        UnavailableKind::TimedOut => HOSTS_UNAVAILABLE_TIMED_OUT,
        UnavailableKind::Unreachable => HOSTS_UNAVAILABLE_UNREACHABLE,
        UnavailableKind::Failed => HOSTS_UNAVAILABLE_FAILED,
    })
}

impl TerminalView {
    /// Why this terminal cannot show its host, while it cannot.
    pub fn host_unavailable(&self) -> Option<&Unavailable> {
        self.host.unavailable.as_ref()
    }

    /// How long the host has not answered, once it has not for long enough, while it is attached.
    pub fn host_not_responding(&self) -> Option<std::time::Duration> {
        let stats = self.pty.as_ref()?.link_stats()?;
        (!stats.detached && !stats.ended && !stats.lost).then_some(stats.not_responding_for).flatten()
    }

    /// Whether the bar for an unanswering host shows: it does not answer and "Wait" was not pressed.
    pub(super) fn shows_not_responding(&self) -> bool {
        !self.host.waiting && self.host_not_responding().is_some()
    }

    /// Called as the host answers again: "Wait" is spent, a later silence says it again.
    pub(super) fn host_answers_again(&mut self) {
        if self.host.waiting && self.host_not_responding().is_none() {
            self.host.waiting = false;
        }
    }

    /// What went wrong the last time the host was asked to end, until something else happens.
    pub fn host_end_failure(&self) -> Option<&str> {
        self.host.end_failed.as_deref()
    }

    /// Where an attach that failed in a way that may pass is tried once more before the bar is shown. `true`
    /// when a retry was scheduled (the pane stays blank meanwhile).
    pub(super) fn retry_once_soon(&mut self, unavailable: &Unavailable, cx: &mut Context<Self>) -> bool {
        if self.host.auto_retried || !unavailable.kind.is_transient() {
            return false;
        }
        self.host.auto_retried = true;
        self.host.connecting = true;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RETRY_AFTER).await;
            this.update(cx, |this, cx| this.try_attach_again(cx)).ok();
        })
        .detach();
        true
    }

    /// Attach to the host again, as at launch: the bar goes, the pane is blank until it is done.
    pub(super) fn try_attach_again(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.host_id().map(str::to_owned) else { return };
        let Some(options) = self.host.options.clone() else { return };
        self.host.unavailable = None;
        self.host.end_failed = None;
        self.host.connecting = true;
        let visible = self.host.on_screen;
        let size = self.painted_size().unwrap_or(GridSize {
            cols: 100,
            rows: 30,
            cell_width: 8.0,
            cell_height: 18.0,
        });
        let task = cx.background_spawn({
            let options = options.clone();
            async move {
                let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
                match wormhole_term::client::socket_of(&dir, &id) {
                    Ok(socket) => wormhole_core::terminal::attach_hosted(&socket, &id, &options, visible),
                    // No meta file to find a socket by: the same question as for a connection that found nothing.
                    Err(_) => Err(hosts::failure_of(
                        &dir,
                        &id,
                        &wormhole_term::client::AttachError::Unreachable("no meta file".into()),
                    )),
                }
                .map_err(HostFailure::Attach)
            }
        });
        self.await_host(task, options, size, true, cx);
        cx.notify();
    }

    /// End the host: `Kill` over its socket, then a signal (`hard`: for a host that does not answer, a
    /// process checked to be the host is killed outright). The terminal closes only when that worked.
    pub(crate) fn end_host_then_close(&mut self, hard: bool, cx: &mut Context<Self>) {
        let Some(id) = self.host_id().map(str::to_owned) else { return };
        self.host.end_failed = None;
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move {
                    let meta = hosts::find(&dir, &id)?;
                    if hard {
                        hosts::end_unresponsive_host(&meta, END_WAIT)
                    } else {
                        hosts::end_host(&meta, END_WAIT)
                    }
                })
                .await;
            this.update(cx, |this, cx| match done {
                Ok(_) => cx.emit(TerminalEvent::HostEnded),
                Err(why) => {
                    tracing::warn!("ending a session host from its terminal failed: {why}");
                    this.host.end_failed = Some(why);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// "Wait" on the bar for a host that does not answer.
    pub(super) fn wait_for_host(&mut self, cx: &mut Context<Self>) {
        self.host.waiting = true;
        cx.notify();
    }
}

/// Used by `host_ready`: how a failed attach is taken.
pub(super) enum Taken {
    /// Shown on the bar now.
    Shown,
    /// Tried again soon; the pane stays blank.
    Retrying,
}

impl TerminalView {
    /// An attach ended as `failure`, a host that runs and cannot be shown: say so, or try once more first.
    pub(super) fn host_unavailable_now(&mut self, unavailable: Unavailable, cx: &mut Context<Self>) -> Taken {
        tracing::warn!(
            id = self.host_id(),
            kind = ?unavailable.kind,
            "a session host runs and cannot be attached to: {}",
            unavailable.detail
        );
        if self.retry_once_soon(&unavailable, cx) {
            return Taken::Retrying;
        }
        self.host.unavailable = Some(unavailable);
        cx.emit(TerminalEvent::HostChanged);
        Taken::Shown
    }
}
