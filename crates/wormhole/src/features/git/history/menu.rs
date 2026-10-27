//! A commit's context menu in the History list: the copies it offers. The menu is the platform's own
//! (`platform::show_menu`); every item acts on the History entity, which finds the commit again by its
//! hash, since the log may have moved while the menu was open.

use gpui_kit::*;

use super::History;
use super::copy::CopyWhat;
use crate::platform::{self, Item, Menu};

/// The title of an item, in the menus' wording.
fn title(what: CopyWhat) -> &'static str {
    match what {
        CopyWhat::Hash => "Copy Hash",
        CopyWhat::ShortHash => "Copy Short Hash",
        CopyWhat::Message => "Copy Message",
        CopyWhat::Subject => "Copy Subject",
        CopyWhat::Author => "Copy Author",
    }
}

/// The menu of the commit `sha` of terminal `session`.
pub(super) fn commit_menu(this: &WeakEntity<History>, session: i64, sha: &str) -> Menu {
    CopyWhat::ALL.into_iter().fold(Menu::new(), |menu, what| {
        let sha = sha.to_string();
        menu.item(Item::new(title(what)).on_select(platform::run_on(this, move |history, window, cx| {
            history.copy_commit(session, &sha, what, window, cx)
        })))
    })
}

#[cfg(test)]
mod tests {
    use gpui_kit::WeakEntity;

    use super::commit_menu;

    #[test]
    fn a_commit_offers_every_copy() {
        let menu = commit_menu(&WeakEntity::new_invalid(), 1, "abc");
        assert_eq!(
            menu.outline(),
            ["- Copy Hash", "- Copy Short Hash", "- Copy Message", "- Copy Subject", "- Copy Author"]
        );
    }
}
