//! A group's head in the list: its tiles and dot while folded, its + (always while pinned and empty, else under the pointer), its
//! drag, its drop, the popover its name opens (`group_popover`) and its context menu, which a
//! right-click and ⌃↩ open: Rename… (in place, `rename`), a row of the nine colours as Finder's tags
//! are in its context menu, Other colour…, its folder with Set folder… and Clear folder, and the
//! actions. Resting on the name shows the folder.
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::sidebar::{Entry, Listing};
use wormhole_core::sidebar::{Urgency, spoken};

use super::super::focus::SidebarStop;
use super::facts::RowInfo;
use super::group_popover::{group_menu, group_popover};
use super::rows::{Frame, glyph_for, tile_for};
use crate::platform::{self, Anchor};
use crate::settings::store_in;
use crate::style;
use crate::style::{ClickHandler, GroupHead};
use crate::theme_bridge;
use crate::workspace::Workspace;

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
        let color = self.sidebar.view.groups.get(group).and_then(|g| g.paint());
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
        let (on_toggle, on_add, on_label) = (this.clone(), this.clone(), this);
        // The popover's arrow points at the name, which the probe on it measures.
        let label_at = Anchor::default();
        let label_anchor = label_at.clone();
        // The + is the same button for a pinned, empty group (always shown) and any other head
        // (under the pointer): New terminal in group, which starts in the group's folder.
        let tooltip = SharedString::from(spoken::new_in_group_label(&band.label));
        let add: ClickHandler = Box::new(move |_, window, cx| {
            on_add.update(cx, |this, cx| this.new_session_in_group(group, window, cx)).ok();
        });
        let (empty, add) = if band.pinned && band.members.is_empty() {
            (Some((tooltip, add)), None)
        } else {
            (None, Some((tooltip, add)))
        };
        let folder = self.group_folder_label(group);
        let parts = GroupHead {
            metrics: frame.metrics,
            name: band.label.clone().into(),
            tooltip: folder.clone().map(Into::into),
            folder: self.group_folder_line(group).map(Into::into),
            color: hsla,
            folded,
            pinned: band.pinned,
            tiles,
            count: band.members.len(),
            alert,
            empty,
            add,
            on_toggle: Box::new(move |_, _, cx| {
                on_toggle.update(cx, |this, cx| this.toggle_group_band(group, cx)).ok();
            }),
            spoken_edit: spoken::edit_label(&band.label).into(),
            spoken_fold: spoken::fold_label(&band.label, !folded).into(),
            on_label: Some(Box::new(move |_, window, cx| {
                let at = label_at.bounds();
                on_label.update(cx, |this, cx| this.open_group_popover(group, at, window, cx)).ok();
            })),
            label_anchor: Some(label_anchor),
            name_edit: self.group_rename_field(group, cx),
        };
        let id: ElementId = ElementId::Name(SharedString::from(format!("band-{}", band.key())));
        let mut label = spoken::band_label(
            &band.label,
            band.pinned,
            color.map(wormhole_core::sidebar::GroupPaint::label),
            folder.as_deref(),
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
    pub(super) fn open_group_menu(
        &mut self,
        group: u32,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let facts = self.group_facts(group, cx);
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
        platform::show_menu(menu, at, window, cx);
    }

    /// Open group `group`'s popover under its name (`anchor`): the name in a field, the colours, and
    /// the actions.
    pub(super) fn open_group_popover(
        &mut self,
        group: u32,
        anchor: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.sidebar.view.groups.get(group).map(|g| g.name.clone()) else { return };
        let facts = self.group_facts(group, cx);
        let appearance = store_in(&self.profile, cx).tokens.appearance;
        let popover = group_popover(facts, name, appearance, &cx.weak_entity());
        platform::show_popover(popover, anchor, window, cx);
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

#[cfg(test)]
mod tests {
    use gpui_kit::{WeakEntity, hsla};
    use wormhole_core::sidebar::{GroupColor, GroupPaint};

    use super::super::group_popover::{GroupFacts, group_menu};

    fn facts(color: Option<GroupColor>, pinned: bool, folder: Option<&str>) -> GroupFacts {
        GroupFacts {
            group: 1,
            paint: color.map(GroupPaint::Named),
            custom: None,
            shown: None,
            pinned,
            folder: folder.map(str::to_string),
            colors: vec![
                (GroupColor::Grey, hsla(0.0, 0.0, 0.5, 1.0)),
                (GroupColor::Blue, hsla(0.6, 0.8, 0.5, 1.0)),
            ],
        }
    }

    #[test]
    fn the_menu_has_the_rename_the_colour_row_and_the_actions_and_pinned_reads_unpin() {
        assert_eq!(
            group_menu(facts(Some(GroupColor::Blue), true, None), &WeakEntity::new_invalid()).outline(),
            [
                "- Rename…",
                "---",
                "~ Grey | Blue [x]",
                "- Other colour…",
                "- No colour",
                "---",
                "- Set folder…",
                "---",
                "- New terminal in group",
                "- Ungroup",
                "- Close group",
                "- Unpin group",
                "- Move group to new window",
            ]
        );
    }

    #[test]
    fn a_group_with_a_folder_shows_it_greyed_out_above_set_and_clear() {
        let outline =
            group_menu(facts(None, false, Some("~/Projects/foo")), &WeakEntity::new_invalid()).outline();
        assert_eq!(
            outline[3..10],
            [
                "- Other colour…",
                "- No colour [x]",
                "---",
                "- ~/Projects/foo [off]",
                "- Set folder…",
                "- Clear folder",
                "---"
            ],
            "{outline:?}"
        );
        let none = group_menu(facts(None, false, None), &WeakEntity::new_invalid()).outline();
        assert!(none.iter().all(|line| !line.contains("Clear folder")), "{none:?}");
    }

    #[test]
    fn a_group_with_a_colour_offers_no_colour_unchecked() {
        let outline =
            group_menu(facts(Some(GroupColor::Blue), false, None), &WeakEntity::new_invalid()).outline();
        assert_eq!(outline[4], "- No colour");
    }

    #[test]
    fn a_group_without_a_colour_checks_no_colour_and_unpinned_reads_pin() {
        let outline = group_menu(facts(None, false, None), &WeakEntity::new_invalid()).outline();
        assert_eq!(outline[2], "~ Grey | Blue");
        assert_eq!(outline[3], "- Other colour…");
        assert_eq!(outline[4], "- No colour [x]");
        assert!(outline.contains(&"- Pin group".to_string()));
    }
}
