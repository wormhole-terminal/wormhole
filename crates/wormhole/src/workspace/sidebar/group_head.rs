//! A group's head in the list: its tiles and dot while folded, its + while pinned and empty, its
//! label's popover (`groups`), its drag, its drop and its context menu (New terminal in group,
//! Rename…, Colour ▸, Ungroup, Pin or Unpin group, Close group, Move group to new window).
use wormhole_core::sidebar::{GroupColor, Urgency, spoken};

use super::super::focus::SidebarStop;
use super::groups::{group_colors, group_popover};
use super::rows::{Frame, glyph_for, tile_for};
use super::*;
use crate::native::{self, Icon, Item, Menu};
use crate::style::{ClickHandler, GroupHead};
use crate::theme_bridge;

impl Workspace {
    /// A group's head: the tiles of its terminals while folded, the dot while one needs you or
    /// failed, a + when it is pinned and empty.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_group_band(
        &self,
        listing: &Listing,
        infos: &[RowInfo],
        band_ix: usize,
        (position, of): (usize, usize),
        frame: &Frame,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let band = &listing.bands[band_ix];
        let group = band.group;
        let color = self.sidebar.view.groups.get(group).and_then(|g| g.color());
        let tokens = &store_in(&self.profile, cx).tokens;
        let hsla = color.map(|color| theme_bridge::hsla(color.color(tokens)));
        let folded = band.folded;
        // One order of urgency for the dot and for the tiles (core's, the same the tab switcher and
        // the status bar pick by).
        let urgency_of = |ix: usize| Urgency::from(glyph_for(&infos[ix]));
        let alerts = band.members.iter().filter(|&&ix| urgency_of(ix).is_alert()).count();
        let alert = match band.members.iter().map(|&ix| urgency_of(ix)).min() {
            Some(Urgency::NeedsYou) => Some(ui.agent_attention),
            Some(Urgency::Failed) => Some(ui.danger),
            _ => None,
        };
        let tiles: Vec<AnyElement> = if folded {
            // The loudest first; terminals alike keep the list's order.
            let mut members = band.members.clone();
            members.sort_by_key(|&ix| urgency_of(ix));
            members
                .iter()
                .map(|&ix| tile_for(&infos[ix], frame, false, frame.metrics.folded_tile, cx))
                .collect()
        } else {
            Vec::new()
        };
        let this = cx.weak_entity();
        let (on_toggle, on_add, on_popover) = (this.clone(), this.clone(), this);
        let empty = (band.pinned && band.members.is_empty()).then(|| {
            let tooltip = SharedString::from(spoken::new_in_group_label(&band.label));
            let add: ClickHandler = Box::new(move |_, window, cx| {
                on_add.update(cx, |this, cx| this.new_session_in_group(group, window, cx)).ok();
            });
            (tooltip, add)
        });
        let snapshot = self.popover_snapshot(group, ui, cx);
        let parts = GroupHead {
            metrics: frame.metrics,
            name: band.label.clone().into(),
            color: hsla,
            folded,
            pinned: band.pinned,
            tiles,
            count: band.members.len(),
            alert,
            empty,
            on_toggle: Box::new(move |_, _, cx| {
                on_toggle.update(cx, |this, cx| this.toggle_group_band(group, cx)).ok();
            }),
            spoken_edit: spoken::edit_label(&band.label).into(),
            spoken_fold: spoken::fold_label(&band.label, !folded).into(),
            wrap_label: Box::new(move |trigger| group_popover(snapshot, trigger, on_popover)),
        };
        let id: ElementId = ElementId::Name(SharedString::from(format!("band-{}", band.key())));
        let mut label = spoken::band_label(
            &band.label,
            band.pinned,
            color.map(GroupColor::label),
            band.members.len(),
            alerts,
            !folded,
            position,
            of,
        );
        if alerts > 0 && folded {
            label = format!("{label}, {}", wormhole_core::spoken::terminals_need_you(alerts));
        }
        let (keyed, ring) = self.sidebar_keyed(&SidebarStop::Group(group), window);
        let menu_open = self.sidebar.menu_group == Some(group);
        let element = style::group_head(id, ui, parts)
            .role(Role::TreeItem)
            .aria_label(label)
            .aria_expanded(!folded)
            .aria_level(1)
            .aria_position_in_set(position)
            .aria_size_of_set(of)
            .when(keyed, |d| d.aria_active_descendant())
            .when(ring || menu_open, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(frame.metrics.row_radius)))
            });
        let element = self.with_group_drag(element, group, band.label.clone().into(), hsla, cx);
        element
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |ws, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    ws.open_group_menu(group, event.position, window, cx);
                }),
            )
            .into_any_element()
    }

    /// ⌃↩ with the keys on group `group`'s head: its menu, under it.
    pub(super) fn group_key_menu(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let listing = self.session_listing(cx);
        // The list's children are its entries, in order.
        let at = listing.entries.iter().position(
            |entry| matches!(entry, Entry::Band { band, .. } if listing.bands[*band].group == group),
        );
        let Some(bounds) = at.and_then(|at| self.sidebar.scroll.bounds_for_item(at)) else { return };
        self.open_group_menu(group, super::super::focus::key_menu_origin(bounds), window, cx);
    }

    /// Open group `group`'s menu at `at`, its head wearing the ring until the menu closes.
    fn open_group_menu(
        &mut self,
        group: u32,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let found = self.sidebar.view.groups.get(group);
        let facts = GroupFacts {
            group,
            color: found.and_then(|g| g.color()),
            pinned: found.is_some_and(|g| g.pinned),
            colors: group_colors(&self.profile, cx),
        };
        self.sidebar.menu_row = None;
        self.sidebar.menu_group = Some(group);
        cx.notify();
        let this = cx.weak_entity();
        let ring = this.clone();
        let menu = group_menu(facts, &this).on_close(move |_, cx| {
            ring.update(cx, |ws, cx| {
                if ws.sidebar.menu_group == Some(group) {
                    ws.sidebar.menu_group = None;
                    cx.notify();
                }
            })
            .ok();
        });
        native::show_menu(menu, at, window, cx);
    }

    /// The groups a terminal's menu offers to move it to: every group, with its name and whether
    /// the terminal is in it.
    pub(super) fn group_choices(&self, id: i64) -> Vec<(u32, SharedString, bool)> {
        let current = self.sidebar.view.groups.group_of(id);
        self.sidebar
            .view
            .groups
            .iter()
            .map(|group| (group.id, SharedString::from(group.name.clone()), current == Some(group.id)))
            .collect()
    }
}

/// What a group's menu is made from, as it is when the menu opens.
struct GroupFacts {
    group: u32,
    /// The group's colour, if it has one.
    color: Option<GroupColor>,
    pinned: bool,
    /// Every colour on offer, with how the theme draws it.
    colors: Vec<(GroupColor, Hsla)>,
}

/// The menu of the group `facts` describe; every item acts on `this`.
fn group_menu(facts: GroupFacts, this: &WeakEntity<Workspace>) -> Menu {
    let GroupFacts { group, color: current, pinned, colors } = facts;
    let swatches = colors.iter().fold(Menu::new(), |menu, &(color, hsla)| {
        menu.item(
            Item::new(color.label())
                .icon(Some(Icon::Swatch(hsla)))
                .checked(current == Some(color))
                .on_select(native::run_on(this, move |ws, _, cx| ws.set_group_color(group, Some(color), cx))),
        )
    });
    Menu::new()
        .item(Item::new("New terminal in group").on_select(native::run_on(this, move |ws, window, cx| {
            ws.new_session_in_group(group, window, cx)
        })))
        .item(
            Item::new("Rename…").on_select(native::run_on(this, move |ws, window, cx| {
                ws.start_group_edit(group, window, cx)
            })),
        )
        .submenu("Colour", swatches)
        .separator()
        .item(
            Item::new(if pinned { "Unpin group" } else { "Pin group" })
                .on_select(native::run_on(this, move |ws, _, cx| ws.toggle_group_pin(group, cx))),
        )
        .item(Item::new("Ungroup").on_select(native::run_on(this, move |ws, _, cx| ws.ungroup(group, cx))))
        .item(Item::new("Move group to new window").on_select(native::run_on(this, move |ws, window, cx| {
            ws.move_group_to_window(group, window, cx)
        })))
        .separator()
        .item(
            Item::new("Close group")
                .on_select(native::run_on(this, move |ws, window, cx| ws.close_group(group, window, cx))),
        )
}

#[cfg(test)]
mod tests {
    use gpui_kit::{WeakEntity, hsla};
    use wormhole_core::sidebar::GroupColor;

    use super::{GroupFacts, group_menu};

    #[test]
    fn the_colour_of_the_group_is_checked_and_pinned_reads_unpin() {
        let facts = GroupFacts {
            group: 1,
            color: Some(GroupColor::Blue),
            pinned: true,
            colors: vec![
                (GroupColor::Grey, hsla(0.0, 0.0, 0.5, 1.0)),
                (GroupColor::Blue, hsla(0.6, 0.8, 0.5, 1.0)),
            ],
        };
        assert_eq!(
            group_menu(facts, &WeakEntity::new_invalid()).outline(),
            [
                "- New terminal in group",
                "- Rename…",
                "> Colour",
                "  - Grey",
                "  - Blue [x]",
                "---",
                "- Unpin group",
                "- Ungroup",
                "- Move group to new window",
                "---",
                "- Close group",
            ]
        );
    }
}
