//! Buttons as the pages use them: GPUI Kit's variants at the sizes of a form, our icon button, and links.

use gpui_kit::assets::IconName;
use gpui_kit::component::Disableable as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::{Sheet, category, specimen};
use crate::style::{self, TextRole};

pub(super) fn draw(sheet: &Sheet, _cx: &App) -> AnyElement {
    let ui = sheet.ui;
    let line = |items: Vec<AnyElement>| div().flex().flex_wrap().items_center().gap_2().children(items);
    let kit = line(vec![
        Button::new("components-b-primary").primary().small().label("Primary").into_any_element(), // text: sample (a button's label)
        Button::new("components-b-outline").outline().small().label("Outline").into_any_element(), // text: sample (a button's label)
        Button::new("components-b-ghost").ghost().small().label("Ghost").into_any_element(), // text: sample (a button's label)
        Button::new("components-b-danger").danger().outline().small().label("Danger").into_any_element(), // text: sample (a button's label)
        Button::new("components-b-link").link().small().label("Link").into_any_element(), // text: sample (a button's label)
    ]);
    let with_icon = line(vec![
        Button::new("components-b-icon")
            .outline()
            .small()
            .icon(IconName::RefreshCw)
            .label("Rescan PATH") // text: sample (a developer label)
            .into_any_element(), // text: sample (a button's label)
        Button::new("components-b-plus")
            .outline()
            .small()
            .icon(IconName::Plus)
            .label("New profile") // text: sample (a developer label)
            .into_any_element(), // text: sample (a button's label)
        Button::new("components-b-loading")
            .outline()
            .small()
            .icon(IconName::RefreshCw)
            .label("Scanning") // text: sample (a developer label)
            .loading(true)
            .into_any_element(), // text: sample (a button's label)
        Button::new("components-b-disabled")
            .outline()
            .small()
            .label("Disabled") // text: sample (a developer label)
            .disabled(true)
            .into_any_element(), // text: sample (a button's label)
    ]);
    let icons = line(vec![
        style::icon_button("components-ib", ui, IconName::Copy, "Copy").into_any_element(), // text: sample (a tooltip)
        style::icon_button("components-ib-marked", ui, IconName::Pin, "Pin").marked(true).into_any_element(), // text: sample (a tooltip)
        style::icon_button("components-ib-disabled", ui, IconName::Trash, "Delete") // text: sample (a developer label)
            .disabled(true)
            .into_any_element(), // text: sample (a tooltip)
        style::icon_button("components-ib-ring", ui, IconName::Download, "Download") // text: sample (a developer label)
            .focus_ring(true)
            .into_any_element(), // text: sample (a tooltip)
        div()
            .text_caption()
            .text_color(ui.muted)
            .child("normal, marked, disabled, with the focus ring") // text: sample (a developer label)
            .into_any_element(), // text: sample (a note)
    ]);
    let links = line(vec![
        style::link_button("components-l-plain", ui, "Edit in project", false).into_any_element(), // text: sample (a link)
        style::link_button("components-l-arrow", ui, "Edit in Fonts", true).into_any_element(), // text: sample (a link)
    ]);
    category(
        ui,
        "Buttons",                                          // text: sample (a category's head)
        "GPUI Kit's Button, style/buttons.rs, link_button", // text: sample (a category's head)
        [
            specimen(ui, "Button · small, the variants the pages use", kit).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "Button · with an icon, loading, disabled", with_icon).into_any_element(), // text: sample (a specimen's name)
            specimen(ui, "icon_button", icons).into_any_element(),
            specimen(ui, "link_button", links).into_any_element(),
        ],
    )
    .into_any_element()
}
