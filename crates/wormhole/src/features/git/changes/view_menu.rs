//! The menu behind the Changes head's view button: how the list is shown. It writes the same settings
//! as Settings → Git, so the two never disagree.

use gpui_kit::*;
use wormhole_text::{keys, text};

use super::Changes;
use crate::platform::{self, Item, Menu};
use crate::settings::{settings_in, update_settings};

fn menu(tree: bool, group_untracked: bool) -> Menu {
    Menu::new()
        .item(
            Item::new(text(keys::SETTINGS_GIT_LIST))
                .checked(!tree)
                .on_select(|_, cx| update_settings(cx, |s| s.changes_tree = false)),
        )
        .item(
            Item::new(text(keys::SETTINGS_GIT_TREE))
                .checked(tree)
                .on_select(|_, cx| update_settings(cx, |s| s.changes_tree = true)),
        )
        .separator()
        .item(
            Item::new(text(keys::SETTINGS_GIT_GROUP_UNTRACKED))
                .checked(group_untracked)
                .on_select(|_, cx| update_settings(cx, |s| s.group_untracked = !s.group_untracked)),
        )
}

impl Changes {
    /// Opens the menu at the pointer, with the settings as they are now.
    pub(super) fn show_view_menu(&self, window: &mut Window, cx: &mut App) {
        let s = settings_in(&self.profile, cx);
        let menu = menu(s.changes_tree, s.group_untracked);
        platform::show_menu(menu, window.mouse_position(), window, cx);
    }
}

#[cfg(test)]
mod tests {
    use wormhole_text::{Language, with_language};

    use super::menu;

    #[test]
    fn the_menu_checks_how_the_list_is_shown() {
        with_language(Language::ENGLISH, || {
            assert_eq!(
                menu(true, false).outline(),
                ["- List", "- Tree [x]", "---", "- Untracked files separately"]
            );
            assert_eq!(menu(false, true).outline()[0], "- List [x]");
            assert!(menu(false, true).outline()[3].ends_with("[x]"));
        });
    }
}
