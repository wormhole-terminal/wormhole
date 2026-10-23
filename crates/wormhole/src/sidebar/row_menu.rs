//! A row's context menu, in a browser's words: Rename…, Add terminal to new group, Add terminal to
//! group ▸ (the person's groups), Remove from group, Split right, Close terminal; then Show beside…
//! (which terminal to show next to this one, making a split), Remove from split, Pin to top / Unpin,
//! Don't keep this terminal's text (ticked when chosen; greyed out while no terminal's text is kept);
//! Reveal in Finder, Copy path.
//! The row it was opened on wears the focus ring until it closes. ⌃↩ opens it for the row the keys
//! are on, and a group's own menu (`groups`) for the head they are on. The terminals "Show beside…"
//! offers depend on the panes' room, which only the window knows: the list asks it
//! ([`SidebarEvent::RowMenu`]) and shows the menu with the answer ([`Sidebar::show_row_menu`]).
use std::path::Path;
use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::view::sidebar::Entry;

use super::{RowInfo, Sidebar, SidebarEvent};
use crate::focus_keys::key_menu_origin;
use crate::platform::{self, Item, Menu};
use crate::settings::settings_in;
use crate::sidebar::SidebarStop;

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

impl Sidebar {
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
            cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                cx.emit(SidebarEvent::RowMenu { id, at: event.position });
            }),
        )
    }

    /// ⌃↩: the menu of the terminal or the band the keys are on, under it. (The "No folder" band
    /// has none.)
    pub(super) fn open_key_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.keys_at.clone() {
            Some(SidebarStop::Row(id)) => self.row_key_menu(id, cx),
            Some(SidebarStop::Group(group)) => self.group_key_menu(group, window, cx),
            None => {}
        }
    }

    /// ⌃↩ on the terminal `id`: its row menu, under its row.
    fn row_key_menu(&mut self, id: i64, cx: &mut Context<Self>) {
        let infos = &self.input.rows;
        let listing = self.listing(cx);
        // The list's children are its entries, in order.
        let at = listing
            .entries
            .iter()
            .position(|e| matches!(e, Entry::Row { session, .. } if infos[*session].id == id));
        let Some(row) = at.and_then(|at| self.scroll.bounds_for_item(at)) else { return };
        cx.emit(SidebarEvent::RowMenu { id, at: key_menu_origin(row) });
    }

    /// Open terminal `id`'s menu at `at`, its row wearing the ring until the menu closes, with the
    /// terminals the window says could show beside it. Nothing opens for a terminal that is gone.
    pub(crate) fn show_row_menu(
        &mut self,
        id: i64,
        at: Point<Pixels>,
        beside: Vec<(i64, SharedString)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(menu) = self.row_menu(id, beside, cx) else { return };
        self.menu_row = Some(id);
        self.menu_group = None;
        cx.notify();
        platform::show_menu(menu, at, window, cx);
    }

    /// Terminal `id`'s row menu, with its groups as they are now.
    fn row_menu(&self, id: i64, beside: Vec<(i64, SharedString)>, cx: &mut Context<Self>) -> Option<Menu> {
        let row = &self.input.rows[self.input.find(id)?];
        let facts = RowFacts {
            id,
            pinned: row.pinned,
            forget_text: row.forget_text,
            restoring: settings_in(&self.input.profile, cx).restore_terminal_text,
            in_split: row.split.is_some(),
            cwd: row.cwd.clone(),
            groups: self.group_choices(id),
            beside,
        };
        let this = cx.weak_entity();
        let ring = this.clone();
        Some(row_menu(facts, &this).on_close(move |_, cx| {
            ring.update(cx, |sidebar, cx| {
                if sidebar.menu_row == Some(id) {
                    sidebar.menu_row = None;
                    cx.notify();
                }
            })
            .ok();
        }))
    }
}

/// Show a terminal's folder in the Finder, selected in its parent (also Terminal → Reveal in
/// Finder).
// GPUI's test platform panics on `reveal_path` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
pub(crate) fn reveal_in_finder(path: &Path, cx: &App) {
    cx.reveal_path(path);
}

/// The menu of the row `facts` describe; every item acts on `this`.
fn row_menu(facts: RowFacts, this: &WeakEntity<Sidebar>) -> Menu {
    let RowFacts { id, pinned, forget_text, restoring, in_split, cwd, groups, beside } = facts;
    // An item that tells the window, which finds the terminal again by its id: the list may have
    // changed while the menu was open.
    let tell = |event: fn(i64) -> SidebarEvent| platform::run_on(this, move |_, _, cx| cx.emit(event(id)));
    let grouped = groups.iter().any(|(_, _, current)| *current);
    let into = Menu::new().when(!groups.is_empty(), |menu| {
        groups.iter().fold(menu, |menu, (group, name, current)| {
            let group = *group;
            menu.item(
                Item::new(name.clone())
                    .checked(*current)
                    .on_select(platform::run_on(this, move |sidebar, _, cx| {
                        sidebar.move_to_group(id, group, cx)
                    })),
            )
        })
    });
    let next_to = beside.iter().fold(Menu::new(), |menu, (other, name)| {
        let other = *other;
        menu.item(Item::new(name.clone()).on_select(platform::run_on(this, move |_, _, cx| {
            cx.emit(SidebarEvent::ShowBeside { other, id })
        })))
    });
    Menu::new()
        .item(Item::new("Rename…").on_select(platform::run_on(this, move |sidebar, window, cx| {
            sidebar.start_terminal_rename(id, window, cx)
        })))
        .item(
            Item::new("Add terminal to new group")
                .on_select(platform::run_on(this, move |sidebar, window, cx| {
                    sidebar.group_terminal(id, window, cx)
                })),
        )
        .when(groups.is_empty(), |menu| menu.disabled_submenu("Add terminal to group"))
        .when(!groups.is_empty(), |menu| menu.submenu("Add terminal to group", into))
        .when(grouped, |menu| {
            menu.item(
                Item::new("Remove from group").on_select(platform::run_on(this, move |sidebar, _, cx| {
                    sidebar.remove_from_group(id, cx)
                })),
            )
        })
        .item(Item::new("Split right").on_select(tell(SidebarEvent::SplitRight)))
        .item(Item::new("Close terminal").on_select(tell(SidebarEvent::Close)))
        .separator()
        .when(beside.is_empty(), |menu| menu.disabled_submenu("Show beside…"))
        .when(!beside.is_empty(), |menu| menu.submenu("Show beside…", next_to))
        .when(in_split, |menu| {
            menu.item(Item::new("Remove from split").on_select(tell(SidebarEvent::LeaveSplit)))
        })
        .item(Item::new(if pinned { "Unpin" } else { "Pin to top" }).on_select(tell(SidebarEvent::TogglePin)))
        .item(
            Item::new("Don't keep this terminal's text")
                .checked(forget_text)
                .enabled(restoring)
                .on_select(tell(SidebarEvent::ToggleForgetText)),
        )
        .separator()
        .item(Item::new("Reveal in Finder").enabled(cwd.is_some()).on_select({
            let cwd = cwd.clone();
            move |_, cx| {
                if let Some(cwd) = &cwd {
                    reveal_in_finder(cwd, cx);
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
