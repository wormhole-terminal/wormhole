//! The menu of a file in the git panel (a Changes row, a commit's file): open it, reveal it, copy its
//! path, show its history. It is plain data (`platform::Menu`), so a list adds only what is its own, and
//! a tab that is off just has no History to offer.

use gpui_kit::*;
use wormhole_core::storage::files;
use wormhole_core::view::notice::Copied;
use wormhole_text::{keys, text};

use super::history::History;
use super::open_file::OpenTarget;
use crate::platform::{self, Item, Menu};

pub(crate) struct PathMenu<'a> {
    pub open: &'a OpenTarget,
    /// Relative to the repository's root.
    pub path: &'a str,
    /// A deleted file has nothing to open or reveal.
    pub on_disk: bool,
    /// Where "File History" shows the log, and for which terminal; `None` while History is off.
    pub history: Option<(WeakEntity<History>, i64)>,
}

impl PathMenu<'_> {
    pub(crate) fn menu(self) -> Menu {
        let Self { open, path, on_disk, history } = self;
        let file = open.file(path);
        let (root, relative) = (open.root().to_path_buf(), path.to_string());
        let reveal = {
            let (root, relative) = (root.clone(), relative.clone());
            move |_: &mut Window, cx: &mut App| match files::resolve_within(&root, &relative) {
                Ok(full) => cx.reveal_path(&full),
                Err(err) => tracing::warn!("not revealing {relative}: {err}"),
            }
        };
        let copy = |what: String| {
            move |window: &mut Window, cx: &mut App| {
                platform::clipboard::write(what.clone(), cx);
                crate::notice::post(window, Copied::Path, None, cx);
            }
        };
        let absolute = root.join(path).to_string_lossy().into_owned();
        Menu::new()
            // With Finder as the app, opening is revealing: one item, not two.
            .when(!open.opens_in_finder(), |menu| {
                menu.item(
                    Item::new(file.tooltip(None)).enabled(on_disk).on_select(move |_, _| file.open(None)),
                )
            })
            .item(Item::new(text(keys::HISTORY_MENU_REVEAL)).enabled(on_disk).on_select(reveal))
            .separator()
            .item(Item::new(text(keys::HISTORY_MENU_COPY_PATH)).on_select(copy(absolute)))
            .item(Item::new(text(keys::HISTORY_MENU_COPY_RELATIVE_PATH)).on_select(copy(relative.clone())))
            .when(history.is_some(), |menu| {
                let Some((this, session)) = history else { return menu };
                menu.separator().item(Item::new(text(keys::HISTORY_MENU_FILE_HISTORY)).on_select(
                    platform::run_on(&this, move |history, _, cx| {
                        history.show_paths(session, vec![relative.clone()], cx)
                    }),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use gpui_kit::WeakEntity;
    use wormhole_core::platform::open_in::{EDITORS, FINDER};
    use wormhole_text::{Language, with_language};

    use super::PathMenu;
    use crate::features::git::open_file::OpenTarget;

    fn outline(
        app: impl Into<wormhole_core::platform::open_in::Installed>,
        on_disk: bool,
        history: bool,
    ) -> Vec<String> {
        let open = OpenTarget::with_app(Path::new("/repo"), app.into());
        let history = history.then(|| (WeakEntity::new_invalid(), 1));
        PathMenu { open: &open, path: "src/a.rs", on_disk, history }.menu().outline()
    }

    #[test]
    fn a_file_offers_opening_its_copies_and_its_history() {
        with_language(Language::ENGLISH, || {
            let editor = EDITORS[0];
            let all = outline(editor, true, true);
            assert_eq!(all[0], format!("- Open in {}", editor.name));
            assert_eq!(
                &all[1..],
                ["- Reveal in Finder", "---", "- Copy Path", "- Copy Relative Path", "---", "- File History"]
            );
            // A deleted file cannot be opened or revealed; History off leaves no History item.
            let gone = outline(editor, false, false);
            assert!(gone[0].ends_with("[off]") && gone[1].ends_with("[off]"), "{gone:?}");
            assert_eq!(gone.len(), 5);
        });
    }

    #[test]
    fn with_finder_chosen_opening_is_not_listed_twice() {
        with_language(Language::ENGLISH, || {
            assert_eq!(outline(FINDER, true, false)[0], "- Reveal in Finder");
            assert_eq!(outline(FINDER, true, false).len(), 4);
        });
    }
}
