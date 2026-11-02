//! A terminal whose session host runs but could not be attached to: it keeps the host's id (the
//! program is in the host), shows why in a bar over its blank pane, and offers the two things a
//! person can do: end the host, or open a new terminal in its place and leave the host be.

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::ipc::hosts::{self, Unavailable, UnavailableKind};
use wormhole_text::keys::{
    HOSTS_UNAVAILABLE_BUSY, HOSTS_UNAVAILABLE_END, HOSTS_UNAVAILABLE_FAILED, HOSTS_UNAVAILABLE_NEW,
    HOSTS_UNAVAILABLE_REFUSED, HOSTS_UNAVAILABLE_TIMED_OUT, HOSTS_UNAVAILABLE_TOO_NEW,
    HOSTS_UNAVAILABLE_TOO_OLD, HOSTS_UNAVAILABLE_UNREACHABLE,
};
use wormhole_text::text;

use super::{TerminalEvent, TerminalView};
use crate::settings::store_in;
use crate::style::{Edges, TextRole};

/// How long ending a host waits for it to take the hangup.
const END_WAIT: Duration = Duration::from_secs(5);

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

    /// End the host (asked over its socket, signalled only when the process is checked to be the
    /// host), then close the terminal: nothing is left to show.
    pub(super) fn end_unavailable_host(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.host_id().map(str::to_owned) else { return };
        let dir = hosts::hosts_dir(&wormhole_core::storage::paths::data_dir());
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(
                    async move { hosts::find(&dir, &id).and_then(|meta| hosts::end_host(&meta, END_WAIT)) },
                )
                .await;
            if let Err(why) = &done {
                tracing::warn!("ending a session host that could not be attached to failed: {why}");
            }
            this.update(cx, |_, cx| cx.emit(TerminalEvent::Close)).ok();
        })
        .detach();
    }

    /// The bar along the bottom of a pane whose host cannot be shown.
    pub(super) fn render_host_unavailable(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let unavailable = self.host.unavailable.as_ref()?;
        let ui = store_in(&self.profile, cx).ui;
        Some(
            div()
                .id("host-unavailable-bar")
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
                .child(div().text_color(ui.warning_text).child(crate::style::icon(IconName::TriangleAlert)))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_caption()
                        .text_color(ui.muted)
                        .child(unavailable_text(unavailable.kind)),
                )
                .child(
                    Button::new("host-unavailable-end")
                        .small()
                        .outline()
                        .label(text(HOSTS_UNAVAILABLE_END))
                        .on_click(cx.listener(|this, _, _, cx| this.end_unavailable_host(cx))),
                )
                .child(
                    Button::new("host-unavailable-new")
                        .small()
                        .outline()
                        .label(text(HOSTS_UNAVAILABLE_NEW))
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(TerminalEvent::Restart))),
                )
                .into_any_element(),
        )
    }
}
