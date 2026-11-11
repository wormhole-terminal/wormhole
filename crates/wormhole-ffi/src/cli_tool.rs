//! The `wh` command line tool as Settings and the launch handle it (`wormhole_core::terminal::cli_tool`): where
//! it stands, Install and Uninstall, the copy every terminal of the app finds first on its `PATH`, and whether
//! the folder Install used is on a new terminal's `PATH`. The rules are core's; the sentences are the
//! catalogs'.
//!
//! The folders Install writes in are core's `Targets::from_env`: `/usr/local/bin` or `~/.local/bin`, or
//! `WORMHOLE_CLI_DIR` alone when it is set. An experiment instance sets it to a folder of its own in its data
//! folder (the Swift app does), so nothing here writes outside the instance unless the caller says where.

use std::path::{Path, PathBuf};

use wormhole_core::ipc::control::COMMAND_NAME;
use wormhole_core::platform::paths;
use wormhole_core::terminal::cli_tool::{self, Status, Targets};
use wormhole_text::keys::{
    settings_agents_cli_foreign_at, settings_agents_cli_foreign_note, settings_agents_cli_install_failed,
    settings_agents_cli_installed_at, settings_agents_cli_not_installed, settings_agents_cli_path_off,
    settings_agents_cli_path_on, settings_agents_cli_path_unknown, settings_agents_cli_remove_failed,
    settings_agents_cli_stale_note,
};

/// Where the tool stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CliToolState {
    NotInstalled,
    /// Our script is in a folder.
    Installed,
    /// Somebody else's `wh` is in the way: left alone.
    Foreign,
}

/// What the Command line tool row shows.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CliToolRecord {
    pub state: CliToolState,
    /// The command's name (`wh`), for the button's tooltip and the row's words.
    pub command: String,
    /// Where the script is (installed or foreign), as the person would say it (`~/.local/bin/wh`).
    pub shown_path: Option<String>,
    /// The folder of the script, for [`cli_tool_path_note`].
    pub folder: Option<String>,
    /// The row's line: what it is for and where it stands.
    pub headline: String,
    /// What needs saying under the row: whose `wh` is in the way, or that the installed copy runs another
    /// Wormhole (`Update` is then offered).
    pub note: Option<String>,
    /// The installed copy runs the binary that runs now.
    pub current: bool,
}

/// Whether the folder the script is in is on the `PATH` a new terminal's shell gets.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CliToolPathRecord {
    /// `on`, `off` (with `advice_file` and `advice_line`) or `unknown`.
    pub verdict: CliToolPathVerdict,
    pub text: String,
    pub advice_file: Option<String>,
    /// The line to paste into the startup file.
    pub advice_line: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CliToolPathVerdict {
    On,
    Off,
    Unknown,
}

/// Why Install, Uninstall or keeping the copy current did not work, in the catalogs' words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum CliToolError {
    /// Another `wh` stands there: the row already says so, there is nothing to add.
    #[error("{message}")]
    Foreign { message: String },
    #[error("{message}")]
    Failed { message: String },
}

fn shown(path: &Path) -> String {
    wormhole_core::view::sidebar::display_path(path, paths::home().as_deref())
}

/// `binary` as core wants it: the running executable (the script runs it).
fn binary(path: &str) -> PathBuf {
    PathBuf::from(path)
}

/// Where the tool stands for the app at `binary`. Reads, never writes.
#[uniffi::export]
pub fn cli_tool_status(binary_path: String) -> CliToolRecord {
    status_in(&Targets::from_env(), &binary_path)
}

fn status_in(targets: &Targets, binary_path: &str) -> CliToolRecord {
    let status = cli_tool::status(targets, &binary(binary_path));
    let command = COMMAND_NAME.to_string();
    match status {
        Status::NotInstalled => CliToolRecord {
            state: CliToolState::NotInstalled,
            headline: settings_agents_cli_not_installed(COMMAND_NAME),
            command,
            shown_path: None,
            folder: None,
            note: None,
            current: false,
        },
        Status::Installed { path, current } => CliToolRecord {
            state: CliToolState::Installed,
            headline: settings_agents_cli_installed_at(COMMAND_NAME, &shown(&path)),
            command,
            shown_path: Some(shown(&path)),
            folder: path.parent().map(|dir| dir.to_string_lossy().into_owned()),
            note: (!current).then(|| settings_agents_cli_stale_note(COMMAND_NAME)),
            current,
        },
        Status::Foreign { path } => CliToolRecord {
            state: CliToolState::Foreign,
            headline: settings_agents_cli_foreign_at(COMMAND_NAME, &shown(&path)),
            command,
            shown_path: Some(shown(&path)),
            folder: path.parent().map(|dir| dir.to_string_lossy().into_owned()),
            note: Some(settings_agents_cli_foreign_note(COMMAND_NAME)),
            current: false,
        },
    }
}

/// Install (or update) the script for the app at `binary_path`. Returns where it went.
#[uniffi::export]
pub fn cli_tool_install(binary_path: String) -> Result<String, CliToolError> {
    install_in(&Targets::from_env(), &binary_path)
}

fn install_in(targets: &Targets, binary_path: &str) -> Result<String, CliToolError> {
    match cli_tool::install(targets, &binary(binary_path)) {
        Ok(done) => {
            tracing::info!(path = %done.path.display(), changed = done.changed, "installed the command line tool");
            Ok(done.path.to_string_lossy().into_owned())
        }
        Err(cli_tool::Error::Foreign(path)) => Err(CliToolError::Foreign {
            message: settings_agents_cli_foreign_at(COMMAND_NAME, &shown(&path)),
        }),
        Err(err) => {
            tracing::warn!("installing the command line tool: {err}");
            Err(CliToolError::Failed {
                message: settings_agents_cli_install_failed(COMMAND_NAME, &err.to_string()),
            })
        }
    }
}

/// Remove the scripts of ours. How many went.
#[uniffi::export]
pub fn cli_tool_uninstall() -> Result<u32, CliToolError> {
    uninstall_in(&Targets::from_env())
}

fn uninstall_in(targets: &Targets) -> Result<u32, CliToolError> {
    match cli_tool::uninstall(targets) {
        Ok(removed) => {
            tracing::info!(removed = removed.len(), "removed the command line tool");
            Ok(u32::try_from(removed.len()).unwrap_or(u32::MAX))
        }
        Err(err) => {
            tracing::warn!("removing the command line tool: {err}");
            Err(CliToolError::Failed {
                message: settings_agents_cli_remove_failed(COMMAND_NAME, &err.to_string()),
            })
        }
    }
}

/// Whether `folder` (the script's) is on the `PATH` of the person's login shell, with the line to add when it
/// is not. Starts that shell, which reads the person's startup files: off the main thread.
#[uniffi::export]
pub fn cli_tool_path_note(folder: String) -> CliToolPathRecord {
    let dir = PathBuf::from(&folder);
    let said = shown(&dir);
    let Some(path) = cli_tool::login_shell_path() else {
        return CliToolPathRecord {
            verdict: CliToolPathVerdict::Unknown,
            text: settings_agents_cli_path_unknown(COMMAND_NAME, &said),
            advice_file: None,
            advice_line: None,
        };
    };
    if cli_tool::on_path(&dir, &path) {
        return CliToolPathRecord {
            verdict: CliToolPathVerdict::On,
            text: settings_agents_cli_path_on(&said, COMMAND_NAME),
            advice_file: None,
            advice_line: None,
        };
    }
    let advice =
        cli_tool::advice(&dir, &wormhole_core::terminal::shells::login_shell(), paths::home().as_deref());
    CliToolPathRecord {
        verdict: CliToolPathVerdict::Off,
        text: settings_agents_cli_path_off(&said, COMMAND_NAME, &advice.file),
        advice_file: Some(advice.file),
        advice_line: Some(advice.line),
    }
}

/// At launch: make the copy every terminal of the app finds first on its `PATH` (`<data dir>/bin/wh`) run the
/// binary that runs now. Whether it was written. Never an error for the launch: the caller logs it, and the
/// terminals start without the command.
#[uniffi::export]
pub fn cli_tool_keep_current(binary_path: String) -> Result<bool, CliToolError> {
    crate::isolation::require().map_err(|err| CliToolError::Failed { message: err.to_string() })?;
    cli_tool::keep_private_current(&binary(&binary_path))
        .map_err(|err| CliToolError::Failed { message: err.to_string() })
}

#[cfg(test)]
mod tests {
    use wormhole_core::terminal::cli_tool::{MARKER, script_path};

    use super::*;

    #[test]
    fn install_shows_the_script_and_uninstall_takes_only_ours_away() {
        let dir = tempfile::tempdir().expect("a folder");
        let targets = Targets::only(dir.path().to_path_buf());
        let script = script_path(dir.path());
        let binary = std::env::current_exe().expect("this test's executable").to_string_lossy().into_owned();

        assert_eq!(status_in(&targets, &binary).state, CliToolState::NotInstalled);
        let at = install_in(&targets, &binary).expect("installed");
        assert_eq!(std::path::Path::new(&at), script);
        assert!(std::fs::read_to_string(&script).expect("the script").contains(MARKER));
        let status = status_in(&targets, &binary);
        assert_eq!((status.state, status.current), (CliToolState::Installed, true));
        assert!(status.headline.contains(COMMAND_NAME), "{}", status.headline);
        // Another binary makes the installed copy stale, and Install updates it where it stands.
        let stale = status_in(&targets, "/Applications/Elsewhere.app/Contents/MacOS/Wormhole");
        assert_eq!((stale.state, stale.current), (CliToolState::Installed, false));
        assert!(stale.note.is_some());
        assert_eq!(uninstall_in(&targets).expect("removed"), 1);
        assert!(!script.exists());

        // Somebody's own `wh` is neither replaced nor removed.
        std::fs::write(&script, "#!/bin/sh\necho mine\n").expect("a foreign script");
        assert_eq!(status_in(&targets, &binary).state, CliToolState::Foreign);
        assert!(matches!(install_in(&targets, &binary), Err(CliToolError::Foreign { .. })));
        assert_eq!(uninstall_in(&targets).expect("nothing of ours"), 0);
        assert!(std::fs::read_to_string(&script).expect("still there").contains("mine"));
    }
}
