//! The marker row of a fold: one line in the terminal's text, drawn by the app and never from output
//! text, that says how many lines are hidden and opens them on a click. Built in the canvas's prepaint
//! from the frame's own snapshot (so it is never a frame behind the grid) and painted over the row the
//! snapshot left blank for it. It is a real accessibility node: a button named with the count, whose
//! default action (VoiceOver's Show) is the click.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_term::ScreenSnapshot;
use wormhole_text::{keys, whole};

use super::frame::Geometry;
use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::terminal_view::TerminalView;

/// The markers in view, laid out and ready to paint (none, and nothing allocated, without folds).
pub(super) fn markers(
    snapshot: &ScreenSnapshot,
    geometry: Geometry,
    cols: usize,
    ui: UiColors,
    entity: &Entity<TerminalView>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<AnyElement> {
    let width = geometry.cell_width * cols as f32;
    let mut markers = Vec::new();
    for (row, fold) in
        snapshot.rows().iter().enumerate().filter_map(|(row, shown)| Some((row, shown.fold()?)))
    {
        let count = whole(fold.hidden());
        let (label, glyph) = if fold.expanded {
            (keys::terminal_fold_shown(count), IconName::ChevronDown)
        } else {
            (keys::terminal_fold_hidden(count), IconName::ChevronRight)
        };
        let id = fold.id;
        let entity = entity.clone();
        let mut marker = div()
            .id(("fold", id))
            .role(Role::Button)
            .aria_label(label.clone())
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .bg(ui.hover)
            .text_caption()
            .text_color(if fold.failed { ui.danger } else { ui.muted })
            .cursor_pointer()
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
