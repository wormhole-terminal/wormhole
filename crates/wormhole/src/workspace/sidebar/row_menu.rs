//! A row's context menu, in a browser's words: Rename…, Add terminal to new group, Add terminal to
//! group ▸ (the person's groups), Remove from group, Split right, Close terminal; then Show beside…
//! (which terminal to show next to this one, making a split), Remove from split, Pin to top / Unpin,
//! Don't keep this terminal's text (ticked when chosen; greyed out while no terminal's text is kept);
//! Reveal in Finder, Copy path.
//! The row it was opened on wears the focus ring until it closes. ⌃↩ opens it for the row the keys
//! are on, and a group's own menu (`groups`) for the head they are on.
use std::path::PathBuf;

use wormhole_core::panes::{TabPanes, Zone};

use super::super::focus::SidebarStop;
use super::*;
use crate::platform::{self, Item, Menu};

/// What a row's menu is made from, as it is when the menu opens.
struct RowFacts {
    id: i64,
    pinned: bool,
    /// Its text isn't kept for a restart.
    forget_text: bool,
    /// Restore terminal text after restart is on.
    restoring: bool,
    in_split: bool,
    cwd: Option<PathBuf>,
    /// The person's groups it can go in: id, name, whether the terminal is in it.
    groups: Vec<(u32, SharedString, bool)>,
    /// The terminals it could show beside this one.
    beside: Vec<(i64, SharedString)>,
}

impl Workspace {
    /// `row` with its context menu under a right-click: the last thing put on a row.
    pub(super) fn with_row_menu(
        &self,
        row: Stateful<Div>,
        info: &RowInfo,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = info.id;
        row.on_mouse_down(
            MouseButton::Right,
            cx.listener(move |ws, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                ws.open_row_menu(id, event.position, window, cx);
            }),
        )
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
        let Some(row) = at.and_then(|at| self.sidebar.scroll.bounds_for_item(at)) else { return };
        self.open_row_menu(id, super::super::focus::key_menu_origin(row), window, cx);
    }

    /// Open terminal `id`'s menu at `at`, its row wearing the ring until the menu closes. Nothing
    /// opens for a terminal that is gone.
    fn open_row_menu(&mut self, id: i64, at: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.row_menu(id, cx) else { return };
        self.sidebar.menu_row = Some(id);
        self.sidebar.menu_group = None;
        cx.notify();
        platform::show_menu(menu, at, window, cx);
    }

    /// Terminal `id`'s row menu, with its groups and neighbours as they are now.
    fn row_menu(&self, id: i64, cx: &mut Context<Self>) -> Option<Menu> {
        let ix = self.find_session(id)?;
        let facts = RowFacts {
            id,
            pinned: self.split_pinned(ix),
            forget_text: self.sessions[ix].forget_text,
            restoring: settings_in(&self.profile, cx).restore_terminal_text,
            in_split: self.sessions[ix].split.is_some(),
            cwd: self.sessions[ix].terminal().read(cx).cwd.clone(),
            groups: self.group_choices(id),
            beside: self.beside_choices(id, cx),
        };
        let this = cx.weak_entity();
        let ring = this.clone();
        Some(row_menu(facts, &this).on_close(move |_, cx| {
            ring.update(cx, |ws, cx| {
                if ws.sidebar.menu_row == Some(id) {
                    ws.sidebar.menu_row = None;
                    cx.notify();
                }
            })
            .ok();
        }))
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
    if let Err(err) = crate::platform::spawn::spawn_detached(open) {
        tracing::warn!("revealing {}: {err}", path.display());
    }
}

/// The menu of the row `facts` describe; every item acts on `this`.
fn row_menu(facts: RowFacts, this: &WeakEntity<Workspace>) -> Menu {
    let RowFacts { id, pinned, forget_text, restoring, in_split, cwd, groups, beside } = facts;
    // An item that finds the terminal again by its id: the list may have changed while the menu
    // was open.
    let on = |run: fn(&mut Workspace, usize, &mut Window, &mut Context<Workspace>)| {
        platform::run_on(this, move |ws, window, cx| {
            if let Some(ix) = ws.find_session(id) {
                run(ws, ix, window, cx);
            }
        })
    };
    let grouped = groups.iter().any(|(_, _, current)| *current);
    let into = Menu::new().when(!groups.is_empty(), |menu| {
        groups.iter().fold(menu, |menu, (group, name, current)| {
            let group = *group;
            menu.item(
                Item::new(name.clone())
                    .checked(*current)
                    .on_select(platform::run_on(this, move |ws, _, cx| ws.move_to_group(id, group, cx))),
            )
        })
    });
    let next_to =
        beside.iter().fold(Menu::new(), |menu, (other, name)| {
            let other = *other;
            menu.item(Item::new(name.clone()).on_select(platform::run_on(this, move |ws, window, cx| {
                ws.show_beside(other, id, window, cx)
            })))
        });
    Menu::new()
        .item(Item::new("Rename…").on_select(platform::run_on(this, move |ws, window, cx| {
            ws.start_terminal_rename(id, window, cx)
        })))
        .item(
            Item::new("Add terminal to new group")
                .on_select(platform::run_on(this, move |ws, window, cx| ws.group_terminal(id, window, cx))),
        )
        .when(groups.is_empty(), |menu| menu.disabled_submenu("Add terminal to group"))
        .when(!groups.is_empty(), |menu| menu.submenu("Add terminal to group", into))
        .when(grouped, |menu| {
            menu.item(
                Item::new("Remove from group")
                    .on_select(platform::run_on(this, move |ws, _, cx| ws.remove_from_group(id, cx))),
            )
        })
        .item(Item::new("Split right").on_select(on(|ws, ix, window, cx| {
            ws.activate(ix, window, cx);
            ws.split_pane(wormhole_core::panes::Axis::Row, window, cx);
        })))
        .item(
            Item::new("Close terminal")
                .on_select(on(|ws, ix, window, cx| ws.request_close_session(ix, window, cx))),
        )
        .separator()
        .when(beside.is_empty(), |menu| menu.disabled_submenu("Show beside…"))
        .when(!beside.is_empty(), |menu| menu.submenu("Show beside…", next_to))
        .when(in_split, |menu| {
            menu.item(Item::new("Remove from split").on_select(on(|ws, ix, _, cx| {
                ws.leave_split(ix, cx);
            })))
        })
        .item(
            Item::new(if pinned { "Unpin" } else { "Pin to top" })
                .on_select(on(|ws, ix, _, cx| ws.toggle_pin(ix, cx))),
        )
        .item(
            Item::new("Don't keep this terminal's text")
                .checked(forget_text)
                .enabled(restoring)
                .on_select(on(|ws, ix, _, cx| ws.toggle_forget_text(ix, cx))),
        )
        .separator()
        .item(Item::new("Reveal in Finder").enabled(cwd.is_some()).on_select({
            let cwd = cwd.clone();
            move |_, _| {
                if let Some(cwd) = &cwd {
                    reveal_in_finder(cwd);
                }
            }
        }))
        .item(Item::new("Copy path").enabled(cwd.is_some()).on_select(move |_, cx| {
            if let Some(cwd) = &cwd {
                cx.write_to_clipboard(ClipboardItem::new_string(cwd.to_string_lossy().into_owned()));
            }
        }))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use gpui_kit::WeakEntity;

    use super::{RowFacts, row_menu};

    fn facts() -> RowFacts {
        RowFacts {
            id: 7,
            pinned: false,
            forget_text: false,
            restoring: true,
            in_split: false,
            cwd: Some(PathBuf::from("/tmp")),
            groups: Vec::new(),
            beside: Vec::new(),
        }
    }

    fn outline(facts: RowFacts) -> Vec<String> {
        row_menu(facts, &WeakEntity::new_invalid()).outline()
    }

    /// A lone terminal in no group: nothing to move to or to show beside, so those two are
    /// greyed out, as before.
    #[test]
    fn a_lone_terminal_offers_no_group_and_no_neighbour() {
        assert_eq!(
            outline(facts()),
            [
                "- Rename…",
                "- Add terminal to new group",
                "> Add terminal to group [off]",
                "- Split right",
                "- Close terminal",
                "---",
                "> Show beside… [off]",
                "- Pin to top",
                "- Don't keep this terminal's text",
                "---",
                "- Reveal in Finder",
                "- Copy path",
            ]
        );
    }

    /// With Restore terminal text after restart off nothing is kept anyway: the choice is greyed out.
    #[test]
    fn the_text_choice_is_off_while_no_text_is_kept() {
        let outline = outline(RowFacts { restoring: false, ..facts() });
        assert!(outline.contains(&"- Don't keep this terminal's text [off]".to_string()), "{outline:?}");
    }

    #[test]
    fn groups_neighbours_and_state_show_up() {
        let outline = outline(RowFacts {
            pinned: true,
            forget_text: true,
            in_split: true,
            cwd: None,
            groups: vec![(1, "Work".into(), true), (2, "Play".into(), false)],
            beside: vec![(9, "api".into())],
            ..facts()
        });
        assert_eq!(
            outline,
            [
                "- Rename…",
                "- Add terminal to new group",
                "> Add terminal to group",
                "  - Work [x]",
                "  - Play",
                "- Remove from group",
                "- Split right",
                "- Close terminal",
                "---",
                "> Show beside…",
                "  - api",
                "- Remove from split",
                "- Unpin",
                "- Don't keep this terminal's text [x]",
                "---",
                "- Reveal in Finder [off]",
                "- Copy path [off]",
            ]
        );
    }
}
