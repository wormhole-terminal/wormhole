//! The "Open folders in" row of the native window: the apps on this Mac that can open a folder (the title row's
//! button opens the chosen one), as the menu's options, with the bundle identifier as each option's id. The window
//! draws each app's icon from that id, and "Choose app…" asks for one that is not in the list.

use std::path::Path;

use gpui_kit::App;
use wormhole_core::config::Settings;
use wormhole_text::keys::SETTINGS_TERMINAL_NOT_INSTALLED_THE_TITLE_ROW_OPENS;
use wormhole_text::text;

use super::codec::Choice;
use super::words::word;
use crate::open_in::OpenIn;
use crate::platform::apps;
use crate::settings::update_settings;

/// The row's id in the settings index.
pub(super) const ROW: &str = "open-in";
/// The menu's last item: the id the window turns into a file panel.
const CHOOSE: &str = "choose_app";

/// What the snapshot says of the row: the value (the app the title row opens, or the stored one that is gone), the
/// default, and the menu.
pub(super) struct Row {
    pub value: String,
    pub default: String,
    pub options: Vec<Choice>,
}

pub(super) fn row(cx: &App) -> Row {
    let resolved = OpenIn::resolved(cx);
    let mut options: Vec<Choice> =
        OpenIn::apps(cx).into_iter().map(|app| (app.bundle, app.name)).collect();
    let value = match &resolved.missing {
        // An app that is gone is shown as written, with what happens instead.
        Some(missing) => {
            let chosen = crate::settings::settings(cx).open_in.trim().to_string();
            let detail = text(SETTINGS_TERMINAL_NOT_INSTALLED_THE_TITLE_ROW_OPENS);
            options.push((chosen.clone(), format!("{missing} ({detail})")));
            chosen
        }
        None => resolved.app.bundle.clone(),
    };
    options.push(("-".into(), "-".into()));
    options.push((CHOOSE.into(), word("Choose app…")));
    let default = wormhole_core::platform::open_in::resolve("", &OpenIn::apps(cx)).app.bundle;
    Row { value, default, options }
}

/// Make the app with this bundle identifier the one folders open in.
pub(super) fn write(s: &mut Settings, bundle: &str) -> bool {
    s.open_in = bundle.to_string();
    true
}

/// The app the person picked in the file panel, by its bundle identifier; anything that is not an app bundle is
/// ignored.
pub(super) fn chosen(path: &Path, cx: &mut App) {
    if let Some(bundle) = apps::bundle_of(path) {
        update_settings(cx, |s| s.open_in = bundle);
    }
}
