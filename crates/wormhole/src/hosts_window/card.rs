//! One host as a card: a badge for how it stands, its id, the buttons, and the facts in two columns.

use gpui_kit::*;
use wormhole_core::ipc::hosts::report::{Answer, HostRow, Report, Standing, bytes_words};
use wormhole_text::keys::{
    HOSTS_ACTION_COPIED, HOSTS_ACTION_COPY, HOSTS_ACTION_END, HOSTS_ACTION_FORGET, HOSTS_FIELD_ANSWER,
    HOSTS_FIELD_FOLDER, HOSTS_FIELD_HOST_PID, HOSTS_FIELD_LINK, HOSTS_FIELD_MEMORY, HOSTS_FIELD_META,
    HOSTS_FIELD_PROGRAM, HOSTS_FIELD_PROTOCOL, HOSTS_FIELD_SHELL_PID, HOSTS_FIELD_SOCKET,
    HOSTS_FIELD_STARTED, HOSTS_FIELD_VERSION, HOSTS_UNKNOWN, hosts_version_differs,
};
use wormhole_text::text;

use super::{Copied, HostsWindow, words};
use crate::settings::UiColors;
use crate::style::{self, TextRole};

/// How wide a fact's name is: wide enough for the longest ("Shell process").
const LABEL_W: f32 = 104.0;

/// One fact: its name, then its value in `tone`. `wide` takes the card's whole width (paths).
fn fact(ui: UiColors, label: &'static str, value: impl Into<SharedString>, tone: Hsla, wide: bool) -> Div {
    div()
        .flex()
        .gap_2()
        .min_w_0()
        .pr_4()
        .w(relative(if wide { 1.0 } else { 0.5 }))
        .text_label()
        .child(div().w(px(LABEL_W)).flex_none().text_color(ui.muted).child(label))
        .child(div().flex_1().min_w_0().truncate().text_color(tone).child(value.into()))
}

impl HostsWindow {
    pub(super) fn card(
        &self,
        ix: usize,
        row: &HostRow,
        report: &Report,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let app = &report.app;
        let attention = row.needs_attention(app);
        let unknown = || text(HOSTS_UNKNOWN).to_string();
        let number = |n: Option<u32>| n.map_or_else(unknown, |n| n.to_string());
        let path =
            |p: &Option<std::path::PathBuf>| p.as_ref().map_or_else(unknown, |p| p.display().to_string());
        let mismatch = row.version_differs(app);
        let version = match (&row.host_version, mismatch) {
            (Some(v), true) => format!("{v} ({})", hosts_version_differs(&app.version)),
            (Some(v), false) => v.clone(),
            (None, _) => unknown(),
        };
        let bad_link = row.link.is_some_and(|l| l.lost);
        let bad_answer = row.standing.is_live() && matches!(row.answer, Answer::No(_) | Answer::Incompatible);
        let warn = ui.warning_text;
        let mut facts = vec![
            fact(ui, text(HOSTS_FIELD_HOST_PID), number(row.host_pid), ui.text, false),
            fact(ui, text(HOSTS_FIELD_SHELL_PID), number(row.shell_pid), ui.text, false),
            fact(ui, text(HOSTS_FIELD_PROGRAM), row.program.clone().unwrap_or_else(unknown), ui.text, false),
            fact(ui, text(HOSTS_FIELD_MEMORY), row.rss.map_or_else(unknown, bytes_words), ui.text, false),
            fact(ui, text(HOSTS_FIELD_VERSION), version, if mismatch { warn } else { ui.text }, false),
            fact(
                ui,
                text(HOSTS_FIELD_PROTOCOL),
                words::protocol(row).unwrap_or_else(unknown),
                if row.fit_problem() { ui.danger_text } else { ui.text },
                false,
            ),
            fact(ui, text(HOSTS_FIELD_STARTED), words::started(row).unwrap_or_else(unknown), ui.text, false),
            fact(
                ui,
                text(HOSTS_FIELD_ANSWER),
                words::answer(&row.answer),
                if bad_answer { ui.danger_text } else { ui.text },
                false,
            ),
        ];
        if row.link.is_some() || matches!(row.standing, Standing::Attached { .. }) {
            facts.push(fact(
                ui,
                text(HOSTS_FIELD_LINK),
                words::link(row.link.as_ref()),
                if bad_link { ui.danger_text } else { ui.text },
                true,
            ));
        }
        facts.push(fact(ui, text(HOSTS_FIELD_FOLDER), path(&row.cwd), ui.text, true));
        facts.push(fact(ui, text(HOSTS_FIELD_SOCKET), path(&row.socket), ui.muted, true));
        facts.push(fact(ui, text(HOSTS_FIELD_META), row.meta_path.display().to_string(), ui.muted, true));

        let id = row.id.clone();
        let copy_label = if self.copied == Some(Copied::Host(id.clone())) {
            text(HOSTS_ACTION_COPIED)
        } else {
            text(HOSTS_ACTION_COPY)
        };
        let copy = Self::button(("hosts-copy", ix), copy_label).on_click({
            let id = id.clone();
            cx.listener(move |this, _, _, cx| this.copy(Copied::Host(id.clone()), cx))
        });
        let act = if row.can_end() {
            Some(Self::button(("hosts-end", ix), text(HOSTS_ACTION_END)).on_click({
                let id = id.clone();
                cx.listener(move |this, _, window, cx| this.ask_to_end(&id, window, cx))
            }))
        } else if row.can_forget() {
            Some(Self::button(("hosts-forget", ix), text(HOSTS_ACTION_FORGET)).on_click({
                let id = id.clone();
                cx.listener(move |this, _, _, cx| this.forget(id.clone(), cx))
            }))
        } else {
            None
        };
        let border = if attention { ui.warning } else { ui.border };
        style::card(ui, border)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(style::status_badge(
                        ui,
                        words::standing_color(ui, &row.standing),
                        words::standing_label(&row.standing),
                    ))
                    .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(row.id.clone()))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_label()
                            .text_color(ui.muted)
                            .child(words::standing_note(&row.standing)),
                    )
                    .child(copy)
                    .children(act),
            )
            .child(div().flex().flex_wrap().gap_y_1().children(facts))
            .into_any_element()
    }
}
