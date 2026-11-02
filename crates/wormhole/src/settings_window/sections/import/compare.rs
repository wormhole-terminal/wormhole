//! How each terminal found compares with Wormhole, in a sheet: its strength and today's drawbacks, each with the
//! page it was checked against and the date, then Wormhole's own under "What Wormhole doesn't do yet".
//! A fact about the other terminal, never "unlike Wormhole" (`terminal-comparison-app.md`, "Rules for
//! the card"); the content is core's `import::compare`, the sentences the catalog's.

use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::import::{CHECKED, Comparison, Drawback, WORMHOLE, comparison};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::words::{Subject, gap_text, strength_text};
use crate::settings::UiColors;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

/// The sheet's width, and the share of the window it may take in height before it scrolls.
const SHEET_W: f32 = 560.0;
const SHEET_MAX_H: f32 = 0.8;

/// "Compare with Wormhole": a row that opens the comparison in a sheet over the window. Closed it says how
/// many terminals it compares.
pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let count = window.import.found.as_ref().map(|found| found.len()).filter(|n| *n > 0);
    style::more_row(
        "import-compare",
        ui,
        text(SETTINGS_IMPORT_COMPARE),
        count.map(|n| whole(n).to_string().into()),
        window.import.comparing,
    )
    .on_click(cx.listener(|this, _, _, cx| {
        this.import.comparing = true;
        cx.notify();
    }))
    .into_any_element()
}

impl SettingsWindow {
    /// Close the comparison sheet. Returns whether there was one, so Esc is spent on it.
    pub(in crate::settings_window) fn close_import_sheet(&mut self, cx: &mut Context<Self>) -> bool {
        let was = std::mem::take(&mut self.import.comparing);
        if was {
            cx.notify();
        }
        was
    }

    /// The comparison as a sheet over the window: each terminal found against Wormhole, with a scrim that
    /// keeps the page behind it still.
    pub(in crate::settings_window) fn render_import_sheet(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.import.comparing {
            return None;
        }
        let ui = ui(cx);
        let max_h = window.viewport_size().height * SHEET_MAX_H;
        let mut body = div().flex().flex_col().gap_6();
        match self.import.found.as_deref().filter(|found| !found.is_empty()) {
            None => {
                body = body
                    .child(div().text_body().text_color(ui.muted).child(text(SETTINGS_IMPORT_COMPARE_NONE)));
            }
            Some(all) => {
                // Every source link of the sheet has its own element id: the running count of them.
                let mut links = 0;
                for found in all {
                    let subject = Subject::Terminal(found.source);
                    let theirs = part(
                        ui,
                        subject,
                        comparison(found.source),
                        settings_import_compare_gaps(subject.name()),
                        &mut links,
                        cx,
                    );
                    body =
                        body.child(titled(ui, subject.name(), Some(settings_import_as_of(CHECKED)), theirs));
                }
                let wormhole = part(
                    ui,
                    Subject::Wormhole,
                    &WORMHOLE,
                    text(SETTINGS_IMPORT_WORMHOLE_GAPS).to_string(),
                    &mut links,
                    cx,
                );
                body = body.child(titled(ui, Subject::Wormhole.name(), None, wormhole));
            }
        }
        let card = style::card(ui, ui.border_strong)
            .id("import-sheet")
            .w(px(SHEET_W))
            .max_w_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .bg(ui.elevated)
            .occlude()
            .child(style::panel_title(ui, text(SETTINGS_IMPORT_GROUP_COMPARE)))
            .child(div().id("import-sheet-body").max_h(max_h).overflow_y_scroll().child(body))
            .child(div().flex().justify_end().child(
                style::form_button("import-sheet-done").primary().label(text(SETTINGS_IMPORT_DONE)).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.close_import_sheet(cx);
                    }),
                ),
            ));
        Some(
            div()
                .id("import-scrim")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(ui.shadow)
                .occlude()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.close_import_sheet(cx);
                }))
                .child(card)
                .into_any_element(),
        )
    }
}

/// A terminal's name (with the day its facts were checked) over its comparison.
fn titled(ui: UiColors, name: &str, as_of: Option<String>, part: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(name.to_string()))
                .children(as_of.map(|day| div().text_caption().text_color(ui.faint).child(day))),
        )
        .child(part)
}

/// A subject's strength sentence first, then its label ("Where … falls short today") and under it the
/// drawbacks as rows: the label never sits above the strength, which would read as a drawback.
fn part(
    ui: UiColors,
    subject: Subject,
    comparison: &Comparison,
    label: String,
    links: &mut usize,
    cx: &mut Context<SettingsWindow>,
) -> Div {
    let rows: Vec<AnyElement> =
        comparison.drawbacks.iter().map(|drawback| drawback_row(ui, subject, drawback, links, cx)).collect();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(
            strength_text(subject, comparison)
                .map(|strength| div().text_body().text_color(ui.muted).child(strength)),
        )
        .child(style::setting_group_head(ui, &label, None))
        .child(div().flex().flex_col().children(rows))
}

/// One drawback: the sentence, the day it was checked when that isn't the card's own (or it is to be
/// looked at again), and its sources, each of which opens in the
/// browser. A private source has none to open.
fn drawback_row(
    ui: UiColors,
    subject: Subject,
    drawback: &'static Drawback,
    links: &mut usize,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let several = drawback.sources.len() > 1;
    let buttons: Vec<AnyElement> = drawback
        .sources
        .iter()
        .enumerate()
        .map(|(n, url)| {
            *links += 1;
            let label = if several {
                settings_import_source_numbered(&(n + 1).to_string())
            } else {
                text(SETTINGS_IMPORT_SOURCE).to_string()
            };
            style::link_button(("import-source", *links), ui, label, false)
                .child(style::icon(IconName::ExternalLink))
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(url)))
                .into_any_element()
        })
        .collect();
    div()
        .flex()
        .items_center()
        .gap_3()
        .py_2()
        .rule_top(ui)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_body().child(gap_text(subject, drawback.id).unwrap_or_default()))
                .children((drawback.date != CHECKED || drawback.recheck).then(|| {
                    div().text_caption().text_color(ui.muted).child(settings_import_as_of(drawback.date))
                })),
        )
        .children(buttons)
        .into_any_element()
}
