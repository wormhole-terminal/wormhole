//! A split in the terminal list: every terminal of a split is a row of its own. Next to each other
//! (in no group) they share one card; a chain mark on each says they belong together when they
//! aren't next to each other. The row that has the selected look stays one: the focused terminal's.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon;
use crate::settings::UiColors;
use gpui_kit::assets::IconName;

/// The chain mark on a member of a split whose other members are elsewhere in the list (another
/// folder's band between them), at the right of its first line.
#[track_caller]
pub fn chain_mark(id: impl Into<ElementId>, ui: UiColors) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .text_color(ui.muted)
        .child(icon(IconName::Link2))
        .tooltip(|window, cx| Tooltip::new("In a split with terminals in other folders").build(window, cx))
}
