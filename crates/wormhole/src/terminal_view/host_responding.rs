//! A terminal whose session host stopped answering (stopped or stuck: its socket stays open and it
//! says nothing). The client pings a host it watches and tells when a ping has gone unanswered for
//! five seconds (`wormhole_term::client::liveness`); the terminal then says so in a bar over its
//! pane, and offers to wait or to end the host (killed outright when it does not take the request,
//! after checking the process is the host).

use std::time::Duration;

use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::ipc::hosts;
use wormhole_text::keys::{HOSTS_NOT_RESPONDING, HOSTS_NOT_RESPONDING_END, HOSTS_NOT_RESPONDING_WAIT};
use wormhole_text::text;

use super::{TerminalEvent, TerminalView};
use crate::settings::store_in;
use crate::style::{Edges, TextRole};

/// How long ending a host waits for it to take the request before it is killed.
const END_WAIT: Duration = Duration::from_secs(3);

impl TerminalView {
    /// How long the host has not answered, once it has not for long enough, while it is attached.
    pub fn host_not_responding(&self) -> Option<Duration> {
        let stats = self.pty.as_ref()?.link_stats()?;
        (!stats.detached && !stats.ended && !stats.lost).then_some(stats.not_responding_for).flatten()
    }

    /// Whether the bar for an unanswering host shows: it does not answer and "Wait" was not pressed.
    fn shows_not_responding(&self) -> bool {
        !self.host.waiting && self.host_not_responding().is_some()
    }

    /// Called as the host answers again: "Wait" is spent, a later silence says it again.
    pub(super) fn host_answers_again(&mut self) {
        if self.host.waiting && self.host_not_responding().is_none() {
            self.host.waiting = false;
        }
    }

    /// End the host that does not answer (`Kill` over its socket, else `SIGKILL`, only for a process
    /// that is the host), then close the terminal.
    fn end_unresponsive(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.host_id().map(str::to_owned) else { return };
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move {
                    hosts::find(&dir, &id).and_then(|meta| hosts::end_unresponsive_host(&meta, END_WAIT))
                })
                .await;
            if let Err(why) = &done {
                tracing::warn!("ending a session host that does not answer failed: {why}");
            }
            this.update(cx, |_, cx| cx.emit(TerminalEvent::Close)).ok();
        })
        .detach();
    }

    /// The bar along the bottom of a pane whose host does not answer.
    pub(super) fn render_host_not_responding(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.shows_not_responding() {
            return None;
        }
        let ui = store_in(&self.profile, cx).ui;
        Some(
            div()
                .id("host-not-responding-bar")
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(crate::style::TITLE_ROW_H))
                .flex()
                .items_center()
                .gap_2()
                .pl_4()
                .pr_2()
                .bg(ui.panel)
                .rule_top(ui)
                .cursor(CursorStyle::Arrow)
                // The pane below it would start a selection.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_caption()
                        .text_color(ui.warning_text)
                        .child(text(HOSTS_NOT_RESPONDING)),
                )
                .child(
                    Button::new("host-not-responding-wait")
                        .small()
                        .outline()
                        .label(text(HOSTS_NOT_RESPONDING_WAIT))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.host.waiting = true;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("host-not-responding-end")
                        .small()
                        .outline()
                        .label(text(HOSTS_NOT_RESPONDING_END))
                        .on_click(cx.listener(|this, _, _, cx| this.end_unresponsive(cx))),
                )
                .into_any_element(),
        )
    }
}
