//! Open in…: which apps a terminal's folder can be opened in on this
//! Mac, what their icons look like, and opening a folder in one. Which apps are offered and which
//! one a click picks are core's `open_in`; this asks the platform what is installed (by bundle
//! identifier) and keeps the icons it draws.
//!
//! Nothing here runs anything from a project: the person asks an app they chose to open a folder,
//! as when they drag it onto the app, so project trust (`ProjectHub::enabled`) doesn't come into it.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::platform::open_file;
use wormhole_core::platform::open_in::{self, EDITORS, Installed, Resolved};

use crate::platform::apps;
use crate::settings::settings;

/// The apps found on this Mac, and the icons drawn for them so far. A GPUI global, set up when the
/// app starts and looked at again (`refresh`) when Settings opens, shows its Terminal page or comes
/// forward: a lookup is a LaunchServices query for each editor, so never from a draw.
pub(crate) struct OpenIn {
    /// The bundle identifiers of the table's apps that are installed.
    found: Vec<String>,
    /// The icons, by bundle identifier: drawn once, `None` for an app whose icon can't be read.
    icons: RefCell<HashMap<String, Option<Arc<Image>>>>,
}

impl Global for OpenIn {}

/// The icon is drawn this many pixels square: four times the 16 pt it is shown at, so it stays
/// sharp on a Retina display and when the window is scaled.
const ICON_PX: usize = 64;

impl OpenIn {
    /// Look for the apps and keep the result as a global.
    pub(crate) fn install(cx: &mut App) {
        let found = installed();
        cx.set_global(Self { found, icons: RefCell::new(HashMap::new()) });
    }

    /// Look again (an app was installed or removed since): the icons already drawn are kept. The
    /// global is only set when the apps differ, since setting it tells everything that watches it
    /// (every window's regions).
    pub(crate) fn refresh(cx: &mut App) {
        let found = installed();
        match cx.try_global::<Self>() {
            Some(this) if this.found == found => {}
            Some(this) => {
                let icons = this.icons.take();
                cx.set_global(Self { found, icons: RefCell::new(icons) });
            }
            None => cx.set_global(Self { found, icons: RefCell::new(HashMap::new()) }),
        }
    }

    /// The apps the menus list: the table's installed ones, the chosen one if it is a stranger to
    /// the table but installed, and Finder.
    pub(crate) fn apps(cx: &App) -> Vec<Installed> {
        let found = cx.try_global::<Self>().map(|this| this.found.clone()).unwrap_or_default();
        let chosen = settings(cx).open_in.trim().to_string();
        let stranger = (!chosen.is_empty() && open_in::known(&chosen).is_none())
            .then(|| apps::name_of(&chosen))
            .flatten()
            .map(|name| Installed { monogram: open_in::monogram(&name), bundle: chosen, name });
        open_in::listing(&found, stranger)
    }

    /// What a click on the title row's button opens.
    pub(crate) fn resolved(cx: &App) -> Resolved {
        open_in::resolve(&settings(cx).open_in, &Self::apps(cx))
    }

    /// The icon a menu item of the app has: the file icon of its `.app`, which AppKit gives as Finder
    /// shows it; for an app whose place isn't known, the one drawn from its bundle (below), if that
    /// can be read; else none.
    pub(crate) fn menu_icon(cx: &App, app: &Installed) -> Option<crate::platform::Icon> {
        use crate::platform::Icon;
        match apps::app_path(&app.bundle) {
            Some(path) => Some(Icon::File(path)),
            None => Some(match Self::icon(cx, &app.bundle) {
                Some(image) => Icon::Image(image),
                // What the old menu showed where the icon couldn't be read.
                None => Icon::Monogram(app.monogram.clone().into()),
            }),
        }
    }

    /// The app's own icon, drawn at [`ICON_PX`]; `None` when it can't be read (the button shows
    /// the app's monogram instead).
    pub(crate) fn icon(cx: &App, bundle: &str) -> Option<Arc<Image>> {
        let this = cx.try_global::<Self>()?;
        if let Some(icon) = this.icons.borrow().get(bundle) {
            return icon.clone();
        }
        let icon =
            apps::icon_png(bundle, ICON_PX).map(|png| Arc::new(Image::from_bytes(ImageFormat::Png, png)));
        this.icons.borrow_mut().insert(bundle.to_string(), icon.clone());
        icon
    }
}

/// Open `folder` in the app with this bundle identifier (Finder shows it as a window).
pub(crate) fn open(bundle: &str, folder: &Path) {
    // A folder is always absolute here, so it can't be read as an option. Finder is not asked to open
    // one that is a program to the system (`x.app`): it would run it.
    if let Err(err) = apps::launch(bundle, open_file::folder(bundle, folder)) {
        tracing::warn!("opening {} in {bundle}: {err}", folder.display());
    }
}

/// The table's apps that the system knows by their bundle identifier.
fn installed() -> Vec<String> {
    EDITORS
        .iter()
        .filter(|editor| apps::app_path(editor.bundle).is_some())
        .map(|e| e.bundle.to_string())
        .collect()
}
