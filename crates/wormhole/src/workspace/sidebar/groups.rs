//! The person's own groups in the list: a group's head (its name on a pill of its colour, the count
//! while folded, the + for a new terminal in it), its menu (New terminal in group, Rename…,
//! Colour ▸, Ungroup), renaming it in place, and putting terminals in and out of groups (a row's
//! menu, a drop on a group's head or beside one of its rows). Core keeps the groups and their
//! order (`sidebar::WindowSidebar`, saved with the window); this draws them and changes them.
use std::rc::Rc;

use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenu, ContextMenuExt as _, PopupMenu, PopupMenuItem};
use wormhole_core::sidebar::{GroupColor, Item, Landing, spoken};

use super::super::focus::{MenuBuilder, SidebarStop};
use super::rows::Frame;
use super::*;
use crate::settings::SettingsStore;
use crate::style::{BandKind, BandParts, BandTint, FolderLook, Section};
use crate::theme_bridge;

/// A group being renamed in place: which, and its name field.
pub(super) struct Renaming {
    pub(super) group: u32,
    pub(super) input: Entity<InputState>,
    _events: Subscription,
}

/// The colours a group can wear, as the theme in force draws them.
pub(in crate::workspace) fn group_colors(cx: &App) -> Vec<(GroupColor, Hsla)> {
    let tokens = &cx.global::<SettingsStore>().tokens;
    GroupColor::ALL.into_iter().map(|color| (color, theme_bridge::hsla(color.color(tokens)))).collect()
}

impl Workspace {
    /// Fold an open group, or open a folded one.
    pub(in crate::workspace) fn toggle_group_band(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.view.toggle_group_fold(group);
        self.mark_dirty(cx);
    }

    /// The group the selected terminal is in: where a new terminal goes.
    pub(in crate::workspace) fn active_group(&self) -> Option<u32> {
        self.sessions.get(self.active).and_then(|session| self.sidebar.view.groups.group_of(session.id))
    }

    /// Terminal `id` and the rest of its split: they move, join and leave groups together.
    fn with_split_mates(&self, id: i64) -> Vec<i64> {
        let Some(ix) = self.find_session(id) else { return Vec::new() };
        match self.sessions[ix].split {
            Some(split) => self.members(split).into_iter().map(|member| self.sessions[member].id).collect(),
            None => vec![id],
        }
    }

    /// A new terminal in group `group` (its head's +), where the selected terminal is.
    pub(in crate::workspace) fn new_session_in_group(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.default_cwd(cx);
        let ix = self.open_session_in(Some(Some(group)), None, cwd, window, cx);
        let _ = ix;
        self.mark_dirty(cx);
    }

    /// Put terminal `id` (with the rest of its split) in group `group`, last in it.
    pub(in crate::workspace) fn move_to_group(&mut self, id: i64, group: u32, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        for member in self.with_split_mates(id) {
            self.sidebar.view.place_in_group(&ids, member, group, None);
        }
        self.list_rearranged(cx);
    }

    /// Take terminal `id` (with the rest of its split) out of its group: it stands right after it.
    pub(in crate::workspace) fn remove_from_group(&mut self, id: i64, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        let Some(group) = self.sidebar.view.groups.group_of(id) else { return };
        let top = self.sidebar.view.arranged(&ids);
        let after =
            top.iter().position(|&item| item == Item::Group(group)).and_then(|at| top.get(at + 1)).copied();
        for member in self.with_split_mates(id) {
            self.sidebar.view.place_at_top(&ids, Item::Terminal(member), after);
        }
        self.list_rearranged(cx);
    }

    /// A new group holding `terminals` (and the rest of their splits). Its id.
    pub(in crate::workspace) fn group_terminals(&mut self, terminals: &[i64], cx: &mut Context<Self>) -> u32 {
        let ids = self.session_ids();
        let mut members: Vec<i64> = Vec::new();
        for &id in terminals {
            for member in self.with_split_mates(id) {
                if !members.contains(&member) {
                    members.push(member);
                }
            }
        }
        let group = self.sidebar.view.new_group(&ids, &members);
        self.list_rearranged(cx);
        group
    }

    /// A new group holding terminal `id` (and its split), renamed at once.
    pub(in crate::workspace) fn group_terminal(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let group = self.group_terminals(&[id], cx);
        self.start_group_rename(group, window, cx);
    }

    /// Take group `group` apart; its terminals stand where it stood.
    pub(in crate::workspace) fn ungroup(&mut self, group: u32, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        self.sidebar.view.ungroup(&ids, group);
        self.list_rearranged(cx);
    }

    pub(in crate::workspace) fn set_group_color(
        &mut self,
        group: u32,
        color: Option<GroupColor>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.view.groups.set_color(group, color);
        self.mark_dirty(cx);
    }

    /// Put what a drop landed where it landed: a terminal (with its split, unless it leaves it) or a
    /// group.
    pub(in crate::workspace) fn land_terminal(
        &mut self,
        id: i64,
        landing: Landing,
        with_mates: bool,
        cx: &mut Context<Self>,
    ) {
        let ids = self.session_ids();
        let members = if with_mates { self.with_split_mates(id) } else { vec![id] };
        for member in members {
            self.sidebar.view.land(&ids, member, landing);
        }
        self.list_rearranged(cx);
    }

    pub(in crate::workspace) fn land_group(&mut self, group: u32, landing: Landing, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        self.sidebar.view.land_group(&ids, group, landing);
        self.list_rearranged(cx);
    }

    /// The group's name becomes a field, its text selected: ↩ or a click elsewhere keeps what was
    /// typed, ⎋ the old name.
    pub(in crate::workspace) fn start_group_rename(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(name) = self.sidebar.view.groups.get(group).map(|g| g.name.clone()) else { return };
        let input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let events =
            cx.subscribe_in(&input, window, move |this, _, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { .. } | InputEvent::Blur => {
                    this.finish_group_rename(true, window, cx)
                }
                _ => {}
            });
        input.update(cx, |input, cx| {
            input.focus(window, cx);
            input.select_all(window, cx);
        });
        self.sidebar.renaming = Some(Renaming { group, input, _events: events });
        cx.notify();
    }

    /// Keep the typed name (`keep`) or the old one, and give the keyboard back to the terminal.
    pub(in crate::workspace) fn finish_group_rename(
        &mut self,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(renaming) = self.sidebar.renaming.take() else { return };
        if keep {
            let name = renaming.input.read(cx).value().to_string();
            self.sidebar.view.groups.rename(renaming.group, &name);
        }
        self.mark_dirty(cx);
        window.focus(&self.active_terminal().focus_handle(cx), cx);
    }

    /// A group's head: its name on a pill of its colour (or a field while it is renamed), the count
    /// while folded, the + for a new terminal in it. A terminal dragged onto it joins it.
    pub(super) fn render_group_band(
        &self,
        listing: &Listing,
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
        let tint = color.map(|color| BandTint { color: theme_bridge::hsla(color.color(tokens)), peak: 0.2 });
        let folded = band.folded;
        let waiting = band
            .members
            .iter()
            .filter(|&&ix| {
                self.sessions.get(ix).is_some_and(|s| s.terminal().read(cx).status() == Status::Attention)
            })
            .count();
        let this = cx.weak_entity();
        let (on_toggle, on_add) = (this.clone(), this.clone());
        let renaming = self.sidebar.renaming.as_ref().filter(|r| r.group == group).map(|r| r.input.clone());
        let renaming_now = renaming.is_some();
        let parts = BandParts {
            kind: BandKind::Folder,
            section: if folded { Section::Collapsed } else { Section::Open },
            title: if renaming_now { SharedString::default() } else { band.label.clone().into() },
            detail: renaming.map(|input| {
                // The field takes the name's place; ⎋ gives the old name back.
                div()
                    .flex_1()
                    .min_w_0()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            cx.stop_propagation();
                            this.finish_group_rename(false, window, cx);
                        }
                    }))
                    .child(Input::new(&input).xsmall())
                    .into_any_element()
            }),
            count: Some(style::count_label(band.members.len()).into()),
            waiting: (folded && waiting > 0).then(|| {
                let id = ElementId::Name(SharedString::from(format!("band-waiting-{}", band.key())));
                style::status_glyph(id, ui, style::StatusGlyph::NeedsYou, cx)
            }),
            add: Some((
                "New terminal in this group",
                Box::new(move |_, window, cx| {
                    on_add.update(cx, |this, cx| this.new_session_in_group(group, window, cx)).ok();
                }),
            )),
            on_toggle: Some(Box::new(move |_, _, cx| {
                on_toggle.update(cx, |this, cx| this.toggle_group_band(group, cx)).ok();
            })),
            drop_target: false,
            sticky: false,
            joined: false,
        };
        let look = FolderLook { cmd_held: frame.hints, tint: tint.filter(|_| !renaming_now), card: true };
        let id: ElementId = ElementId::Name(SharedString::from(format!("band-{}", band.key())));
        let mut label = spoken::band_label(
            &band.label,
            band.pinned,
            color.map(GroupColor::label),
            band.members.len(),
            waiting,
            !folded,
            position,
            of,
        );
        if folded && waiting > 0 {
            label = format!("{label}, {}", wormhole_core::spoken::terminals_need_you(waiting));
        }
        let (keyed, ring) = self.sidebar_keyed(&SidebarStop::Group(group), window);
        let menu_open = self.sidebar.menu_group == Some(group);
        let element = style::folder_band(id, ui, parts, look)
            .relative()
            .role(Role::TreeItem)
            .aria_label(label)
            .aria_expanded(!folded)
            .aria_level(1)
            .aria_position_in_set(position)
            .aria_size_of_set(of)
            .when(keyed, |d| d.aria_active_descendant())
            .when(ring || menu_open, |d| {
                d.child(style::focus_ring(ui, style::RingPlacement::Inside, px(style::CARD_ROW_RADIUS)))
            })
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                // A double click on the name renames the group.
                if event.click_count() == 2 {
                    this.start_group_rename(group, window, cx);
                }
            }));
        let element = self.with_group_drag(
            element,
            group,
            band.label.clone().into(),
            color.map(|color| theme_bridge::hsla(color.color(tokens))),
            cx,
        );
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

    /// What builds group `group`'s menu: New terminal in group, Rename…, Colour ▸, Ungroup.
    fn group_menu(&self, group: u32, cx: &mut Context<Self>) -> MenuBuilder {
        let this = cx.weak_entity();
        Rc::new(move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            let popup = cx.entity();
            let current = this
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
                    ws.sidebar.view.groups.get(group).and_then(|g| g.color())
                })
                .ok()
                .flatten();
            let choices = group_colors(cx);
            let ui = ui(cx);
            let (add, rename, colors, ungroup) = (this.clone(), this.clone(), this.clone(), this.clone());
            menu.item(PopupMenuItem::new("New terminal in group").on_click(move |_, window, cx| {
                add.update(cx, |ws, cx| ws.new_session_in_group(group, window, cx)).ok();
            }))
            .item(PopupMenuItem::new("Rename…").on_click(move |_, window, cx| {
                rename.update(cx, |ws, cx| ws.start_group_rename(group, window, cx)).ok();
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
            .item(PopupMenuItem::new("Ungroup").on_click(move |_, _, cx| {
                ungroup.update(cx, |ws, cx| ws.ungroup(group, cx)).ok();
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
