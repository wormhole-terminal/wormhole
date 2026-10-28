//! The button at the top of the card: the shown folder's icon, its name and a chevron. It opens the path
//! menu under itself, on a click and on ↩ with the keys on it.

use std::path::Path;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::file_tree::names::clean;
use wormhole_core::view::sidebar::display_path;
use wormhole_text::keys;

use crate::platform::Anchor;
use crate::settings::UiColors;
use crate::style::{self, BAND_H, CARD_INSET, CARD_ROW_RADIUS, TextRole as _};

/// What the button says: the folder's name (the disk's, for `/`) and its path, as `~/…` where it can.
pub(super) struct HeadFacts {
    pub name: String,
    pub path: String,
}

impl HeadFacts {
    pub(super) fn of(folder: &Path, volume: impl FnOnce() -> String) -> Self {
        let name = match folder.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => volume(),
        };
        Self { name: clean(&name), path: clean(&display_path(folder, style::home_dir())) }
    }
}

/// The path button. `at` is told where it is drawn, for the menu the keys open; `on_click` opens the
/// menu under it.
#[track_caller]
pub(super) fn path_button(
    ui: UiColors,
    facts: &HeadFacts,
    icon: AnyElement,
    at: &Anchor,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let path = SharedString::from(facts.path.clone());
    div()
        .id("files-path")
        .debug_selector(|| "files-path".into())
        .role(Role::Button)
        .aria_label(SharedString::from(keys::files_spoken_path_button(&facts.name)))
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(BAND_H))
        .px(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .cursor_pointer()
        .hover(move |d| d.bg(ui.hover))
        .on_click(on_click)
        .tooltip(move |window, cx| Tooltip::new(path.clone()).build(window, cx))
        .child(style::lead(icon))
        .child(div().min_w_0().truncate().text_label().text_color(ui.text).child(facts.name.clone()))
        .child(Icon::new(IconName::ChevronDown).xsmall().text_color(ui.muted).flex_none())
        .child(at.probe())
        .into_any_element()
}
