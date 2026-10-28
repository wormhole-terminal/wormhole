//! The two menus of the tab, both the platform's own: the path menu under the button at the top (like the
//! one a Finder window's title opens: the folder and its parents, the pins, then Pin, Choose Folder… and
//! Follow Terminal) and the menu of a row. Each item acts on the `Files` entity, which finds the terminal
//! again by its id, since the list may have moved while the menu was open.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::platform::open_in::Installed;
use wormhole_core::view::file_tree::{PathMenu, Pin, Row, on_boot_volume};
use wormhole_text::{keys, text};

use super::{Files, open};
use crate::open_in::OpenIn;
use crate::platform::{self, Icon, Item, Menu};

impl Files {
    /// The path menu of `session`'s tree; `None` while the tree has no folder.
    pub(super) fn path_menu_for(&self, session: i64, cx: &mut Context<Self>) -> Option<Menu> {
        let of = self.of.get(&session)?;
        let model = PathMenu::of(
            of.root.as_deref()?,
            &platform::boot_volume_name(),
            &self.pinned(cx),
            of.chosen.is_some(),
            crate::style::home_dir(),
        );
        Some(path_menu(model, session, &cx.weak_entity()))
    }

    /// Opens the path menu at `at`: under the button, or where the keys are.
    pub(super) fn show_path_menu(
        &mut self,
        session: i64,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = self.path_menu_for(session, cx) {
            platform::show_menu(menu, at, window, cx);
        }
    }

    /// Opens the menu of `row` at `at`.
    pub(super) fn show_row_menu(
        &mut self,
        session: i64,
        row: Row,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.of.get(&session).and_then(|of| of.root.clone()) else { return };
        let pinned = row.is_folder() && self.pinned(cx).contains(&root.join(&row.path));
        let apps = OpenIn::apps(cx)
            .into_iter()
            .filter(|app| open::may_open_in(&app.bundle, &row))
            .map(|app| {
                let icon = OpenIn::menu_icon(cx, &app);
                (app, icon)
            })
            .collect();
        let chosen = OpenIn::resolved(cx).app.bundle;
        let menu = row_menu(&row, RowMenu { root, apps, chosen, pinned }, &cx.weak_entity());
        platform::show_menu(menu, at, window, cx);
    }

    /// A folder picked in the path menu: the tree is of it from now on, until the terminal is followed
    /// again.
    pub(super) fn choose(&mut self, session: i64, folder: PathBuf, cx: &mut Context<Self>) {
        if !folder.is_absolute() {
            return;
        }
        self.of_mut(session).chosen = Some(folder);
        self.retarget(session, cx);
        cx.notify();
    }

    /// Follow Terminal: the tree is of the terminal's folder again.
    pub(super) fn follow_terminal(&mut self, session: i64, cx: &mut Context<Self>) {
        self.of_mut(session).chosen = None;
        self.retarget(session, cx);
        cx.notify();
    }

    /// Choose Folder…: the system's panel, folders only.
    pub(super) fn pick_folder(&mut self, session: i64, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(text(keys::FILES_CHOOSE_PROMPT).into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(folder) = paths.into_iter().next() else { return };
            this.update(cx, |files, cx| files.choose(session, folder, cx)).ok();
        })
        .detach();
    }
}

/// The icon of a folder in the path menu: the one Finder shows for it, unless it is not on this Mac's own disk
/// (a share, another volume): asking the system for that icon would touch the share, so it gets the generic
/// folder's.
fn folder_icon(path: &Path) -> Icon {
    if on_boot_volume(path) { Icon::File(path.to_path_buf()) } else { Icon::Folder }
}

/// The path menu for `model`: the folder (checked) and its parents with Finder's icons, the pins, then
/// the pin command, Choose Folder… and, if a folder was chosen, Follow Terminal.
pub(super) fn path_menu(model: PathMenu, session: i64, this: &WeakEntity<Files>) -> Menu {
    let go = |path: &Path| {
        let path = path.to_path_buf();
        platform::run_on(this, move |files, _, cx| files.choose(session, path.clone(), cx))
    };
    let mut menu = Menu::new();
    for (at, folder) in model.folders.iter().enumerate() {
        menu = menu.item(
            Item::new(folder.name.clone())
                .icon(Some(folder_icon(&folder.path)))
                .checked(at == 0)
                .on_select(go(&folder.path)),
        );
    }
    menu = menu.separator();
    for pinned in &model.pinned {
        menu = menu.item(
            Item::new(pinned.title.clone()).icon(Some(folder_icon(&pinned.path))).on_select(go(&pinned.path)),
        );
    }
    let shown = model.folders.first().map(|folder| folder.path.clone());
    let pin = match &model.pin {
        Pin::Pin(name) => (keys::files_menu_pin(name), true),
        Pin::Unpin(name) => (keys::files_menu_unpin(name), false),
    };
    menu = menu.separator().item(Item::new(pin.0).enabled(shown.is_some()).on_select(platform::run_on(
        this,
        move |files, _, cx| {
            if let Some(folder) = &shown {
                files.set_pinned(folder, pin.1, cx);
            }
        },
    )));
    menu = menu.item(
        Item::new(text(keys::FILES_MENU_CHOOSE_FOLDER))
            .on_select(platform::run_on(this, move |files, _, cx| files.pick_folder(session, cx))),
    );
    menu.when(model.can_follow, |menu| {
        menu.item(
            Item::new(text(keys::FILES_MENU_FOLLOW_TERMINAL))
                .on_select(platform::run_on(this, move |files, _, cx| files.follow_terminal(session, cx))),
        )
    })
}

/// What the menu of a row needs besides the row.
pub(super) struct RowMenu {
    /// The tree's root, which the row's path is below.
    pub root: PathBuf,
    /// The apps Open in… offers, with their icons.
    pub apps: Vec<(Installed, Option<Icon>)>,
    /// The bundle identifier of the app a double-click opens a file in: checked in the list.
    pub chosen: String,
    /// The row is a folder that is pinned.
    pub pinned: bool,
}

/// The menu of `row`: Open in (the apps), Reveal in Finder, Copy Path and, on a folder, Pin or Unpin.
pub(super) fn row_menu(row: &Row, facts: RowMenu, this: &WeakEntity<Files>) -> Menu {
    let RowMenu { root, apps, chosen, pinned } = facts;
    let title = text(keys::FILES_ROW_OPEN_IN);
    let open_in = if open::can_open(row) {
        let apps = apps.into_iter().fold(Menu::new(), |menu, (app, icon)| {
            let (bundle, root, row) = (app.bundle.clone(), root.clone(), row.clone());
            menu.item(
                Item::new(app.name)
                    .icon(icon)
                    .checked(app.bundle == chosen)
                    .on_select(move |_, cx| open::open_in(&bundle, &root, &row, cx)),
            )
        });
        Some(apps)
    } else {
        None
    };
    let menu = match open_in {
        Some(apps) => Menu::new().submenu(title, apps),
        // A link is not opened: it may lead anywhere.
        None => Menu::new().disabled_submenu(title),
    };
    let (reveal_root, reveal_row) = (root.clone(), row.clone());
    let (copy_root, copy_row) = (root.clone(), row.clone());
    let menu = menu
        .item(
            Item::new(text(keys::FILES_ROW_REVEAL))
                .on_select(move |_, _| open::reveal(&reveal_root, &reveal_row)),
        )
        .item(
            Item::new(text(keys::FILES_ROW_COPY_PATH))
                .on_select(move |window, cx| open::copy_path(&copy_root, &copy_row, window, cx)),
        );
    if !row.is_folder() {
        return menu;
    }
    let folder = root.join(&row.path);
    let title = if pinned { keys::FILES_ROW_UNPIN_FOLDER } else { keys::FILES_ROW_PIN_FOLDER };
    menu.separator().item(Item::new(text(title)).on_select(platform::run_on(this, move |files, _, cx| {
        files.set_pinned(&folder, !pinned, cx);
    })))
}
