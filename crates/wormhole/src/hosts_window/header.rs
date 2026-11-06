//! What is above the cards: the title with Copy all, a quiet summary (this app, the feature's switch, the
//! host program and the folder), and how many hosts stand each way.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use wormhole_core::ipc::host_binary::HostProgram;
use wormhole_core::ipc::hosts::report::Report;
use wormhole_text::keys::{
    HOSTS_ACTION_COPIED, HOSTS_ACTION_COPY_ALL, HOSTS_COUNT_ATTACHED, HOSTS_COUNT_DETACHED,
    HOSTS_COUNT_ORPHAN, HOSTS_COUNT_STALE, HOSTS_HEADER_BINARY, HOSTS_HEADER_BINARY_DISPOSABLE,
    HOSTS_HEADER_BINARY_MISSING, HOSTS_HEADER_FLAG, HOSTS_HEADER_FLAG_OFF, HOSTS_HEADER_FLAG_ON,
    HOSTS_HEADER_FOLDER, HOSTS_OFF_NOTE, HOSTS_WINDOW_TITLE, hosts_header_app,
};
use wormhole_text::text;

use super::{Copied, HostsWindow, words};
use crate::settings::UiColors;
use crate::style::{self, TextRole};

/// How wide a summary line's name is: wide enough for "Hosts folder".
const LABEL_W: f32 = 104.0;

/// A line of the summary: its name, then its value. A path is set in the code font.
fn line(ui: UiColors, label: &'static str, value: String, color: Hsla, code: Option<SharedString>) -> Div {
    let value = div().min_w_0().truncate().text_color(color).child(value);
    div()
        .flex()
        .gap_2()
        .text_caption()
        .child(div().w(px(LABEL_W)).flex_none().text_color(ui.muted).child(label))
        .child(match code {
            Some(family) => value.font_family(family),
            None => value,
        })
}

impl HostsWindow {
    pub(super) fn header(&self, report: &Report, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let app = &report.app;
        let counts = report.counts();
        let (code_font, _) = crate::fonts::code_font(cx);
        let flag = if report.flag_on {
            style::status_badge(ui, ui.success_text, text(HOSTS_HEADER_FLAG_ON))
        } else {
            style::status_badge(ui, ui.warning_text, text(HOSTS_HEADER_FLAG_OFF))
        };
        let (binary, binary_color, binary_code) = match &report.host_program {
            HostProgram::Found(path) => (words::path_keeping(path, 64), ui.muted, Some(code_font.clone())),
            HostProgram::DisposableCopy => {
                (text(HOSTS_HEADER_BINARY_DISPOSABLE).to_string(), ui.warning_text, None)
            }
            HostProgram::Missing => (text(HOSTS_HEADER_BINARY_MISSING).to_string(), ui.warning_text, None),
        };
        let copy_all = Button::new("hosts-copy-all")
            .outline()
            .small()
            .icon(IconName::Copy)
            .label(if self.copied == Some(Copied::All) {
                text(HOSTS_ACTION_COPIED)
            } else {
                text(HOSTS_ACTION_COPY_ALL)
            })
            .on_click(cx.listener(|this, _, _, cx| this.copy(Copied::All, cx)));
        // A count of none is a plain 0, so the chips that are lit are the ones with something in them.
        let tally = |label: &'static str, n: usize| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_caption()
                .child(div().text_color(ui.muted).child(label))
                .child(if n > 0 {
                    style::count_badge(ui, n.to_string())
                } else {
                    div().px_1().text_color(ui.faint).child("0")
                })
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .px_6()
            .pb_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .child(div().text_title().font_weight(FontWeight::BOLD).child(text(HOSTS_WINDOW_TITLE)))
                    .child(copy_all),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_body()
                            .child(hosts_header_app(
                                &app.version,
                                &words::app_protocol(app.proto_major, app.proto_minor),
                            ))
                            .child(div().text_caption().text_color(ui.muted).child(text(HOSTS_HEADER_FLAG)))
                            .child(flag),
                    )
                    .child(line(ui, text(HOSTS_HEADER_BINARY), binary, binary_color, binary_code))
                    .child(line(
                        ui,
                        text(HOSTS_HEADER_FOLDER),
                        words::path(&report.hosts_dir),
                        ui.muted,
                        Some(code_font),
                    )),
            )
            .children(
                (!report.flag_on)
                    .then(|| style::banner(ui, style::BannerKind::Info, false, text(HOSTS_OFF_NOTE))),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(tally(text(HOSTS_COUNT_ATTACHED), counts.attached))
                    .child(tally(text(HOSTS_COUNT_DETACHED), counts.detached))
                    .child(tally(text(HOSTS_COUNT_ORPHAN), counts.orphan))
                    .child(tally(text(HOSTS_COUNT_STALE), counts.stale)),
            )
            .into_any_element()
    }
}
