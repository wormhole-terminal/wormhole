//! The person's own groups in the list (grouping "Groups"): a group's band (its name on a pill of
//! its colour, the count while folded, the + for a new terminal in it), its menu (New terminal in
//! group, Rename…, Colour ▸, Ungroup), renaming it in place, and putting terminals in and out of
//! groups (a row's menu, a drop on a group's band or beside one of its rows). Core keeps the groups
//! (`sidebar::Groups`, saved with the window); this draws them and changes them.
use std::rc::Rc;

use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{ContextMenu, ContextMenuExt as _, PopupMenu, PopupMenuItem};
use wormhole_core::folder_color::FolderColor;
use wormhole_core::sidebar::spoken;

use super::super::focus::{MenuBuilder, SidebarStop};
use super::super::folder_color;
use super::super::terminal_drag::DraggedTerminal;
use super::rows::Frame;
use super::*;
use crate::settings::SettingsStore;
use crate::style::{BandKind, BandParts, FolderLook, Section};

/// A group being renamed in place: which, and its name field.
pub(super) struct Renaming {
    pub(super) group: u32,
    pub(super) input: Entity<InputState>,
    _events: Subscription,
}

impl Workspace {
    /// Fold an open group, or open a folded one.
    pub(super) fn toggle_group_band(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.view.toggle_group_fold(group);
        self.mark_dirty(cx);
    }

    /// The group the selected terminal is in, while the list shows the groups: where a new
    /// terminal goes.
    pub(in crate::workspace) fn active_group(&self) -> Option<u32> {
        if self.sidebar.view.grouping != Grouping::Custom {
            return None;
        }
        self.sessions.get(self.active).and_then(|session| self.sidebar.view.groups.group_of(session.id))
    }

    /// A new terminal in group `group` (its band's +), where the selected terminal is.
    pub(in crate::workspace) fn new_session_in_group(
        &mut self,
        group: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self.default_cwd(cx);
        let ix = self.open_session(None, cwd, window, cx);
        if let Some(id) = self.sessions.get(ix).map(|session| session.id) {
            self.sidebar.view.groups.join(group, id);
        }
        self.mark_dirty(cx);
    }

    /// Put terminal `id` (with the rest of its split) in group `group`, or in none.
    pub(in crate::workspace) fn move_to_group(
        &mut self,
        id: i64,
        group: Option<u32>,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(id) else { return };
        let members = match self.sessions[ix].split {
            Some(split) => self.members(split),
            None => vec![ix],
        };
        for member in members {
            let member = self.sessions[member].id;
            match group {
                Some(group) => {
                    self.sidebar.view.groups.join(group, member);
                }
                None => self.sidebar.view.groups.leave(member),
            }
        }
        self.mark_dirty(cx);
    }

    /// A new group holding terminal `id` (and its split), shown as groups, renamed at once.
    pub(in crate::workspace) fn group_terminal(
        &mut self,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let group = self.sidebar.view.groups.create(&[]);
        self.move_to_group(id, Some(group), cx);
        self.set_grouping(Grouping::Custom, cx);
        self.start_group_rename(group, window, cx);
    }

    /// A new, empty group at the end of the list, renamed at once.
    pub(in crate::workspace) fn new_group(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let group = self.sidebar.view.groups.create(&[]);
        self.set_grouping(Grouping::Custom, cx);
        self.mark_dirty(cx);
        self.start_group_rename(group, window, cx);
    }

    /// Remove group `group`; its terminals stay, loose.
    pub(in crate::workspace) fn ungroup(&mut self, group: u32, cx: &mut Context<Self>) {
        self.sidebar.view.groups.remove(group);
        self.sidebar.view.folded.remove(&wormhole_core::sidebar::group_key(group));
        self.mark_dirty(cx);
    }

    pub(in crate::workspace) fn set_group_color(
        &mut self,
        group: u32,
        color: Option<FolderColor>,
        cx: &mut Context<Self>,
    ) {
        self.sidebar.view.groups.set_color(group, color.map_or("", FolderColor::key));
        self.mark_dirty(cx);
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

    /// The band of one of the person's groups: its name on a pill of its colour (or a field while
    /// it is renamed), the count while folded, the + for a new terminal in it. A terminal dragged
    /// onto it joins it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_group_band(
        &self,
        listing: &Listing,
        band_ix: usize,
        (position, of): (usize, usize),
        frame: &Frame,
        sticky: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = frame.ui;
        let band = &listing.bands[band_ix];
        let Some(group) = band.group else { return div().into_any_element() };
        let color = self.sidebar.view.groups.get(group).and_then(|g| FolderColor::from_key(&g.color));
        let tokens = &cx.global::<SettingsStore>().tokens;
        let tint = color.map(|color| folder_color::band_tint_in(color, tokens));
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
        let parts = BandParts {
            kind: BandKind::Folder,
            section: if folded { Section::Collapsed } else { Section::Open },
            title: band.label.clone().into(),
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
            count: Some(style::count_label(band.matches).into()),
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
            sticky,
            joined: false,
        };
        let renaming_now = self.sidebar.renaming.as_ref().is_some_and(|r| r.group == group);
        let parts = if renaming_now { BandParts { title: SharedString::default(), ..parts } } else { parts };
        let look = FolderLook {
            cmd_held: frame.hints,
            unknown: false,
            tint: tint.filter(|_| !renaming_now),
            card: true,
        };
        let id: ElementId = if sticky {
            "sticky-band".into()
        } else {
            ElementId::Name(SharedString::from(format!("band-{}", band.key())))
        };
        let mut label = spoken::band_label(
            &band.label,
            false,
            color.map(FolderColor::label),
            None,
            band.members.len(),
            !folded,
            position,
            of,
        );
        if folded && waiting > 0 {
            label = format!("{label}, {}", wormhole_core::spoken::terminals_need_you(waiting));
        }
        let (keyed, ring) = self.sidebar_keyed(&SidebarStop::Group(group), window);
        let keyed = keyed && !sticky;
        let menu_open = self.sidebar.menu_group == Some(group);
        let highlight = ui.highlight;
        let element = style::folder_band(id, ui, parts, look, None)
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
            // A terminal dropped on the band joins the group, at its end.
            .drag_over::<DraggedTerminal>(move |style, _, _, _| style.bg(highlight))
            .on_drop(cx.listener(move |this, dragged: &DraggedTerminal, _, cx| {
                // The list underneath would take it out of its split instead.
                cx.stop_propagation();
                this.terminal_drag = None;
                this.move_to_group(dragged.id, Some(group), cx);
            }))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                // A double click on the name renames the group.
                if event.click_count() == 2 {
                    this.start_group_rename(group, window, cx);
                }
            }));
        let build = self.group_menu(group, cx);
        let menu: ContextMenu<Stateful<Div>> =
            element.context_menu(move |menu, window, cx| build(menu, window, cx));
        menu.into_any_element()
    }

    /// ⌃↩ with the keys on group `group`'s band: its menu, under it.
    pub(super) fn group_key_menu(&mut self, group: u32, window: &mut Window, cx: &mut Context<Self>) {
        let listing = self.session_listing(cx);
        // The list's children are its entries, in order.
        let at = listing.entries.iter().position(
            |entry| matches!(entry, Entry::Band { band, .. } if listing.bands[*band].group == Some(group)),
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
                    ws.sidebar.menu_band = None;
                    ws.sidebar.menu_group = Some(group);
                    ws.sidebar.menu_dismissed =
                        Some(cx.subscribe(&popup, move |ws, _, _: &DismissEvent, cx| {
                            if ws.sidebar.menu_group == Some(group) {
                                ws.sidebar.menu_group = None;
                                cx.notify();
                            }
                        }));
                    cx.notify();
                    ws.sidebar.view.groups.get(group).and_then(|g| FolderColor::from_key(&g.color))
                })
                .ok()
                .flatten();
            let choices = folder_color::choices(cx);
            let ui = ui(cx);
            let (add, rename, colors, ungroup) = (this.clone(), this.clone(), this.clone(), this.clone());
            menu.item(PopupMenuItem::new("New terminal in group").on_click(move |_, window, cx| {
                add.update(cx, |ws, cx| ws.new_session_in_group(group, window, cx)).ok();
            }))
            .item(PopupMenuItem::new("Rename group…").on_click(move |_, window, cx| {
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
                let this = colors.clone();
                submenu.separator().item(PopupMenuItem::new("None").checked(current.is_none()).on_click(
                    move |_, _, cx| {
                        this.update(cx, |ws, cx| ws.set_group_color(group, None, cx)).ok();
                    },
                ))
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
