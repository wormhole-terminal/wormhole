//! The system's answers to [`Processes`], for the walk that finds who is behind a `wh` (the GPUI app's
//! `agent_access/processes.rs`, which lives in a crate this one cannot link: the same reads, the same cache).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use wormhole_core::ipc::access::{Processes, ScriptPin};
use wormhole_core::platform::codesign::{self, Signature};
use wormhole_core::platform::process;

/// How many signatures are kept: an agent's calls come in bursts, and its signature does not change while
/// it runs. Keyed by pid and start time, so a pid that came back is read afresh.
const CACHE_MAX: usize = 32;

/// How far up a script's folders the nearest `package.json` is looked for.
const PACKAGE_LEVELS: usize = 8;

/// What was read of programs, kept between requests.
#[derive(Default)]
pub(crate) struct Signatures {
    known: HashMap<(u32, SystemTime), Signature>,
}

/// The real process table, with the shells of Wormhole's terminals.
pub(crate) struct System {
    signatures: RefCell<Signatures>,
    shells: Vec<u32>,
}

impl System {
    pub(crate) fn new(signatures: Signatures, shells: Vec<u32>) -> Self {
        Self { signatures: RefCell::new(signatures), shells }
    }

    /// The cache, to keep for the next request.
    pub(crate) fn into_signatures(self) -> Signatures {
        self.signatures.into_inner()
    }
}

impl Processes for System {
    fn executable(&self, pid: u32) -> Option<PathBuf> {
        process::executable(pid)
    }

    fn command_line(&self, pid: u32) -> Option<(String, Vec<String>)> {
        process::command_line(pid)
    }

    fn start_time(&self, pid: u32) -> Option<SystemTime> {
        process::start_time(pid)
    }

    fn signature(&self, pid: u32) -> Signature {
        let Some(started) = process::start_time(pid) else { return codesign::of_pid(pid) };
        let mut cache = self.signatures.borrow_mut();
        if let Some(known) = cache.known.get(&(pid, started)) {
            return known.clone();
        }
        let read = codesign::of_pid(pid);
        // A process that ended between the two reads is not worth remembering.
        if process::start_time(pid) == Some(started) {
            if cache.known.len() >= CACHE_MAX {
                cache.known.clear();
            }
            cache.known.insert((pid, started), read.clone());
        }
        read
    }

    fn is_terminal_shell(&self, pid: u32) -> bool {
        self.shells.contains(&pid)
    }

    fn env_runs_code(&self, pid: u32) -> bool {
        // An environment that cannot be read is not given the benefit of the doubt.
        process::environment_names(pid).is_none_or(|names| {
            names.iter().any(|name| wormhole_core::agents::CODE_LOADING_VARIABLES.contains(&name.as_str()))
        })
    }

    fn script_pin(&self, pid: u32, script: &str) -> Option<ScriptPin> {
        let path = Path::new(script);
        // Relative to the folder the interpreter was started in, which is where it looked for it.
        let path = if path.is_absolute() { path.to_path_buf() } else { process::cwd(pid)?.join(path) };
        let file = path.canonicalize().ok()?;
        if !file.metadata().ok()?.is_file() {
            return None;
        }
        // The package the file is in, or the file itself: never a folder the file is merely named in.
        let package = file
            .ancestors()
            .skip(1)
            .take(PACKAGE_LEVELS)
            .find(|folder| folder.join("package.json").is_file());
        let pin = package.map_or_else(|| file.clone(), Path::to_path_buf);
        file.starts_with(&pin).then_some(ScriptPin { file, pin })
    }

    fn stamp(&self, path: &Path) -> Option<(u64, i64)> {
        let meta = std::fs::metadata(path).ok()?;
        let modified = meta.modified().ok()?.duration_since(SystemTime::UNIX_EPOCH).ok()?;
        Some((meta.len(), i64::try_from(modified.as_secs()).unwrap_or(i64::MAX)))
    }
}
