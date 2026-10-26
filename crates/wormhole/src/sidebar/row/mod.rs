//! One terminal's row: the lead glyph or agent logo, the name
//! at x 40, the second line (what the agent does, the command in front, where it is), what
//! sits at x 334 (⌘N keycap, status glyph on the right, bell, close ×), and the states laid
//! over it (its context menu's focus ring, a drag, the flash after a move). A terminal in a
//! split also shows it: the chain mark at the right of its first line when its mates are elsewhere
//! (rows of one split in no group share a card; VoiceOver is told they are on screen).

mod face;
mod frame;
mod label;
mod second_line;
mod shift;
mod tile;

use std::time::Duration;

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::hints::keyshortcut;

use self::face::RowFace;
pub(super) use self::face::{Place, row_face};
pub(super) use self::frame::Frame;
pub(super) use self::second_line::{PLACE_CHARS, line_of, time_text};
pub(super) use self::shift::{RowShift, Was};
pub(super) use self::tile::{glyph_for, tile_for, tile_parts};

use super::facts::RowInfo;
use super::{Sidebar, SidebarEvent, two_line_tooltip};
use crate::settings::UiColors;
use crate::sidebar::SidebarStop;
use crate::sidebar::terminal_drag::DragOrigin;
use crate::style;

/// How long the row that moved to another band stays highlighted, fading out.
const MOVE_FLASH: Duration = Duration::from_millis(600);
/// The close ×: drawn 22 square, this far under the row's top.
const CLOSE_X_TOP: f32 = 3.0;

/// How far the × stands in from the row's right side: its glyph ends where line 1 ends (the row's
/// own padding, and a lone card's), on the right edge of what sits there at rest.
fn close_right(metrics: &style::Metrics, solo: bool) -> f32 {
    let line_end = metrics.row_px + if solo { metrics.card_pad } else { 0.0 };
    line_end - (style::CONTROL_H_SM - style::GLYPH) / 2.0
}

impl Sidebar {
    pub(in crate::sidebar) fn render_row(
        &self,
        info: &RowInfo,
        place: &Place,
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let id = info.id;
        let selected = place.selected;
        let RowFace { row, tooltip, label, group } =
            row_face(info, place, frame, self.rename_field(id, cx), window, cx);
        let flash = self.flash.filter(|(flashed, _)| *flashed == id).map(|(_, generation)| {
            div().absolute().top_0().left_0().size_full().bg(ui.highlight).with_animation(
                ("move-flash", generation),
                Animation::new(MOVE_FLASH),
                |d, t| d.opacity(1.0 - t),
            )
        });
        let dragged = cx.has_active_drag()
            && self.input.terminal_drag.is_some_and(|(drag, origin)| drag == id && origin == DragOrigin::Row);
        // The keys' place: VoiceOver follows it, and the ring shows while keys move it.
        let (keyed, ring) = self.keyed(&SidebarStop::Row(id), window);
        let row = row
            .group(group.clone())
            .relative()
            .role(Role::TreeItem)
            .aria_label(label)
            .aria_selected(selected)
            .aria_level(if place.grouped { 2 } else { 1 })
            .aria_position_in_set(place.position)
            .aria_size_of_set(place.of)
            .when_some(place.number, |d, n| d.aria_keyshortcuts(keyshortcut(n)))
            .when(keyed, |d| d.aria_active_descendant())
            .when(dragged, |d| d.opacity(style::DRAG_LIFT_OPACITY))
            .on_click(
                cx.listener(move |_, _, _, cx| {
                    cx.emit(SidebarEvent::Activate { id, then_focus_list: false })
                }),
            )
            .when_some(tooltip, |d, (name, second)| {
                d.tooltip(move |window, cx| two_line_tooltip(name.clone(), second.clone(), ui, window, cx))
            })
            .children(flash)
            .when(self.menu_row == Some(id) || ring, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(style::CARD_ROW_RADIUS)))
            })
            .when(!frame.hints, |d| {
                d.child(self.close_button(
                    info,
                    selected,
                    &group,
                    close_right(&frame.metrics, place.solo),
                    ui,
                    cx,
                ))
            })
            .map(|d| self.with_drag(d, info, cx));
        // Last: the menu wraps the row.
        self.with_row_menu(row, info, cx).into_any_element()
    }

    /// The × that closes a terminal: on the row under the pointer, the selected one too.
    pub(in crate::sidebar) fn close_button(
        &self,
        info: &RowInfo,
        selected: bool,
        group: &SharedString,
        right: f32,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = info.id;
        let fill = if selected { style::SelectedFill::Tint } else { style::SelectedFill::Plain };
        // No tooltip while a close question is up: the pointer still rests on the ×, and GPUI
        // would show (or keep) its tooltip beside the dialog. Dropping it clears what is pending.
        let asking = self.input.asking;
        div()
            .absolute()
            .top(px(CLOSE_X_TOP))
            .right(px(right))
            .size(px(style::CONTROL_H_SM))
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .child(style::hit_area(
                ("close-hit", info.id as u64),
                style::CONTROL_H_SM,
                style::close_x(("close", info.id as u64), ui, fill),
                |hit| {
                    hit.role(Role::Button)
                        .aria_label("Close terminal")
                        .when(!asking, |d| {
                            d.tooltip(|window, cx| Tooltip::new("Close terminal").build(window, cx))
                        })
                        .on_click(cx.listener(move |_, _, _, cx| {
                            cx.stop_propagation();
                            cx.emit(SidebarEvent::Close(id));
                        }))
                },
            ))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
