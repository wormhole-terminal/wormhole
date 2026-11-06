//! The text fields: one line and several, the states a field has, and the search field of the nav.

use gpui_kit::assets::IconName;
use gpui_kit::*;

use super::{Sheet, category, specimen};
use crate::style::{self, TextRole};

pub(super) fn draw(sheet: &Sheet, _cx: &App) -> AnyElement {
    let ui = sheet.ui;
    let now = &sheet.now;
    let row = |items: Vec<AnyElement>| div().flex().flex_wrap().items_center().gap_3().children(items);
    let states = row(vec![
        style::text_field(ui, &now.hint).w(style::FIELD_W_PAIR).into_any_element(),
        style::text_field(ui, &now.filled).w(style::FIELD_W_PAIR).into_any_element(),
        style::text_field(ui, &now.focused).w(style::FIELD_W_PAIR).show_focus(true).into_any_element(),
        div()
            .text_caption()
            .text_color(ui.muted)
            .child("hint, text, focus outline (click one to type)") // text: sample (a developer label)
            .into_any_element(), // text: sample (a note)
    ]);
    let decorated = row(vec![
        style::text_field(ui, &now.prefixed)
            .w(style::FIELD_W)
            .prefix(style::icon(IconName::Search).text_color(ui.muted))
            .into_any_element(),
        style::text_field(ui, &now.search)
            .w(style::FIELD_W)
            .prefix(style::icon(IconName::Search).text_color(ui.muted))
            .cleanable(true)
            .into_any_element(),
        style::text_field(ui, &now.letter).w(style::FIELD_W_LETTER).into_any_element(),
    ]);
    let nav_search = div().w(px(crate::settings_window::nav::NAV_W)).child(
        style::text_field(ui, &now.search)
            .cleanable(true)
            .prefix(style::icon(IconName::Search).text_color(ui.muted))
            .suffix(div().text_caption().tabular_nums().text_color(ui.muted).child("12")), // text: sample (a hit count)
    );
    category(
        ui,
        "Text fields",         // text: sample (a category's head)
        "style/text_field.rs", // text: sample (a category's head)
        [
            specimen(ui, "text_field · empty, filled, focused", states).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "text_field · prefix, cleanable, one letter wide", decorated).into_any_element(), // text: sample (a specimen's name)
            specimen(
                ui,
                "TextField::multiline", // text: sample (a specimen's name)
                style::TextField::multiline(ui, &now.lines).w(style::FIELD_W_BOX),
            )
            .into_any_element(),
            specimen(ui, "nav search · text_field with prefix, clear and a hit count", nav_search) // text: sample (a developer label)
                .into_any_element(), // text: sample (a specimen's name)
        ],
    )
    .into_any_element()
}
