//! A commit's context menu in the History list, and the one of its opened details: the copies, and for
//! the details also "Show more" / "Show less" and the parents that can be opened. The menu is the
//! platform's own (`platform::show_menu`); every item acts on the History entity, which finds the commit
//! again by its hash, since the log may have moved while the menu was open. It is how the keys reach what
//! the buttons and links of the details do.

use gpui_kit::*;
use wormhole_text::{keys, text};

use super::History;
use super::copy::CopyWhat;
use crate::platform::{self, Item, Menu};

/// The title of a copy item, in the menus' words.
fn title(what: CopyWhat) -> String {
    let key = match what {
        CopyWhat::Hash => keys::HISTORY_MENU_COPY_HASH,
        CopyWhat::ShortHash => keys::HISTORY_MENU_COPY_SHORT_HASH,
        CopyWhat::Message => keys::HISTORY_MENU_COPY_MESSAGE,
        CopyWhat::Subject => keys::HISTORY_MENU_COPY_SUBJECT,
        CopyWhat::Author => keys::HISTORY_MENU_COPY_AUTHOR,
    };
    text(key).to_string()
}

/// What the menu of an opened commit's details lists besides the copies.
#[derive(Default)]
pub(super) struct Extras {
    /// "Show more" or "Show less", when the message has more to show.
    pub toggle: Option<bool>,
    /// The parents the log lists: full hash and short hash.
    pub parents: Vec<(String, String)>,
}

/// The menu of the commit `sha` of terminal `session`: the copies.
pub(super) fn commit_menu(this: &WeakEntity<History>, session: i64, sha: &str) -> Menu {
    details_menu(this, session, sha, Extras::default())
}

/// [`commit_menu`] with the details' own items first and last.
pub(super) fn details_menu(this: &WeakEntity<History>, session: i64, sha: &str, extras: Extras) -> Menu {
    let Extras { toggle, parents } = extras;
    let menu = match toggle {
        Some(expanded) => {
            let key = if expanded { keys::HISTORY_SHOW_LESS } else { keys::HISTORY_SHOW_MORE };
            Menu::new()
                .item(
                    Item::new(text(key).to_string())
                        .on_select(platform::run_on(this, move |history, _, cx| {
                            history.toggle_body(session, cx)
                        })),
                )
                .separator()
        }
        None => Menu::new(),
    };
    let menu = CopyWhat::ALL.into_iter().fold(menu, |menu, what| {
        let sha = sha.to_string();
        menu.item(Item::new(title(what)).on_select(platform::run_on(this, move |history, window, cx| {
            history.copy_commit(session, &sha, what, window, cx)
        })))
    });
    parents.into_iter().enumerate().fold(menu, |menu, (ix, (parent, short))| {
        let menu = if ix == 0 { menu.separator() } else { menu };
        menu.item(
            Item::new(keys::history_menu_open_parent(&short))
                .on_select(platform::run_on(this, move |history, _, cx| {
                    history.reveal_parent(session, parent.clone(), cx)
                })),
        )
    })
}

impl History {
    /// Opens the menu of the opened commit's details at `at`: where a right-click was, or under the row
    /// the keys are on.
    pub(super) fn show_details_menu(
        &mut self,
        session: i64,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get(&session) else { return };
        let Some(sha) = state.expanded.clone() else { return };
        let extras = state.details_of(&sha).map_or_else(Extras::default, |shown| Extras {
            toggle: shown.more_offered().then_some(state.body_expanded),
            parents: (shown.parents.iter())
                .filter(|parent| state.commits.iter().any(|c| c.sha == parent.sha))
                .map(|parent| (parent.sha.clone(), parent.short.to_string()))
                .collect(),
        });
        platform::show_menu(details_menu(&cx.weak_entity(), session, &sha, extras), at, window, cx);
    }

    /// ↩ on the details row: the same menu, under the row.
    pub(super) fn show_details_menu_at_row(
        &mut self,
        session: i64,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.of.get(&session) else { return };
        let at = state
            .list
            .state()
            .bounds_for_item(row)
            .map_or_else(|| window.mouse_position(), crate::focus_keys::key_menu_origin);
        self.show_details_menu(session, at, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::WeakEntity;
    use wormhole_text::{Language, with_language};

    use super::{Extras, commit_menu, details_menu};

    #[test]
    fn a_commit_offers_every_copy() {
        with_language(Language::ENGLISH, || {
            let menu = commit_menu(&WeakEntity::new_invalid(), 1, "abc");
            assert_eq!(
                menu.outline(),
                ["- Copy Hash", "- Copy Short Hash", "- Copy Message", "- Copy Subject", "- Copy Author"]
            );
        });
    }

    #[test]
    fn the_details_add_the_toggle_and_the_parents_they_can_open() {
        with_language(Language::ENGLISH, || {
            let extras = Extras { toggle: Some(false), parents: vec![("a".repeat(40), "aaaaaaa".into())] };
            let menu = details_menu(&WeakEntity::new_invalid(), 1, "abc", extras);
            assert_eq!(
                menu.outline(),
                [
                    "- Show more",
                    "---",
                    "- Copy Hash",
                    "- Copy Short Hash",
                    "- Copy Message",
                    "- Copy Subject",
                    "- Copy Author",
                    "---",
                    "- Open Parent aaaaaaa",
                ]
            );
            let shown = Extras { toggle: Some(true), parents: Vec::new() };
            assert_eq!(details_menu(&WeakEntity::new_invalid(), 1, "abc", shown).outline()[0], "- Show less");
        });
    }
}
