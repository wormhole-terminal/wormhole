//! What the native window shows of this Mac and of the app's own state, and the buttons that act on it: where an
//! update stands (the card of the Updates page), the default terminal, the login shell's choices, and the
//! Advanced page's files and tips. Each reads the same state the GPUI window does and calls the same code.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::config::Settings;
use wormhole_core::storage::paths::data_dir;
use wormhole_core::terminal::shells;
use wormhole_text::keys::{
    SETTINGS_TERMINAL_MAKE_DEFAULT, SETTINGS_TERMINAL_WORMHOLE_IS_THE_DEFAULT, SETTINGS_UPDATES_CHECK_NOW,
    SETTINGS_UPDATES_CHECKING, SETTINGS_UPDATES_DONT_RESTART, SETTINGS_UPDATES_DOWNLOADING,
    SETTINGS_UPDATES_INSTALL_UPDATE, SETTINGS_UPDATES_NOT_CHECKED_YET, SETTINGS_UPDATES_RESTART_NOW,
    SETTINGS_UPDATES_SKIP_THIS_VERSION, SETTINGS_UPDATES_WORMHOLE_IS_UP_TO_DATE,
    settings_updates_installed_restart, settings_updates_installed_waits,
};
use wormhole_text::text;

use crate::app_state::app_mut;
use crate::platform::default_app;
use crate::settings_window::sections::{available_text, checked_ago};
use crate::update::{self, UpdatePhase, Updates};

/// The row of the Updates page that shows the version.
const VERSION_ROW: &str = "Updates:1:0";
/// The Terminal page's Default terminal button.
const DEFAULT_TERMINAL_ROW: &str = "Terminal:1:1";

/// Values the window shows that are not settings: the version, whether Wormhole is the default terminal.
fn facts() -> Vec<(&'static str, String)> {
    let default = match default_app::is_default() {
        Some(true) => text(SETTINGS_TERMINAL_WORMHOLE_IS_THE_DEFAULT),
        _ => text(SETTINGS_TERMINAL_MAKE_DEFAULT),
    };
    vec![(VERSION_ROW, env!("CARGO_PKG_VERSION").to_string()), (DEFAULT_TERMINAL_ROW, default.to_string())]
}

/// Pages whose fixed notice does not apply now: a build with an update feed has no "development build" note.
pub(super) fn quiet_notices(cx: &App) -> Vec<&'static str> {
    let updates = cx.global::<Updates>();
    if matches!(updates.phase, UpdatePhase::Idle) && !updates.available() {
        Vec::new()
    } else {
        vec!["Updates"]
    }
}

/// The Updates page's status card: the words, their tone, the buttons, the meter. `null` where there is no feed
/// to ask (the page's notice says why).
pub(super) fn update_card(cx: &App) -> Value {
    let updates = cx.global::<Updates>();
    let (phase, last, pending) = (updates.phase.clone(), updates.last_check(), updates.restart_pending());
    if matches!(phase, UpdatePhase::Idle) && !updates.available() {
        return Value::Null;
    }
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let status = match &phase {
        UpdatePhase::Idle => {
            last.map_or_else(|| text(SETTINGS_UPDATES_NOT_CHECKED_YET).into(), |l| checked_ago(l, now))
        }
        UpdatePhase::Checking => text(SETTINGS_UPDATES_CHECKING).into(),
        UpdatePhase::UpToDate => text(SETTINGS_UPDATES_WORMHOLE_IS_UP_TO_DATE).into(),
        UpdatePhase::Available(release) => available_text(release),
        UpdatePhase::Unsupported(message) | UpdatePhase::Failed(message) => message.clone(),
        UpdatePhase::Downloading => text(SETTINGS_UPDATES_DOWNLOADING).into(),
        UpdatePhase::Installed(v) if pending => settings_updates_installed_waits(v),
        UpdatePhase::Installed(v) => settings_updates_installed_restart(v),
    };
    let hint = match (&phase, last) {
        (UpdatePhase::UpToDate, Some(last)) => checked_ago(last, now),
        _ => String::new(),
    };
    let tone = match &phase {
        UpdatePhase::Failed(_) | UpdatePhase::Unsupported(_) => "warn",
        UpdatePhase::Installed(_) | UpdatePhase::UpToDate => "ok",
        _ => "muted",
    };
    let words = |key| text(key).to_string();
    let buttons: Vec<String> = match &phase {
        UpdatePhase::Available(release) if release.is_security_update() => {
            vec![words(SETTINGS_UPDATES_INSTALL_UPDATE)]
        }
        UpdatePhase::Available(_) => {
            vec![words(SETTINGS_UPDATES_SKIP_THIS_VERSION), words(SETTINGS_UPDATES_INSTALL_UPDATE)]
        }
        UpdatePhase::Installed(_) if pending => {
            vec![words(SETTINGS_UPDATES_DONT_RESTART), words(SETTINGS_UPDATES_RESTART_NOW)]
        }
        UpdatePhase::Installed(_) => vec![words(SETTINGS_UPDATES_RESTART_NOW)],
        UpdatePhase::Checking | UpdatePhase::Downloading => Vec::new(),
        _ => vec![words(SETTINGS_UPDATES_CHECK_NOW)],
    };
    // Installing and restarting are the next step; checking again is not.
    let primary = if matches!(phase, UpdatePhase::Available(_) | UpdatePhase::Installed(_)) {
        buttons.len() as i64 - 1
    } else {
        -1
    };
    let (done, total) = updates.progress();
    let percent = if matches!(phase, UpdatePhase::Downloading) {
        (done * 100).checked_div(total).unwrap_or(0) as i64
    } else {
        -1
    };
    json!({ "status": status, "hint": hint, "tone": tone, "buttons": buttons, "busy": updates.busy(), "percent": percent, "primary": primary })
}

/// The shells to choose from: (key stored, name shown), the current default last when it names nothing any more.
fn shell_choices(s: &Settings) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = shells::choices(s).into_iter().map(|c| (c.key, c.name)).collect();
    let current = shells::default_key(s).to_string();
    if !out.iter().any(|(key, _)| *key == current) {
        out.push((current.clone(), current));
    }
    out
}

/// The default shell as the window names it, and the names to choose from.
pub(super) fn shell(s: &Settings) -> (String, Vec<String>) {
    let choices = shell_choices(s);
    let current = shells::default_key(s).to_string();
    let name =
        choices.iter().find(|(key, _)| *key == current).map(|(_, name)| name.clone()).unwrap_or(current);
    (name, choices.into_iter().map(|(_, name)| name).collect())
}

/// Make the shell called `name` the default. Whether there is one.
pub(super) fn set_shell(s: &mut Settings, name: &str) -> bool {
    match shell_choices(s).into_iter().find(|(_, shown)| shown == name) {
        Some((key, _)) => {
            s.default_shell = key;
            true
        }
        None => false,
    }
}

/// A button of the Updates, Terminal or Advanced pages. Whether it was one of these.
pub(super) fn perform(key: &str, name: &str, cx: &mut App) -> bool {
    let is = |k| text(k) == name;
    match key {
        "Updates:" => {
            if is(SETTINGS_UPDATES_CHECK_NOW) {
                if !cx.global::<Updates>().busy() {
                    update::check_for_updates(true, cx);
                }
            } else if is(SETTINGS_UPDATES_INSTALL_UPDATE) || is(SETTINGS_UPDATES_SKIP_THIS_VERSION) {
                if let UpdatePhase::Available(release) = cx.global::<Updates>().phase.clone() {
                    if is(SETTINGS_UPDATES_INSTALL_UPDATE) {
                        update::install_update(*release, cx);
                    } else if !release.is_security_update() {
                        update::skip_update(&release, cx);
                    }
                }
            } else if is(SETTINGS_UPDATES_DONT_RESTART) {
                update::cancel_restart_when_idle(cx);
            } else if is(SETTINGS_UPDATES_RESTART_NOW) {
                restart(cx);
            }
            true
        }
        DEFAULT_TERMINAL_ROW => {
            if default_app::is_default() != Some(true)
                && let Err(err) = default_app::make_default()
            {
                tracing::warn!("making Wormhole the default terminal: {err:#}");
            }
            true
        }
        "Advanced:0:1" => {
            crate::settings_window::open_folder(&data_dir(), cx);
            true
        }
        "Advanced:0:2" => {
            crate::settings_window::open_folder(&data_dir().join("logs"), cx);
            true
        }
        "Advanced:1:0" => {
            app_mut(cx).reset_tips();
            cx.refresh_windows();
            true
        }
        "Advanced:1:1" => {
            crate::platform::dev_tools::open_settings(cx);
            true
        }
        _ => false,
    }
}

/// Restart to finish the update: the window in front asks first when something runs.
fn restart(cx: &mut App) {
    match cx.active_window() {
        Some(window) => {
            let _ = window.update(cx, |_, window, cx| update::restart::request_restart(window, cx));
        }
        None => update::restart_to_update(cx),
    }
}

/// The facts as a map for the snapshot.
pub(super) fn fact_values() -> Map<String, Value> {
    facts().into_iter().map(|(k, v)| (k.to_string(), Value::String(v))).collect()
}
