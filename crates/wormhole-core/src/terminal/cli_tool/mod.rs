//! The command line tool: `wh` (`ipc::control::COMMAND_NAME`), a short script that runs this app's
//! binary, so `wh list`, `wh notify …` and the rest of `docs/cli.md` have a name people can type.
//!
//! It is kept in two places:
//!
//! - **Wormhole's own terminals.** A copy in `<data dir>/bin`, rewritten at every launch whenever
//!   its content differs ([`keep_private_current`]), so it points at the binary that runs now
//!   after an update or a move; the folder goes first on the `PATH` of each terminal the app starts
//!   ([`terminal_dir`], [`path_with_first`]). Nobody has to do anything, and nothing outside the
//!   app's data folder is touched.
//! - **Any terminal, on request.** Settings → Command line tool writes it into `/usr/local/bin`, or
//!   `~/.local/bin` when that isn't writable ([`install`]) and takes it out again ([`uninstall`]),
//!   and says whether the folder is on the login shell's `PATH` ([`path`]). Only the button does
//!   this; a launch never does.
//!
//! It is a script and not a symlink to the binary for two reasons: it carries [`MARKER`], the one
//! line by which Wormhole tells its own file from somebody else's `wh` (it replaces and removes
//! only its own), and `wh` alone prints the usage, where the binary alone would start the app.

mod path;
#[cfg(test)]
mod tests;

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::ipc::control::COMMAND_NAME;
use crate::platform::shell;
use crate::storage::files::{self, LinkPolicy};
pub use path::{Advice, advice, login_path, login_shell_path, on_path, path_with_first};

/// The line that makes a file this module's. Not derived from the command's name: a file that
/// carried it under an earlier name is still ours to update and remove.
pub const MARKER: &str = "# wormhole:command-line-tool";

/// `WORMHOLE_CLI_DIR`: the one folder Install, Uninstall and the state look at, in place of
/// `/usr/local/bin` and `~/.local/bin`. For devctl instances and tests, which must never write
/// into the real ones (the way `WORMHOLE_AGENT_HOME` stands in for the agents' folders); it is
/// one of the variables a terminal's shell does not inherit.
pub const DIR_VAR: &str = "WORMHOLE_CLI_DIR";

/// `WORMHOLE_CLI_LOGIN_PATH`: what an instance with [`DIR_VAR`] set answers when Settings asks for
/// the login shell's `PATH` (it never starts the real shell then), so the "on your PATH" and "not
/// on your PATH" states can both be looked at.
pub const LOGIN_PATH_VAR: &str = "WORMHOLE_CLI_LOGIN_PATH";

/// More than a script this module writes is ever: anything longer at the target isn't ours.
const READ_CAP: u64 = 16 * 1024;

/// The script, running `binary` with the arguments it is given. `None` for a path a script can't
/// carry: one that isn't absolute (the script runs from anywhere), isn't UTF-8 or has a line break.
pub fn shim(binary: &Path) -> Option<String> {
    let binary = binary.to_str().filter(|path| binary.is_absolute() && !path.contains(['\n', '\r', '\0']))?;
    let name = COMMAND_NAME;
    Some(format!(
        "#!/bin/sh\n\
         {MARKER}\n\
         # Written by Wormhole. Settings > Coding agents > Command line tool updates and removes it;\n\
         # a copy edited by hand loses the line above and is left alone.\n\
         bin={quoted}\n\
         [ -x \"$bin\" ] || {{ echo \"{name}: Wormhole is no longer at $bin; open Wormhole and update \
         the command line tool in Settings > Coding agents.\" >&2; exit 127; }}\n\
         [ \"$#\" -gt 0 ] || set -- help\n\
         exec \"$bin\" \"$@\"\n",
        quoted = shell::single_quote(binary),
    ))
}

/// Whether `text` is a script of this module's (or one an earlier build wrote).
pub fn is_ours(text: &str) -> bool {
    text.lines().take(8).any(|line| line == MARKER)
}

/// What stands where the script goes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Slot {
    Missing,
    /// Ours; `current` when it is the script this build would write.
    Ours {
        current: bool,
    },
    /// Anything else: somebody's own `wh`, a directory, a file that can't be read.
    Foreign,
}

fn inspect(path: &Path, wanted: Option<&str>) -> Slot {
    match files::read_to_string_capped(path, READ_CAP, LinkPolicy::Follow) {
        Ok(text) if is_ours(&text) => Slot::Ours { current: wanted == Some(text.as_str()) },
        // Nothing there to see: no file, or no folder to have one in.
        Err(_) if path.symlink_metadata().is_err() => Slot::Missing,
        // Anything else that is there (a file that can't be read, a directory, a dangling link)
        // isn't known to be ours, so it is left alone.
        Ok(_) | Err(_) => Slot::Foreign,
    }
}

/// Why the script couldn't be put in place.
#[derive(Debug)]
pub enum Error {
    /// Another `wh` stands there; it is left as it is.
    Foreign(PathBuf),
    /// The binary's path can't be written into a script ([`shim`]).
    Unusable,
    /// No folder to put it in (no home folder).
    NoFolder,
    Io {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Foreign(path) => {
                write!(f, "another {COMMAND_NAME} is at {}; Wormhole leaves it alone", path.display())
            }
            Error::Unusable => write!(f, "this copy of Wormhole is somewhere a script can't name"),
            Error::NoFolder => write!(f, "there is no folder to put {COMMAND_NAME} in"),
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

/// `dir`'s script.
pub fn script_path(dir: &Path) -> PathBuf {
    dir.join(COMMAND_NAME)
}

/// Write `script` at `path` unless it is there already; refuses to replace what isn't ours. Returns
/// whether the file changed.
fn put(path: &Path, script: &str) -> Result<bool, Error> {
    match inspect(path, Some(script)) {
        Slot::Foreign => Err(Error::Foreign(path.to_path_buf())),
        Slot::Ours { current: true } => Ok(false),
        Slot::Ours { current: false } | Slot::Missing => {
            files::write_atomic_executable(path, script.as_bytes(), LinkPolicy::Follow)
                .map_err(|source| Error::Io { path: path.to_path_buf(), source })?;
            Ok(true)
        }
    }
}

/// The folder Wormhole's own terminals get: `<data dir>/bin`.
pub fn private_dir() -> PathBuf {
    crate::storage::paths::data_dir().join("bin")
}

/// Make the copy in [`private_dir`] the script for `binary` (the running app), rewriting it only
/// when it differs. Returns whether it was written. An error is for the log: a terminal starts
/// either way, without the command.
pub fn keep_private_current(binary: &Path) -> Result<bool, Error> {
    keep_current_in(&private_dir(), binary)
}

pub(crate) fn keep_current_in(dir: &Path, binary: &Path) -> Result<bool, Error> {
    put(&script_path(dir), &shim(binary).ok_or(Error::Unusable)?)
}

/// What the app does at launch: [`keep_private_current`] for the binary that runs now, saying in
/// the log what it did or couldn't. Never fails the launch; the terminals just start without `wh`.
pub fn keep_current_at_launch() {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            tracing::warn!(
                "the {COMMAND_NAME} command isn't set up: where this app runs from is unknown: {err}"
            );
            return;
        }
    };
    match keep_private_current(&exe) {
        Ok(true) => tracing::info!(dir = %private_dir().display(), "wrote the {COMMAND_NAME} command"),
        Ok(false) => {}
        Err(err) => tracing::warn!("the {COMMAND_NAME} command isn't set up for the terminals: {err}"),
    }
}

/// The folder to put first on a terminal's `PATH`: [`private_dir`] while the script is in it.
pub fn terminal_dir() -> Option<PathBuf> {
    terminal_dir_in(private_dir())
}

fn terminal_dir_in(dir: PathBuf) -> Option<PathBuf> {
    std::fs::metadata(script_path(&dir)).ok().filter(|meta| meta.is_file()).map(|_| dir)
}

/// The folders Install, Uninstall and the state look at, in the order Install prefers them.
#[derive(Debug, Clone)]
pub struct Targets {
    dirs: Vec<Folder>,
}

#[derive(Debug, Clone)]
struct Folder {
    path: PathBuf,
    /// Install makes it when it is missing (`~/.local/bin`; `/usr/local/bin` is the system's to make).
    create: bool,
}

impl Targets {
    /// `/usr/local/bin` where it exists and can be written, else `~/.local/bin`.
    pub fn system(home: Option<&Path>) -> Self {
        let mut dirs = vec![Folder { path: PathBuf::from("/usr/local/bin"), create: false }];
        dirs.extend(home.map(|home| Folder { path: home.join(".local/bin"), create: true }));
        Self { dirs }
    }

    /// Only `dir`.
    pub fn only(dir: PathBuf) -> Self {
        Self { dirs: vec![Folder { path: dir, create: true }] }
    }

    /// [`DIR_VAR`]'s folder when it is set, the system's otherwise.
    pub fn from_env() -> Self {
        Self::resolve(std::env::var_os(DIR_VAR), dirs::home_dir().as_deref())
    }

    fn resolve(over: Option<std::ffi::OsString>, home: Option<&Path>) -> Self {
        match over.filter(|dir| !dir.is_empty()) {
            Some(dir) => Self::only(PathBuf::from(dir)),
            None => Self::system(home),
        }
    }

    /// Whether this is the stand-in for a test or devctl instance, not the person's own folders.
    pub fn is_override() -> bool {
        std::env::var_os(DIR_VAR).is_some_and(|dir| !dir.is_empty())
    }

    fn slots(&self, wanted: Option<&str>) -> Vec<(PathBuf, Slot)> {
        self.dirs
            .iter()
            .map(|folder| {
                let path = script_path(&folder.path);
                let slot = inspect(&path, wanted);
                (path, slot)
            })
            .collect()
    }
}

/// What Settings shows of the command line tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    NotInstalled,
    /// Our script is at `path`; `current` when it runs the binary that runs now.
    Installed {
        path: PathBuf,
        current: bool,
    },
    /// Another `wh` is at `path`.
    Foreign {
        path: PathBuf,
    },
}

/// Where the command line tool stands, for the running app at `binary`. Reads, never writes.
pub fn status(targets: &Targets, binary: &Path) -> Status {
    let slots = targets.slots(shim(binary).as_deref());
    let ours = slots.iter().find_map(|(path, slot)| match slot {
        Slot::Ours { current } => Some(Status::Installed { path: path.clone(), current: *current }),
        _ => None,
    });
    let foreign = slots
        .iter()
        .find(|(_, slot)| *slot == Slot::Foreign)
        .map(|(path, _)| Status::Foreign { path: path.clone() });
    ours.or(foreign).unwrap_or(Status::NotInstalled)
}

/// Where Install put the script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub path: PathBuf,
    /// False when the script was there and right already.
    pub changed: bool,
}

/// Put the script for `binary` in the first of `targets` that takes it; a script of ours that is
/// already in one is updated where it stands. Another `wh` in any of them stops it: it would be
/// in the way of ours or hidden by it, and is not Wormhole's to replace.
pub fn install(targets: &Targets, binary: &Path) -> Result<Installed, Error> {
    let script = shim(binary).ok_or(Error::Unusable)?;
    let slots = targets.slots(Some(&script));
    if let Some((path, _)) = slots.iter().find(|(_, slot)| *slot == Slot::Foreign) {
        return Err(Error::Foreign(path.clone()));
    }
    if let Some((path, _)) = slots.iter().find(|(_, slot)| matches!(slot, Slot::Ours { .. })) {
        return Ok(Installed { path: path.clone(), changed: put(path, &script)? });
    }
    let mut refused = None;
    for folder in &targets.dirs {
        if !folder.create && !folder.path.is_dir() {
            continue;
        }
        let path = script_path(&folder.path);
        match put(&path, &script) {
            Ok(changed) => return Ok(Installed { path, changed }),
            // `/usr/local/bin` that this user can't write: the next folder is for that.
            Err(Error::Io { source, .. }) if source.kind() == io::ErrorKind::PermissionDenied => {
                refused = Some(Error::Io { path, source });
            }
            Err(err) => return Err(err),
        }
    }
    Err(refused.unwrap_or(Error::NoFolder))
}

/// Remove the scripts of ours from `targets`; a `wh` that isn't ours stays. Returns what it removed.
pub fn uninstall(targets: &Targets) -> Result<Vec<PathBuf>, Error> {
    let mut removed = Vec::new();
    for (path, slot) in targets.slots(None) {
        if matches!(slot, Slot::Ours { .. }) {
            std::fs::remove_file(&path).map_err(|source| Error::Io { path: path.clone(), source })?;
            removed.push(path);
        }
    }
    Ok(removed)
}
