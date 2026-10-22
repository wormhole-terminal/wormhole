//! The folder in the title row, right-clicked: a native menu like the one a Finder window's title opens.
//! The path as shown (`~/Projects/tube`) heads it; Open in Finder, Copy path and Insert into
//! terminal follow; and then the path's folders, the innermost first, down to the volume, each
//! with the icon Finder gives it (the house for the home folder, a disk for the volume). Choosing
//! a folder opens it in Finder.
//!
//! It opens on a right-click of the folder (and ⌃↩ on the title strip); resting on the folder opens
//! the hover card (`path_card`), and a click opens the folder in Finder.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::view::layout::path_menu::{self, PathFolder};

use crate::platform::{self, Icon, Item, Menu};
use crate::terminal_view::Status;
use crate::workspace::{Workspace, session};

impl Workspace {
    /// Open the menu of the active terminal's folder under the folder in the title row (a right-click
    /// on it, or ⌃↩ with the keys on the strip). Nothing for a terminal whose folder isn't known.
    pub(in crate::workspace) fn open_path_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = self.active_terminal().read(cx);
        let Some(folder) = terminal.cwd.clone() else { return };
        let exited = terminal.status() == Status::Exited;
        let shown = wormhole_core::view::sidebar::display_path(&folder, session::home_dir());
        let menu = path_menu(&folder, &shown, &platform::boot_volume_name(), exited, &cx.weak_entity());
        platform::show_menu(menu, self.path_anchor.below(), window, cx);
    }
}

/// The menu of `folder`, which reads `shown`; `volume` is the name of the volume the system
/// starts from, and `exited` says no terminal is running to type into.
fn path_menu(folder: &Path, shown: &str, volume: &str, exited: bool, this: &WeakEntity<Workspace>) -> Menu {
    let (copy_of, insert_of, open_of) = (folder.to_path_buf(), folder.to_path_buf(), folder.to_path_buf());
    let menu = Menu::new()
        .header(shown.to_string())
        .separator()
        .item(Item::new("Open in Finder").on_select(move |_, cx| cx.open_with_system(&open_of)))
        .item(Item::new("Copy path").on_select(move |_, cx| Workspace::copy_path(&copy_of, cx)))
        .item(
            Item::new("Insert into terminal")
                .enabled(!exited)
                .on_select(platform::run_on(this, move |ws, window, cx| {
                    ws.insert_path(&insert_of, window, cx)
                })),
        )
        .separator();
    path_menu::chain(folder, volume).into_iter().fold(menu, |menu, PathFolder { path, name }| {
        let icon = Icon::File(path.clone());
        menu.item(Item::new(name).icon(Some(icon)).on_select(move |_, cx| cx.open_with_system(&path)))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui_kit::WeakEntity;

    use super::path_menu;

    fn outline(folder: &str, shown: &str, exited: bool) -> Vec<String> {
        path_menu(Path::new(folder), shown, "Macintosh HD", exited, &WeakEntity::new_invalid()).outline()
    }

    #[test]
    fn a_folder_under_home_lists_its_parents_down_to_the_volume() {
        assert_eq!(
            outline("/Users/ben/Projects/tube", "~/Projects/tube", false),
            [
                "# ~/Projects/tube",
                "---",
                "- Open in Finder",
                "- Copy path",
                "- Insert into terminal",
                "---",
                "- tube",
                "- Projects",
                "- ben",
                "- Users",
                "- Macintosh HD",
            ]
        );
    }

    #[test]
    fn a_folder_outside_home_shows_its_whole_path_and_a_volume_root_is_the_volume() {
        let tmp = outline("/tmp", "/tmp", false);
        assert_eq!(tmp[0], "# /tmp");
        assert_eq!(&tmp[6..], ["- tmp", "- Macintosh HD"]);
        let root = outline("/", "/", false);
        assert_eq!(&root[6..], ["- Macintosh HD"]);
    }

    #[test]
    fn another_volume_is_where_the_list_ends() {
        let menu = outline("/Volumes/Work/acme", "/Volumes/Work/acme", false);
        assert_eq!(&menu[6..], ["- acme", "- Work"]);
    }

    #[test]
    fn an_exited_terminal_cannot_be_typed_into() {
        assert!(outline("/tmp", "/tmp", true).contains(&"- Insert into terminal [off]".to_string()));
    }
}
