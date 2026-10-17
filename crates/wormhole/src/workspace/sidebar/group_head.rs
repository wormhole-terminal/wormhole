//! A group's head in the list: its tiles and dot while folded, its + while pinned and empty, its
//! label's popover (`groups`), its drag, its drop and its context menu (New terminal in group,
//! Rename…, Colour ▸, Ungroup, Pin or Unpin group, Close group, Move group to new window).
use std::rc::Rc;

use gpui_kit::component::menu::{ContextMenu, ContextMenuExt as _, PopupMenu, PopupMenuItem};
use wormhole_core::sidebar::{GroupColor, Urgency, spoken};

use super::super::focus::{MenuBuilder, SidebarStop};
use super::groups::{group_colors, group_popover};
use super::rows::{Frame, glyph_for, tile_for};
use super::*;
use crate::settings::SettingsStore;
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
        let tokens = &cx.global::<SettingsStore>().tokens;
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
        let build = self.group_menu(group, cx);
        let menu: ContextMenu<Stateful<Div>> =
            element.context_menu(move |menu, window, cx| build(menu, window, cx));
        menu.into_any_element()
    }

    /// ⌃↩ with the keys on group `group`'s head: its menu, under it.
    pub(super) fn group_key_menu(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let listing = self.session_listing(cx);
        // The list's children are its entries, in order.
        let at = listing.entries.iter().position(
            |entry| matches!(entry, Entry::Band { band, .. } if listing.bands[*band].group == group),
        );
        let Some(bounds) = at.and_then(|at| self.sidebar.scroll.bounds_for_item(at)) else { return };
        let build = self.group_menu(group, cx);
        self.open_key_menu(wormhole_core::focus::Area::Sidebar, bounds, build, window, cx);
    }

    /// What builds group `group`'s menu.
    fn group_menu(&self, group: u32, cx: &mut Context<Self>) -> MenuBuilder {
        let this = cx.weak_entity();
        Rc::new(move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            let popup = cx.entity();
            let (current, pinned) = this
                .update(cx, |ws, cx| {
                    ws.sidebar.menu_row = None;
                    ws.sidebar.menu_group = Some(group);
                    ws.sidebar.menu_dismissed =
                        Some(cx.subscribe(&popup, move |ws, _, _: &DismissEvent, cx| {
                            if ws.sidebar.menu_group == Some(group) {
                                ws.sidebar.menu_group = None;
                                cx.notify();
                            }
                        }));
                    cx.notify();
                    let found = ws.sidebar.view.groups.get(group);
                    (found.and_then(|g| g.color()), found.is_some_and(|g| g.pinned))
                })
                .unwrap_or((None, false));
            let choices = group_colors(cx);
            let ui = ui(cx);
            let (add, rename, colors, ungroup, pin, close, window_move) = (
                this.clone(),
                this.clone(),
                this.clone(),
                this.clone(),
                this.clone(),
                this.clone(),
                this.clone(),
            );
            menu.item(PopupMenuItem::new("New terminal in group").on_click(move |_, window, cx| {
                add.update(cx, |ws, cx| ws.new_session_in_group(group, window, cx)).ok();
            }))
            .item(PopupMenuItem::new("Rename…").on_click(move |_, window, cx| {
                rename.update(cx, |ws, cx| ws.start_group_edit(group, window, cx)).ok();
            }))
            .submenu("Colour", window, cx, move |mut submenu, _, _| {
                for &(color, hsla) in &choices {
                    let this = colors.clone();
                    submenu = submenu.item(
                        PopupMenuItem::element(move |_, _| {
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(style::color_dot(ui, hsla, style::COLOR_DOT_MENU))
                                .child(color.label())
                        })
                        .checked(current == Some(color))
                        .on_click(move |_, _, cx| {
                            this.update(cx, |ws, cx| ws.set_group_color(group, Some(color), cx)).ok();
                        }),
                    );
                }
                submenu
            })
            .separator()
            .item(PopupMenuItem::new(if pinned { "Unpin group" } else { "Pin group" }).on_click(
                move |_, _, cx| {
                    pin.update(cx, |ws, cx| ws.toggle_group_pin(group, cx)).ok();
                },
            ))
            .item(PopupMenuItem::new("Ungroup").on_click(move |_, _, cx| {
                ungroup.update(cx, |ws, cx| ws.ungroup(group, cx)).ok();
            }))
            .item(PopupMenuItem::new("Move group to new window").on_click(move |_, window, cx| {
                window_move.update(cx, |ws, cx| ws.move_group_to_window(group, window, cx)).ok();
            }))
            .separator()
            .item(PopupMenuItem::new("Close group").on_click(move |_, window, cx| {
                close.update(cx, |ws, cx| ws.close_group(group, window, cx)).ok();
            }))
        })
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
