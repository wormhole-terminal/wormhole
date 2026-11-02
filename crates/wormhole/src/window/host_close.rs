//! Closing a terminal whose session host runs but cannot be shown. Closing drops the row, and with it the
//! host's id: the program in the host would be left running with nobody to say so. So the person is asked:
//! end the host with the terminal, or leave it running (it stays in the Session Hosts window, and is
//! adopted again if it ever accepts the app).

use std::rc::Rc;

use gpui_kit::*;
use wormhole_text::keys::{HOSTS_CLOSE_DETAIL, HOSTS_CLOSE_END, HOSTS_CLOSE_LEAVE};
use wormhole_text::text;

use super::{Workspace, session};
use crate::dialog;

impl Workspace {
    /// Whether the terminal at `ix` has a host that runs and cannot be shown.
    pub(super) fn host_cannot_be_shown(&self, ix: usize, cx: &App) -> bool {
        self.sessions.get(ix).is_some_and(|s| s.terminal().read(cx).host_unavailable().is_some())
    }

    /// The question before closing such a terminal (a sheet, as every close question is).
    pub(super) fn ask_close_unavailable(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(ix) else { return };
        if self.closing_session_prompt {
            return;
        }
        self.closing_session_prompt = true;
        cx.notify();
        let (id, terminal) = (session.id, session.terminal().clone());
        let question = dialog::CloseQuestion {
            title: session::close_session_title(&session.name()).into(),
            detail: text(HOSTS_CLOSE_DETAIL).into(),
            confirm: text(HOSTS_CLOSE_END),
        };
        let (leave, dismiss) = (cx.weak_entity(), cx.weak_entity());
        // By id: sessions may come and go while the question is up.
        let leave_it: dialog::OnConfirm = Rc::new(move |window: &mut Window, cx: &mut App| {
            let _ = leave.update(cx, |this, cx| {
                if let Some(ix) = this.sessions.iter().position(|s| s.id == id) {
                    this.close_session(ix, window, cx);
                }
            });
        });
        dialog::ask_before_closing_with(
            window,
            cx,
            question,
            Some((text(HOSTS_CLOSE_LEAVE), leave_it)),
            // Ends the host first; the terminal closes when that worked (`TerminalEvent::HostEnded`).
            move |_, cx| terminal.update(cx, |terminal, cx| terminal.end_host_then_close(false, cx)),
            move |cx| {
                let _ = dismiss.update(cx, |this, cx| {
                    this.closing_session_prompt = false;
                    cx.notify();
                });
            },
        );
    }
}
