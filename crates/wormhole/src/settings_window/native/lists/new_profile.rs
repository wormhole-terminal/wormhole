//! The "New profile" sheet: a name and what the profile starts from. The window draws the sheet from the data below;
//! the app decides when it is open (the Profile page's button, the File menu, the palette) and carries out Create:
//! it makes the profile and switches Wormhole to it, or says why it could not.

use gpui_kit::App;
use serde_json::{Value, json};
use wormhole_core::config::profile::{NewProfile, StartFrom};
use wormhole_core::storage::paths::ProfileId;
use wormhole_text::keys::{
    SETTINGS_PROFILE_CANCEL, SETTINGS_PROFILE_CREATE, SETTINGS_PROFILE_GIVE_THE_PROFILE_A_NAME,
    SETTINGS_PROFILE_LOGINS_NEVER_COPIED, SETTINGS_PROFILE_NAME_FIELD, SETTINGS_PROFILE_NEW_PROFILE,
    SETTINGS_PROFILE_START_FROM, SETTINGS_PROFILE_WORMHOLES_DEFAULTS, settings_profile_start_from_copy,
};
use wormhole_text::text;

use crate::app_state::{AppState, app};

/// The id of the sheet in the snapshot, and of its buttons' actions.
pub(super) const SHEET: &str = "new-profile";
pub(in crate::settings_window::native) const OPEN: &str = "new_profile_open";
const CANCEL: &str = "new_profile_cancel";
/// `new_profile_create:<start from>\n<name>`.
const CREATE: &str = "new_profile_create:";
const DEFAULTS: &str = "defaults";
const COPY: &str = "copy:";

/// An open sheet: what Create said last, if it refused.
#[derive(Default)]
pub(in crate::settings_window::native) struct State {
    pub error: Option<String>,
}

/// What the sheet starts from, as (id, words): a copy of each profile, the profile Wormhole is in first, then the
/// defaults.
fn options(cx: &App) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = app(cx)
        .profiles
        .profiles()
        .iter()
        .map(|p| (format!("{COPY}{}", p.id.as_str()), settings_profile_start_from_copy(&p.name)))
        .collect();
    options.push((DEFAULTS.into(), text(SETTINGS_PROFILE_WORMHOLES_DEFAULTS).into()));
    options
}

/// The sheet for the snapshot: `open` says whether it is up.
pub(super) fn sheet(open: Option<&State>, cx: &App) -> Value {
    let options = options(cx);
    let shown = format!("{COPY}{}", app(cx).active_profile().as_str());
    let chosen = options.iter().find(|(id, _)| *id == shown).map_or(DEFAULTS, |(id, _)| id.as_str());
    json!({
        "open": open.is_some(), "error": open.and_then(|s| s.error.clone()).unwrap_or_default(),
        "title": text(SETTINGS_PROFILE_NEW_PROFILE), "name": text(SETTINGS_PROFILE_NAME_FIELD),
        "from": text(SETTINGS_PROFILE_START_FROM), "options": options, "chosen": chosen,
        "note": text(SETTINGS_PROFILE_LOGINS_NEVER_COPIED),
        "cancel": text(SETTINGS_PROFILE_CANCEL), "create": text(SETTINGS_PROFILE_CREATE),
    })
}

/// Put the sheet up (a second call starts it afresh).
pub(in crate::settings_window::native) fn open(cx: &mut App) {
    super::super::set_new_profile(Some(State::default()), cx);
}

/// A button of the sheet. Whether `action` was one of its.
pub(super) fn perform(action: &str, cx: &mut App) -> bool {
    if action == OPEN {
        open(cx);
    } else if action == CANCEL {
        super::super::set_new_profile(None, cx);
    } else if let Some(typed) = action.strip_prefix(CREATE) {
        let (from, name) = typed.split_once('\n').unwrap_or((typed, ""));
        create(from, name.trim(), cx);
    } else {
        return false;
    }
    true
}

fn create(from: &str, name: &str, cx: &mut App) {
    let refuse = |message: String, cx: &mut App| {
        super::super::set_new_profile(Some(State { error: Some(message) }), cx);
    };
    if name.is_empty() {
        return refuse(text(SETTINGS_PROFILE_GIVE_THE_PROFILE_A_NAME).into(), cx);
    }
    let from = match from.strip_prefix(COPY).map(ProfileId::parse) {
        Some(Ok(id)) => StartFrom::Copy(id),
        Some(Err(err)) => return refuse(err.to_string(), cx),
        None => StartFrom::Defaults,
    };
    match AppState::create_profile(NewProfile { name: name.to_string(), color: None, avatar: None, from }, cx)
    {
        Ok(id) => {
            super::super::set_new_profile(None, cx);
            // Wormhole goes to the new profile (deferred: the windows are updating now).
            cx.defer(move |cx| {
                if let Err(message) = AppState::switch_profile(&id, cx) {
                    tracing::warn!("switching to the new profile: {message}");
                }
            });
        }
        Err(message) => refuse(message, cx),
    }
}
