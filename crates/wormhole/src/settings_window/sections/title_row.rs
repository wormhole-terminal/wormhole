//! Appearance → Title row: the row's parts in the order they stand, each with its switch, and the
//! flexible space among them (the parts above it sit at the row's left, the others at its right);
//! under them the parts it doesn't show. A shown part is dragged to move it; a hidden one, switched
//! on, comes back where it is by default. A right-click on the title row shows and hides them too.

use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::layout::title_parts::TitlePart;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::settings::{UiColors, update_settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, ArrangeParts, Reorder};
use crate::window::title::parts;

/// The list's name for its drags: a part drops only among the parts.
const LIST: &str = "settings-title-row";

/// The row's own control: none, the list under it is.
pub(super) fn draw_control(_: &SettingsWindow, _: &mut Context<SettingsWindow>) -> AnyElement {
    div().into_any_element()
}

/// The parts, shown ones first in their order, then the hidden ones.
pub(super) fn draw_parts(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    let ui = ui(cx);
    // The person's own arrangement, whatever Zen mode shows meanwhile.
    let own = crate::settings::settings_without_zen(cx);
    let row = own.title_row().into_owned();
    let bar = own.status_bar;
    let list = SharedString::from(LIST);
    let shown: Vec<AnyElement> = row
        .parts()
        .iter()
        .enumerate()
        .map(|(ix, &part)| {
            let reorder = Reorder {
                list: list.clone(),
                ix,
                label: text(parts::label(part)).into(),
                on_move: Rc::new(|from, to, _, cx| update_settings(cx, |s| s.move_title_part(from, to))),
                dragged: Reorder::dragging(&list, ix, cx),
            };
            part_row(ui, part, true, bar, Some(reorder)).into_any_element()
        })
        .collect();
    let hidden: Vec<AnyElement> =
        row.hidden().map(|part| part_row(ui, part, false, bar, None).into_any_element()).collect();
    Some(
        div()
            .flex()
            .flex_col()
            .pb_2()
            .children(shown)
            .when(!hidden.is_empty(), |d| {
                d.child(div().pt_3().pb_1().pl_2().child(style::section_label(ui, text(TITLE_ROW_HIDDEN))))
                    .children(hidden)
            })
            .into_any_element(),
    )
}

/// One part: its icon and name, and its switch (the space has its rule instead, as it can't be
/// hidden). The terminal's status says, while the status bar is on (`bar`), that the bar shows it
/// instead.
fn part_row(ui: UiColors, part: TitlePart, shown: bool, bar: bool, reorder: Option<Reorder>) -> Div {
    let slot = TitlePart::ALL.iter().position(|p| *p == part).unwrap_or_default();
    let control = part.hideable().then(|| {
        style::toggle(("title-part-switch", slot), shown, move |on, _, cx| {
            update_settings(cx, |s| s.show_title_part(part, on))
        })
        .into_any_element()
    });
    let note = match part {
        TitlePart::Space => Some(text(TITLE_ROW_PART_SPACE_DESCRIPTION).into()),
        TitlePart::TerminalStatus if bar => Some(text(TITLE_ROW_STATUS_IN_BAR).into()),
        _ => None,
    };
    style::arrange_row(
        ("title-part", slot),
        ui,
        ArrangeParts {
            icon: parts::icon(part),
            label: text(parts::label(part)).into(),
            note,
            quiet: !shown,
            control,
            reorder,
        },
    )
}
