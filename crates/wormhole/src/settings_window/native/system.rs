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
    SETTINGS_AGENTS_INSTALL, SETTINGS_AGENTS_UNINSTALL, SETTINGS_AGENTS_UPDATE,
    SETTINGS_TERMINAL_MAKE_DEFAULT, SETTINGS_TERMINAL_WORMHOLE_IS_THE_DEFAULT, SETTINGS_UPDATES_CHECK_NOW,
    SETTINGS_UPDATES_CHECKING, SETTINGS_UPDATES_DONT_RESTART, SETTINGS_UPDATES_DOWNLOADING,
    SETTINGS_UPDATES_INSTALL_UPDATE, SETTINGS_UPDATES_NOT_CHECKED_YET, SETTINGS_UPDATES_RESTART_NOW,
    SETTINGS_UPDATES_SKIP_THIS_VERSION, SETTINGS_UPDATES_WORMHOLE_IS_UP_TO_DATE,
    SETTINGS_WINDOW_CONFIG_JSON_CANT_BE_READ_SO, SETTINGS_WINDOW_OPEN, settings_agents_cli_foreign_at,
    settings_agents_cli_installed_at, settings_agents_cli_not_installed, settings_updates_installed_restart,
    settings_updates_installed_waits,
};
use wormhole_text::text;

use super::cli_tool::{binary as cli_binary, current_status as cli_status, shown as cli_shown};
use super::update_words::{available_text, checked_ago};
use crate::app_state::app_mut;
use crate::platform::default_app;
use crate::settings::SettingsStore;
use crate::update::{self, UpdatePhase, Updates};
use wormhole_core::ipc::control::COMMAND_NAME;
use wormhole_core::terminal::cli_tool::{self, Status as CliStatus};

/// The row of the Updates page that shows the version.
const VERSION_ROW: &str = "Updates:1:0"; // text: not shown (a key or page id of the native window, or a log line)
/// The Command line tool button of the Coding agents page.
const CLI_TOOL_ROW: &str = "Agents:1:0"; // text: not shown (a key or page id of the native window, or a log line)
/// The page whose buttons are the update card's.
const UPDATES_PAGE: &str = "Updates:"; // text: not shown (a page key of the native window)
/// The Advanced page's buttons.
const DATA_FOLDER_ROW: &str = "Advanced:0:1"; // text: not shown (a row key of the native window)
const LOGS_ROW: &str = "Advanced:0:2"; // text: not shown (a row key of the native window)
const RESET_TIPS_ROW: &str = "Advanced:1:0"; // text: not shown (a row key of the native window)
const DEV_TOOLS_ROW: &str = "Advanced:1:1"; // text: not shown (a row key of the native window)
/// The Terminal page's Default terminal button.
const DEFAULT_TERMINAL_ROW: &str = "Terminal:1:1"; // text: not shown (a key or page id of the native window, or a log line)

/// Values the window shows that are not settings: the version, whether Wormhole is the default terminal.
fn facts() -> Vec<(&'static str, String)> {
    let default = match default_app::is_default() {
        Some(true) => text(SETTINGS_TERMINAL_WORMHOLE_IS_THE_DEFAULT),
        _ => text(SETTINGS_TERMINAL_MAKE_DEFAULT),
    };
    // One button for the tool: Install, Update when the script is older than this app, Uninstall when it is current.
    let cli = match cli_status() {
        CliStatus::Installed { current: true, .. } => text(SETTINGS_AGENTS_UNINSTALL),
        CliStatus::Installed { .. } => text(SETTINGS_AGENTS_UPDATE),
        _ => text(SETTINGS_AGENTS_INSTALL),
    };
    vec![
        (VERSION_ROW, env!("CARGO_PKG_VERSION").to_string()),
        (DEFAULT_TERMINAL_ROW, default.to_string()),
        (CLI_TOOL_ROW, cli.to_string()),
    ]
}

/// Where the `wh` command stands, in words, for the line under the Command line tool button.
pub(super) fn cli_tool_line() -> String {
    match cli_status() {
        CliStatus::NotInstalled => settings_agents_cli_not_installed(COMMAND_NAME),
        CliStatus::Installed { path, .. } => {
            settings_agents_cli_installed_at(COMMAND_NAME, &cli_shown(&path))
        }
        CliStatus::Foreign { path } => settings_agents_cli_foreign_at(COMMAND_NAME, &cli_shown(&path)),
    }
}

/// Install, update or uninstall the `wh` command, as the GPUI page's button does.
fn cli_tool_press() {
    let targets = cli_tool::Targets::from_env();
    match cli_status() {
        CliStatus::Installed { current: true, .. } => {
            if let Err(err) = cli_tool::uninstall(&targets) {
                tracing::warn!("removing the command line tool: {err}");
            }
        }
        // Someone else's `wh` is in the way: the line says whose, and nothing is overwritten.
        CliStatus::Foreign { .. } => {}
        _ => match cli_tool::install(&targets, &cli_binary()) {
            Ok(done) => tracing::info!(path = %done.path.display(), "installed the command line tool"),
            Err(cli_tool::Error::Foreign(_)) => {}
            Err(err) => tracing::warn!("installing the command line tool: {err}"),
        },
    }
}

/// What is wrong with config.json, for the top of every page: the parse error with the way to the file, or that
/// changes are not saved while it cannot be read. `null` when it is fine.
pub(super) fn banner(cx: &App) -> Value {
    let store = cx.global::<SettingsStore>();
    match &store.error {
        Some(error) => {
            json!({ "text": error.to_string(), "action": "edit_config_json", "button": text(SETTINGS_WINDOW_OPEN) })
        }
        None if store.config_unreadable() => {
            json!({ "text": text(SETTINGS_WINDOW_CONFIG_JSON_CANT_BE_READ_SO), "action": "", "button": "" })
        }
        None => Value::Null,
    }
}

/// Pages whose fixed notice does not apply now: a build with an update feed has no "development build" note.
pub(super) fn quiet_notices(cx: &App) -> Vec<&'static str> {
    let updates = cx.global::<Updates>();
    if matches!(updates.phase, UpdatePhase::Idle) && !updates.available() {
        Vec::new()
    } else {
        vec!["Updates"] // text: not shown (a key or page id of the native window, or a log line)
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
    // (id, words): the window sends the id back.
    let button = |id: &str, key| [id.to_string(), text(key).to_string()];
    let buttons: Vec<[String; 2]> = match &phase {
        UpdatePhase::Available(release) if release.is_security_update() => {
            vec![button("install_update", SETTINGS_UPDATES_INSTALL_UPDATE)]
        }
        UpdatePhase::Available(_) => vec![
            button("skip_this_version", SETTINGS_UPDATES_SKIP_THIS_VERSION),
            button("install_update", SETTINGS_UPDATES_INSTALL_UPDATE),
        ],
        UpdatePhase::Installed(_) if pending => vec![
            button("dont_restart", SETTINGS_UPDATES_DONT_RESTART),
            button("restart_now", SETTINGS_UPDATES_RESTART_NOW),
        ],
        UpdatePhase::Installed(_) => vec![button("restart_now", SETTINGS_UPDATES_RESTART_NOW)],
        UpdatePhase::Checking | UpdatePhase::Downloading => Vec::new(),
        _ => vec![button("check_now", SETTINGS_UPDATES_CHECK_NOW)],
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

/// The default shell's key (its id in the window), and the shells to choose from as (key, name).
pub(super) fn shell(s: &Settings) -> (String, Vec<(String, String)>) {
    (shells::default_key(s).to_string(), shell_choices(s))
}

/// Make the shell with this key the default. Whether there is one.
pub(super) fn set_shell(s: &mut Settings, id: &str) -> bool {
    match shell_choices(s).into_iter().find(|(key, _)| key == id) {
        Some((key, _)) => {
            s.default_shell = key;
            true
        }
        None => false,
    }
}

/// A button of the Updates, Terminal or Advanced pages. Whether it was one of these.
pub(super) fn perform(key: &str, id: &str, cx: &mut App) -> bool {
    let is = |wanted: &str| id == wanted;
    match key {
        UPDATES_PAGE => {
            if is("check_now") {
                if !cx.global::<Updates>().busy() {
                    update::check_for_updates(true, cx);
                }
            } else if is("install_update") || is("skip_this_version") {
                if let UpdatePhase::Available(release) = cx.global::<Updates>().phase.clone() {
                    if is("install_update") {
                        update::install_update(*release, cx);
                    } else if !release.is_security_update() {
                        update::skip_update(&release, cx);
                    }
                }
            } else if is("dont_restart") {
                update::cancel_restart_when_idle(cx);
            } else if is("restart_now") {
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
        CLI_TOOL_ROW => {
            cli_tool_press();
            true
        }
        DATA_FOLDER_ROW => {
            crate::settings_window::native::paths::open_folder(&data_dir(), cx);
            true
        }
        LOGS_ROW => {
            crate::settings_window::native::paths::open_folder(&data_dir().join("logs"), cx);
            true
        }
        RESET_TIPS_ROW => {
            app_mut(cx).reset_tips();
            cx.refresh_windows();
            true
        }
        DEV_TOOLS_ROW => {
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
