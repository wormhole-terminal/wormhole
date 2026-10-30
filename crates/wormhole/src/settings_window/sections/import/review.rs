//! One terminal's settings as rows: what it says, what Wormhole would set, a tick; a shell command
//! shown whole with what it would do; then what doesn't carry over and why; then "Import selected".

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::import::{Found, Proposed, Setting, SettingValue, Source, command_line, visible};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::words::{reason_text, setting_label, value_text};
use crate::settings::UiColors;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

/// The label column and the width the value after the arrow keeps.
const LABEL_W: f32 = 150.0;
const VALUE_W: f32 = 180.0;

impl SettingsWindow {
    pub(super) fn render_review(&self, found: &Found, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let source = found.source;
        let proposal = &found.proposal;
        let pair = proposal.get(Setting::ThemeLight).is_some();
        let ticks = self.import.ticked.get(&source).cloned().unwrap_or_default();
        let selected = ticks.iter().filter(|on| **on).count();
        let (code_font, _) = crate::fonts::code_font(cx);

        let mut column = div().flex().flex_col().gap_2();
        column = column.child(style::section_label(ui, text(SETTINGS_IMPORT_TAKE)));
        if proposal.values.is_empty() {
            column = column
                .child(div().text_body().text_color(ui.muted).child(text(SETTINGS_IMPORT_NOTHING_TO_TAKE)));
        }
        for (ix, proposed) in proposal.values.iter().enumerate() {
            let on = ticks.get(ix).copied().unwrap_or(false);
            let row = Row { source, ix, proposed, on, pair };
            column = column.child(proposed_row(self, ui, &code_font, &row, cx));
        }
        if !proposal.skipped.is_empty() {
            column = column
                .child(div().pt_2().child(style::section_label(ui, text(SETTINGS_IMPORT_LEAVE))))
                .children(proposal.skipped.iter().map(|skipped| {
                    // The key above its reason, so a long one (`keybind = super+d=new_split:right`)
                    // has the whole row's width before it is cut.
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .py_1()
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_caption()
                                .text_color(ui.muted)
                                .font_family(code_font.clone())
                                .child(visible(&skipped.key)),
                        )
                        .child(div().min_w_0().text_body().child(reason_text(&skipped.reason)))
                }));
        }
        column
            .child(self.render_import_footer(found, selected, proposal.values.len(), ui, cx))
            .into_any_element()
    }

    fn render_import_footer(
        &self,
        found: &Found,
        selected: usize,
        total: usize,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> Div {
        let source = found.source;
        let done = self.import.done.filter(|(from, _)| *from == source).map(|(_, brought)| {
            div()
                .text_body()
                .text_color(ui.success_text)
                .child(settings_import_summary(whole(brought.brought), &brought.left.to_string()))
        });
        let this = cx.weak_entity();
        let apply = self
            .native_button_with_style(
                format!("import-apply-{}", source.key()),
                text(SETTINGS_IMPORT_APPLY),
                selected > 0,
                true,
                cx,
                move |_, cx| {
                    this.update(cx, |this, cx| {
                        if this.import.open == Some(source)
                            && this.import.ticked.get(&source).is_some_and(|ticks| ticks.iter().any(|on| *on))
                        {
                            this.apply_import(source, cx);
                        }
                    })
                    .ok();
                },
            )
            .unwrap_or_else(|| {
                Button::new("import-apply")
                    .primary()
                    .small()
                    .icon(IconName::ArrowDownToLine)
                    .label(text(SETTINGS_IMPORT_APPLY))
                    .disabled(selected == 0)
                    .on_click(cx.listener(move |this, _, _, cx| this.apply_import(source, cx)))
                    .into_any_element()
            });
        div()
            .flex()
            .items_center()
            .gap_3()
            .pt_2()
            .rule_top(ui)
            .child(apply)
            .child(
                div()
                    .text_caption()
                    .text_color(ui.muted)
                    .child(settings_import_selected(&selected.to_string(), &total.to_string())),
            )
            .child(div().flex_1())
            .children(done)
    }
}

/// The parts of one row of the list.
struct Row<'a> {
    /// The terminal the setting is from, whose ticks the row's checkbox is in.
    source: Source,
    /// Its place in the proposal: the checkbox's id and its slot in the ticks.
    ix: usize,
    proposed: &'a Proposed,
    on: bool,
    /// The proposal has a palette for each appearance, so the dark one says so.
    pair: bool,
}

/// One setting: its tick, name, what the other terminal says, and what Wormhole would set. A shell
/// command is taller: the whole command, and that it runs in every new terminal.
fn proposed_row(
    window: &SettingsWindow,
    ui: UiColors,
    code_font: &SharedString,
    row: &Row<'_>,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let Row { source, ix, proposed, on, pair } = *row;
    let label = setting_label(proposed.setting, pair);
    let this = cx.weak_entity();
    let change = move |on, _: &mut Window, cx: &mut App| {
        this.update(cx, |this, cx| {
            if let Some(slot) = this.import.ticked.get_mut(&source).and_then(|ticks| ticks.get_mut(ix)) {
                *slot = on;
            }
            this.import.done = None;
            cx.notify();
        })
        .ok();
    };
    let tick = if window.native_switches.is_some() {
        window.checkbox_control(
            SharedString::from(format!("import-tick-{source:?}-{:?}-{ix}", proposed.setting)),
            label,
            on,
            true,
            false,
            cx,
            change,
        )
    } else {
        style::checkbox(("import-tick", ix), ui, on)
            .spoken(label)
            .tab_stop(true)
            .on_change(change)
            .into_any_element()
    };
    let head = div()
        .flex()
        .items_center()
        .gap_3()
        .min_h(px(style::ROW_H))
        .child(tick)
        .child(div().w(px(LABEL_W)).flex_none().truncate().text_body().child(label))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_caption()
                .text_color(ui.muted)
                .font_family(code_font.clone())
                .child(visible(&proposed.from)),
        );
    match &proposed.value {
        SettingValue::Profile(profile) => div()
            .flex()
            .flex_col()
            .gap_1()
            .child(head)
            .child(
                div()
                    .ml(px(style::CONTROL_H))
                    .px_2()
                    .py_1()
                    .rounded(ui.radius_chip)
                    .bg(ui.hover)
                    .text_caption()
                    .font_family(code_font.clone())
                    .child(command_line(profile)),
            )
            .child(
                div()
                    .ml(px(style::CONTROL_H))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_caption()
                    .text_color(ui.warning_text)
                    .child(style::icon(IconName::TriangleAlert))
                    .child(text(SETTINGS_IMPORT_RUNS_EVERYWHERE)),
            )
            .into_any_element(),
        value => div()
            .child(
                head.child(style::icon(IconName::ArrowRight).text_color(ui.faint))
                    .child(div().w(px(VALUE_W)).flex_none().truncate().text_body().child(value_text(value))),
            )
            .into_any_element(),
    }
}
