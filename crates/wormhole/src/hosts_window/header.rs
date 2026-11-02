//! The strip above the cards: this app, the folder, the feature's switch, the host program found,
//! and how many hosts stand each way.

use gpui_kit::*;
use wormhole_core::ipc::hosts::report::Report;
use wormhole_text::keys::{
    HOSTS_ACTION_COPIED, HOSTS_ACTION_COPY_ALL, HOSTS_COUNT_ATTACHED, HOSTS_COUNT_DETACHED,
    HOSTS_COUNT_ORPHAN, HOSTS_COUNT_STALE, HOSTS_HEADER_BINARY, HOSTS_HEADER_BINARY_MISSING,
    HOSTS_HEADER_FLAG, HOSTS_HEADER_FLAG_OFF, HOSTS_HEADER_FLAG_ON, HOSTS_HEADER_FOLDER, HOSTS_OFF_NOTE,
    HOSTS_WINDOW_TITLE, hosts_header_app,
};
use wormhole_text::text;

use super::{Copied, HostsWindow};
use crate::settings::UiColors;
use crate::style::{self, TextRole};

impl HostsWindow {
    pub(super) fn header(&self, report: &Report, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let app = &report.app;
        let counts = report.counts();
        let flag = if report.flag_on {
            style::status_badge(ui, ui.success_text, text(HOSTS_HEADER_FLAG_ON))
        } else {
            style::status_badge(ui, ui.warning_text, text(HOSTS_HEADER_FLAG_OFF))
        };
        let (binary, binary_color) = match &report.host_binary {
            Some(path) => (path.display().to_string(), ui.text),
            None => (text(HOSTS_HEADER_BINARY_MISSING).to_string(), ui.warning_text),
        };
        let copy_all = if self.copied == Some(Copied::All) {
            text(HOSTS_ACTION_COPIED)
        } else {
            text(HOSTS_ACTION_COPY_ALL)
        };
        let count = |label: &'static str, n: usize| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().text_caption().text_color(ui.muted).child(label))
                .child(style::count_badge(ui, n.to_string()))
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .pb_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_title().font_weight(FontWeight::BOLD).child(text(HOSTS_WINDOW_TITLE)))
                    .child(
                        Self::button("hosts-copy-all", copy_all)
                            .on_click(cx.listener(|this, _, _, cx| this.copy(Copied::All, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .text_label()
                    .child(hosts_header_app(
                        &app.version,
                        &format!("{}.{}", app.proto_major, app.proto_minor),
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_color(ui.muted).child(text(HOSTS_HEADER_FLAG)))
                            .child(flag),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .min_w_0()
                            .flex_1()
                            .child(div().flex_none().text_color(ui.muted).child(text(HOSTS_HEADER_BINARY)))
                            .child(div().min_w_0().truncate().text_color(binary_color).child(binary)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .text_label()
                    .child(div().flex_none().text_color(ui.muted).child(text(HOSTS_HEADER_FOLDER)))
                    .child(div().min_w_0().truncate().child(report.hosts_dir.display().to_string())),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(count(text(HOSTS_COUNT_ATTACHED), counts.attached))
                    .child(count(text(HOSTS_COUNT_DETACHED), counts.detached))
                    .child(count(text(HOSTS_COUNT_ORPHAN), counts.orphan))
                    .child(count(text(HOSTS_COUNT_STALE), counts.stale)),
            )
            .children(
                (!report.flag_on)
                    .then(|| style::banner(ui, style::BannerKind::Info, false, text(HOSTS_OFF_NOTE))),
            )
            .into_any_element()
    }
}
