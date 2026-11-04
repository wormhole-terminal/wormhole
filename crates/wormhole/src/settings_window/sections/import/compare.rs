//! How the open terminal compares with Wormhole: its strength and today's drawbacks, each with the
//! page it was checked against and the date, then Wormhole's own under "What Wormhole doesn't do yet".
//! A fact about the other terminal, never "unlike Wormhole" (`terminal-comparison-app.md`, "Rules for
//! the card"); the content is core's `import::compare`, the sentences the catalog's.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::import::{CHECKED, Comparison, Drawback, WORMHOLE, comparison};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::words::{Subject, gap_text, strength_text};
use crate::settings::UiColors;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};

pub(super) fn draw(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let Some(found) = window.import.open_found() else {
        return style::empty_note(ui, text(SETTINGS_IMPORT_COMPARE_NONE)).into_any_element();
    };
    let subject = Subject::Terminal(found.source);
    let theirs = comparison(found.source);
    // Every source link on the page has its own element id: the running count of them.
    let mut links = 0;
    let terminal = part(ui, subject, theirs, settings_import_compare_gaps(subject.name()), &mut links, cx);
    let wormhole = part(
        ui,
        Subject::Wormhole,
        &WORMHOLE,
        text(SETTINGS_IMPORT_WORMHOLE_GAPS).to_string(),
        &mut links,
        cx,
    );
    div()
        .flex()
        .flex_col()
        .gap_6()
        .child(
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
                        .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(subject.name()))
                        .child(
                            div().text_caption().text_color(ui.faint).child(settings_import_as_of(CHECKED)),
                        ),
                )
                .child(terminal),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(div().text_body().font_weight(FontWeight::SEMIBOLD).child(Subject::Wormhole.name()))
                .child(wormhole),
        )
        .into_any_element()
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
        .child(style::section_label(ui, &label))
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
