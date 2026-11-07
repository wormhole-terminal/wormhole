//! The marker row of a fold: one line in the terminal's text, drawn by the app and never from output
//! text, that says how many lines are hidden and opens them on a click. Built in the canvas's prepaint
//! from the frame's own snapshot (so it is never a frame behind the grid) and painted over the row the
//! snapshot left blank for it. It is a real accessibility node: a button named with the count, whose
//! default action (VoiceOver's Show) is the click.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_term::ScreenSnapshot;
use wormhole_term::folds::FoldOwner;
use wormhole_text::{keys, whole};

use super::frame::Geometry;
use crate::settings::UiColors;
use crate::style::{self, TextRole};
use crate::terminal_view::TerminalView;

/// What a marker says, what VoiceOver says and its tooltip: the line ("124 lines hidden"), then a Probe's own
/// fixed label (never text of the output) with the Probe's name in what is spoken and in the tooltip, then,
/// while a search is open, how many matches are inside.
fn words(line: String, owner: Option<&FoldOwner>, matches: usize) -> (String, String, Option<String>) {
    let (mut label, mut spoken) = (line.clone(), line);
    if let Some(owner) = owner {
        spoken = keys::terminal_fold_spoken_by(&spoken, &owner.name, &owner.label);
        label = keys::terminal_fold_labelled(&label, &owner.label);
    }
    if matches > 0 {
        let inside = keys::terminal_fold_matches(whole(matches));
        label = keys::terminal_fold_with_matches(&label, &inside);
        spoken = keys::terminal_fold_with_matches(&spoken, &inside);
    }
    (label, spoken, owner.map(|owner| keys::terminal_fold_tooltip_by(&owner.name)))
}

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
        let count = whole(fold.hidden());
        let (line, glyph) = if fold.expanded {
            (keys::terminal_fold_shown(count), IconName::ChevronDown)
        } else {
            (keys::terminal_fold_hidden(count), IconName::ChevronRight)
        };
        let (label, spoken, tooltip) =
            words(line, fold.owner.as_deref(), found.get(row).copied().unwrap_or(0));
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

#[cfg(test)]
mod tests {
    use super::{FoldOwner, words};

    #[test]
    fn a_probes_marker_carries_its_label_and_only_the_spoken_words_and_tooltip_its_name() {
        let owner =
            FoldOwner { probe: "acme".into(), name: "Acme tools".into(), label: "build output".into() };
        let (label, spoken, tooltip) = words("124 lines hidden".into(), Some(&owner), 0);
        assert_eq!(label, "124 lines hidden · build output");
        assert_eq!(spoken, "124 lines hidden by Acme tools, build output");
        assert_eq!(tooltip.as_deref(), Some("Folded by Acme tools"));
        let (label, spoken, tooltip) = words("124 lines hidden".into(), None, 2);
        assert_eq!(label, "124 lines hidden · 2 matches inside");
        assert_eq!(spoken, label);
        assert_eq!(tooltip, None, "the person's own fold has no author to name");
    }
}
