//! The folder in the title row, clicked: a native menu like the one a Finder window's title opens.
//! The path as shown (`~/Projects/tube`) heads it; Open in Finder, Copy path and Insert into
//! terminal follow; and then the path's folders, the innermost first, down to the volume, each
//! with the icon Finder gives it (the house for the home folder, a disk for the volume). Choosing
//! a folder opens it in Finder.
//!
//! Insert into terminal types the path, quoted for the shell, at the focused terminal's prompt
//! as a paste (so an agent's input takes it like text you pasted).

use std::path::Path;

use wormhole_core::layout::path_menu::{self, PathFolder};
use wormhole_core::shells;

use super::*;
use crate::native::{self, Icon, Item, Menu};

impl Workspace {
    /// Open the menu of the active terminal's folder under the folder in the title row (a click on
    /// it, or ⌃↩ with the keys on the strip). Nothing for a terminal whose folder isn't known.
    pub(super) fn open_path_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = self.active_terminal().read(cx);
        let Some(folder) = terminal.cwd.clone() else { return };
        let exited = terminal.status() == Status::Exited;
        let shown = wormhole_core::sidebar::display_path(&folder, session::home_dir());
        let menu = path_menu(&folder, &shown, &native::boot_volume_name(), exited, &cx.weak_entity());
        native::show_menu(menu, self.path_anchor.below(), window, cx);
    }

    /// Insert into terminal: the folder's absolute path, in single quotes, typed at the focused
    /// terminal as a paste (bracketed when the program asked for it), without a Return. The
    /// terminal has the keyboard.
    fn insert_path(&mut self, folder: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let text = insert_text(folder);
        let terminal = self.active_terminal();
        terminal.update(cx, |terminal, _| terminal.paste(&text));
        self.focus_terminal(window, cx);
        cx.notify();
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
        .item(Item::new("Copy path").on_select(move |_, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(copy_of.to_string_lossy().into_owned()));
            crate::a11y::announce("Path copied", false);
        }))
        .item(
            Item::new("Insert into terminal")
                .enabled(!exited)
                .on_select(native::run_on(this, move |ws, window, cx| {
                    ws.insert_path(&insert_of, window, cx)
                })),
        )
        .separator();
    path_menu::chain(folder, volume).into_iter().fold(menu, |menu, PathFolder { path, name }| {
        let icon = Icon::File(path.clone());
        menu.item(Item::new(name).icon(Some(icon)).on_select(move |_, cx| cx.open_with_system(&path)))
    })
}

/// What Insert into terminal types: the path in single quotes, so spaces and `$` survive.
pub(super) fn insert_text(folder: &Path) -> String {
    shells::single_quote(&folder.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui_kit::WeakEntity;

    use super::{insert_text, path_menu};

    #[test]
    fn insert_types_the_absolute_path_in_single_quotes() {
        assert_eq!(insert_text(Path::new("/Users/ben/Projects/tube")), "'/Users/ben/Projects/tube'");
        assert_eq!(insert_text(Path::new("/Users/ben/My Projects/$HOME")), "'/Users/ben/My Projects/$HOME'");
        assert_eq!(insert_text(Path::new("/Users/ben/it's")), r#"'/Users/ben/it'\''s'"#);
    }

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
