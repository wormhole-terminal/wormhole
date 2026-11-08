//! The marker row of a fold: one line in the terminal's text, drawn by the app and never from output
//! text, that says how many lines are hidden and opens them on a click. Built in the canvas's prepaint
//! from the frame's own snapshot (so it is never a frame behind the grid) and painted over the row the
//! snapshot left blank for it. It is a real accessibility node: a button named with the count, whose
//! default action (VoiceOver's Show) is the click.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::fold::{MarkerWords, marker_words};
use wormhole_term::ScreenSnapshot;

use super::frame::Geometry;
use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::terminal_view::TerminalView;

/// The markers in view, laid out and ready to paint (none, and nothing allocated, without folds).
pub(super) fn markers(
    snapshot: &ScreenSnapshot,
    found: &[usize],
    geometry: Geometry,
    ui: UiColors,
    entity: &Entity<TerminalView>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let width = geometry.cell_width * snapshot.columns() as f32;
    let mut markers = Vec::new();
    for (row, fold) in
        snapshot.rows().iter().enumerate().filter_map(|(row, shown)| Some((row, shown.fold()?)))
    {
        let glyph = if fold.expanded { IconName::ChevronDown } else { IconName::ChevronRight };
        let owner = fold.owner.as_deref().map(|owner| (owner.name.as_str(), owner.label.as_str()));
        let MarkerWords { label, spoken, tooltip } =
            marker_words(fold.hidden(), fold.expanded, owner, found.get(row).copied().unwrap_or(0));
        let id = fold.id;
        let entity = entity.clone();
        let mut marker = div()
            .id(("fold", id))
            .role(Role::Button)
            .aria_label(spoken)
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .bg(ui.hover)
            .text_caption()
            .text_color(if fold.failed { ui.danger } else { ui.muted })
            .cursor_pointer()
            .when_some(tooltip, |marker, tooltip| {
                marker.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            })
            // The pane below it would start a selection.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, _, cx| {
                entity.update(cx, |this, cx| this.toggle_fold(id, cx));
            })
            .child(style::icon(glyph))
            .child(label)
            .into_any_element();
        let origin = point(geometry.origin.x, geometry.origin.y + geometry.line_height * row as f32);
        let space = size(AvailableSpace::Definite(width), AvailableSpace::Definite(geometry.line_height));
        marker.prepaint_as_root(origin, space, window, cx);
        markers.push(marker);
    }
    markers
}
