//! One host as a card: its state and the terminal that holds it on the first line, its id and what
//! is wrong under that, the actions at the right; opened, the technical facts.

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::ipc::hosts::report::{HostRow, Report};
use wormhole_text::keys::{
    HOSTS_ACTION_COPIED, HOSTS_ACTION_COPY, HOSTS_ACTION_END, HOSTS_ACTION_FORGET, hosts_problem,
    hosts_row_label,
};
use wormhole_text::text;

use super::detail::{self, Fact, Tone};
use super::{Copied, HostsWindow, words};
use crate::settings::UiColors;
use crate::style::{self, Edges as _, TextRole};

/// How wide a fact's name is: wide enough for the longest ("Shell process").
const LABEL_W: f32 = 104.0;
/// How far the lines under a card's first line start: past its chevron and the gap after it, so they line up
/// with the first line's words.
const LEAD_INDENT: f32 = style::ITEM_LEAD_W + 8.0;

/// One fact: its name muted, then its value in its own tone, in the code font for a path.
fn fact(ui: UiColors, fact: &Fact, code: &SharedString) -> Div {
    let value = div().flex_1().min_w_0().truncate().text_color(fact.tone.color(ui)).child(fact.value.clone());
    div()
        .flex()
        .gap_2()
        .min_w_0()
        .text_caption()
        .child(div().w(px(LABEL_W)).flex_none().text_color(ui.muted).child(fact.label))
        .child(if fact.code { value.font_family(code.clone()) } else { value })
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
        let open = self.open.contains(&row.id);
        let groups = detail::groups(row, &report.app);
        let (code_font, _) = crate::fonts::code_font(cx);
        let note = words::standing_note(&row.standing);
        let label = words::standing_label(&row.standing);
        let id = row.id.clone();

        // What is wrong stays on the card while it is folded.
        let problems = groups.iter().flatten().filter(|fact| fact.tone.is_problem()).map(|fact| {
            let color = if fact.tone == Tone::Danger { ui.danger_text } else { ui.warning_text };
            div()
                .min_w_0()
                .truncate()
                .text_caption()
                .text_color(color)
                .child(hosts_problem(fact.label, &fact.value))
        });
        let first_line = div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(style::lead(style::disclosure(ui, !open)))
            .child(style::status_badge(ui, words::standing_color(ui, &row.standing), label))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_body()
                    .font_weight(FontWeight::MEDIUM)
                    .child(note.clone()),
            );
        let second_line = div()
            .flex()
            .gap_3()
            .min_w_0()
            .pl(px(LEAD_INDENT))
            .text_caption()
            .text_color(ui.muted)
            .child(div().flex_none().font_family(code_font.clone()).child(row.id.clone()))
            .children(row.program.clone().map(|program| div().min_w_0().truncate().child(program)));
        let toggle = div()
            .id(("hosts-toggle", ix))
            .role(Role::Button)
            .aria_label(hosts_row_label(label, &row.id, &note))
            .aria_expanded(open)
            // A tab stop, so ↩ and Space open and close it as a click does.
            .focusable()
            .tab_stop(true)
            .focus_visible(move |d| d.bg(ui.hover))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .rounded(ui.radius)
            .cursor_pointer()
            .on_click(cx.listener({
                let id = id.clone();
                move |this, _, _, cx| this.toggle(&id, cx)
            }))
            .child(first_line)
            .child(second_line)
            .children(
                problems.map(|line| div().pl(px(LEAD_INDENT)).min_w_0().child(line)).collect::<Vec<_>>(),
            );

        let copied = self.copied == Some(Copied::Host(id.clone()));
        let copy = style::icon_button(
            ("hosts-copy", ix),
            ui,
            IconName::Copy,
            if copied { text(HOSTS_ACTION_COPIED) } else { text(HOSTS_ACTION_COPY) },
        )
        .on_click({
            let id = id.clone();
            cx.listener(move |this, _, _, cx| this.copy(Copied::Host(id.clone()), cx))
        });
        let act = if row.can_end() {
            Some(
                Button::new(("hosts-end", ix))
                    .danger()
                    .outline()
                    .small()
                    .label(text(HOSTS_ACTION_END))
                    .on_click({
                        let id = id.clone();
                        cx.listener(move |this, _, window, cx| this.ask_to_end(&id, window, cx))
                    })
                    .into_any_element(),
            )
        } else if row.can_forget() {
            Some(
                Button::new(("hosts-forget", ix))
                    .outline()
                    .small()
                    .label(text(HOSTS_ACTION_FORGET))
                    .on_click({
                        let id = id.clone();
                        cx.listener(move |this, _, _, cx| this.forget(id.clone(), cx))
                    })
                    .into_any_element(),
            )
        } else {
            None
        };
        let header = div()
            .flex()
            .items_start()
            .gap_2()
            .child(toggle)
            .child(div().flex_none().flex().items_center().gap_2().child(copy).children(act));

        let border = if row.needs_attention(&report.app) { ui.warning } else { ui.border };
        style::card(ui, border)
            .flex()
            .flex_col()
            .gap_3()
            .child(header)
            .when(open, |card| {
                card.child(div().flex().flex_col().gap_3().pt_3().pl(px(LEAD_INDENT)).rule_top(ui).children(
                    groups.iter().map(|group| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(group.iter().map(|f| fact(ui, f, &code_font)))
                    }),
                ))
            })
            .into_any_element()
    }
}
