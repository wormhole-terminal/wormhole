//! A row's context menu: Show beside… (which terminal to show next to this one,
//! making a split), Add to group ▸ (the person's groups, New group…), Remove from group, Remove
//! from split, Close, Pin to top / Unpin, Reveal in Finder, Copy path.
//! The row it was opened on wears the focus ring until it closes. ⌃↩ opens it for the row the keys
//! are on, and a group's own menu (`groups`) for the head they are on.
use std::rc::Rc;

use wormhole_core::panes::{TabPanes, Zone};

use gpui_kit::component::menu::{ContextMenu, ContextMenuExt as _, PopupMenu, PopupMenuItem};

use super::super::focus::{MenuBuilder, SidebarStop};
use super::*;

impl Workspace {
    /// `row` with its context menu: the last thing put on a row, since the menu wraps it.
    pub(super) fn with_row_menu(
        &self,
        row: Stateful<Div>,
        info: &RowInfo,
        cx: &mut Context<Self>,
    ) -> ContextMenu<Stateful<Div>> {
        let build = self.row_menu(info, cx);
        row.context_menu(move |menu, window, cx| build(menu, window, cx))
    }

    /// ⌃↩: the menu of the terminal or the band the keys are on, under it. (The "No folder" band
    /// has none.)
    pub(in crate::workspace) fn sidebar_key_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.areas.sidebar_at.clone() {
            Some(SidebarStop::Row(id)) => self.row_key_menu(id, window, cx),
            Some(SidebarStop::Group(group)) => self.group_key_menu(group, window, cx),
            None => {}
        }
    }

    /// ⌃↩ on the terminal `id`: its row menu, under its row.
    fn row_key_menu(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        let infos = self.row_infos(cx);
        let listing = self.listing(&infos, cx);
        // The list's children are its entries, in order.
        let at = listing
            .entries
            .iter()
            .position(|e| matches!(e, Entry::Row { session, .. } if infos[*session].id == id));
        let (Some(row), Some(info)) =
            (at.and_then(|at| self.sidebar.scroll.bounds_for_item(at)), infos.iter().find(|i| i.id == id))
        else {
            return;
        };
        let build = self.row_menu(info, cx);
        self.open_key_menu(wormhole_core::focus::Area::Sidebar, row, build, window, cx);
    }

    /// What builds `info`'s row menu.
    fn row_menu(&self, info: &RowInfo, cx: &mut Context<Self>) -> MenuBuilder {
        let (id, pinned) = (info.id, info.pinned);
        let in_split = self.sessions.get(info.ix).is_some_and(|session| session.split.is_some());
        let cwd = info.cwd.clone();
        let this = cx.weak_entity();
        Rc::new(move |menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>| {
            let popup = cx.entity();
            this.update(cx, |ws, cx| {
                ws.sidebar.menu_row = Some(id);
                ws.sidebar.menu_group = None;
                ws.sidebar.menu_dismissed = Some(cx.subscribe(&popup, move |ws, _, _: &DismissEvent, cx| {
                    if ws.sidebar.menu_row == Some(id) {
                        ws.sidebar.menu_row = None;
                        cx.notify();
                    }
                }));
                cx.notify();
            })
            .ok();
            let on = |f: fn(&mut Workspace, usize, &mut Window, &mut Context<Workspace>)| {
                let this = this.clone();
                move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    this.update(cx, |ws, cx| {
                        if let Some(ix) = ws.sessions.iter().position(|s| s.id == id) {
                            f(ws, ix, window, cx);
                        }
                    })
                    .ok();
                }
            };
            let cwd = cwd.clone();
            // The terminals it could show beside this one, as they are when the menu opens.
            let choices = this.read_with(cx, |ws, cx| ws.beside_choices(id, cx)).unwrap_or_default();
            let menu = if choices.is_empty() {
                menu.item(PopupMenuItem::new("Show beside…").disabled(true))
            } else {
                let this = this.clone();
                menu.submenu("Show beside…", window, cx, move |submenu, _, _| {
                    choices.iter().fold(submenu, |submenu, (other, name)| {
                        let (this, other) = (this.clone(), *other);
                        submenu.item(PopupMenuItem::new(name.clone()).on_click(move |_, window, cx| {
                            this.update(cx, |ws, cx| ws.show_beside(other, id, window, cx)).ok();
                        }))
                    })
                })
            };
            // The person's groups it can go in, as they are when the menu opens.
            let groups = this.read_with(cx, |ws, _| ws.group_choices(id)).unwrap_or_default();
            let grouped = groups.iter().any(|(_, _, current)| *current);
            let into = this.clone();
            let menu = menu.submenu("Add to group", window, cx, move |submenu, _, _| {
                let submenu = groups.iter().fold(submenu, |submenu, (group, name, current)| {
                    let (this, group) = (into.clone(), *group);
                    submenu.item(PopupMenuItem::new(name.clone()).checked(*current).on_click(
                        move |_, _, cx| {
                            this.update(cx, |ws, cx| ws.move_to_group(id, group, cx)).ok();
                        },
                    ))
                });
                let this = into.clone();
                let submenu = if groups.is_empty() { submenu } else { submenu.separator() };
                submenu.item(PopupMenuItem::new("New group…").on_click(move |_, window, cx| {
                    this.update(cx, |ws, cx| ws.group_terminal(id, window, cx)).ok();
                }))
            });
            let out = this.clone();
            menu.when(grouped, |menu| {
                menu.item(PopupMenuItem::new("Remove from group").on_click(move |_, _, cx| {
                    out.update(cx, |ws, cx| ws.remove_from_group(id, cx)).ok();
                }))
            })
            .when(in_split, |menu| {
                menu.item(PopupMenuItem::new("Remove from split").on_click(on(|ws, ix, _, cx| {
                    ws.leave_split(ix, cx);
                })))
            })
            .separator()
            .item(
                PopupMenuItem::new("Close")
                    .on_click(on(|ws, ix, window, cx| ws.request_close_session(ix, window, cx))),
            )
            .separator()
            .item(
                PopupMenuItem::new(if pinned { "Unpin" } else { "Pin to top" })
                    .on_click(on(|ws, ix, _, cx| ws.toggle_pin(ix, cx))),
            )
            .separator()
            .item(PopupMenuItem::new("Reveal in Finder").disabled(cwd.is_none()).on_click({
                let cwd = cwd.clone();
                move |_, _, _| {
                    if let Some(cwd) = &cwd {
                        reveal_in_finder(cwd);
                    }
                }
            }))
            .item(PopupMenuItem::new("Copy path").disabled(cwd.is_none()).on_click(
                move |_, _, cx| {
                    if let Some(cwd) = &cwd {
                        cx.write_to_clipboard(ClipboardItem::new_string(cwd.to_string_lossy().into_owned()));
                    }
                },
            ))
        })
    }

    /// The terminals "Show beside…" offers for the terminal `id`, in the list's order: every other
    /// one that isn't in its split already. None when its pane has no room for another beside it.
    pub(in crate::workspace) fn beside_choices(&self, id: i64, cx: &App) -> Vec<(i64, SharedString)> {
        let Some(ix) = self.find_session(id) else { return Vec::new() };
        let layout = match self.split_of(ix) {
            Some(split) => split.layout.clone(),
            None => TabPanes::single(id),
        };
        let room = self.pane_room();
        let split = self.sessions[ix].split;
        self.session_order(cx)
            .into_iter()
            .filter(|&other| other != ix && (split.is_none() || self.sessions[other].split != split))
            .filter(|&other| layout.can_join(id, Zone::Right, self.sessions[other].id, room).is_ok())
            .map(|other| (self.sessions[other].id, self.sessions[other].name(cx)))
            .collect()
    }

    /// Terminal `other` shows beside terminal `id`, to the right of its pane: the two are a split
    /// now (the same as dropping one row on the other). A refusal for want of room changes nothing.
    pub(in crate::workspace) fn show_beside(
        &mut self,
        other: i64,
        id: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.find_session(other) else { return };
        if let Err(error) = self.join_split(ix, id, Zone::Right, window, cx) {
            tracing::debug!(?error, "Show beside was refused");
        }
    }

    /// Pin a terminal to the top of its group, or let it go back to its place (saved). A split is
    /// pinned or not as a whole: its members all take the new state.
    pub(crate) fn toggle_pin(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(session) = self.sessions.get(ix) else { return };
        let pinned = !self.split_pinned(ix);
        let members = match session.split {
            Some(split) => self.members(split),
            None => vec![ix],
        };
        for member in members {
            self.sessions[member].pinned = pinned;
        }
        self.mark_dirty(cx);
    }
}

/// Show a terminal's folder in the Finder, selected in its parent (also Terminal → Reveal in
/// Finder).
pub(in crate::workspace) fn reveal_in_finder(path: &Path) {
    let mut open = std::process::Command::new("open");
    open.arg("-R").arg(path);
    if let Err(err) = crate::workspace::spawn::spawn_detached(open) {
        tracing::warn!("revealing {}: {err}", path.display());
    }
}
