//! The system's answers to [`Processes`], for the walk that finds who is behind a `wh`.

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
pub(super) struct Signatures {
    known: HashMap<(u32, SystemTime), Signature>,
}

/// The real process table, with the shells of Wormhole's terminals.
pub(super) struct System {
    signatures: RefCell<Signatures>,
    shells: Vec<u32>,
}

impl System {
    pub fn new(signatures: Signatures, shells: Vec<u32>) -> Self {
        Self { signatures: RefCell::new(signatures), shells }
    }

    /// The cache, to keep for the next request.
    pub fn into_signatures(self) -> Signatures {
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use wormhole_core::ipc::access::Processes;

    use super::{Signatures, System};

    /// A scratch folder, removed at the end of the test.
    struct Scratch(std::path::PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("wormhole-agent-access-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("scratch folder");
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_script_is_pinned_by_the_package_it_is_in_or_by_itself() {
        let dir = Scratch::new("package");
        let package = dir.0.join("node_modules/@x/cli");
        std::fs::create_dir_all(package.join("dist")).expect("dirs");
        std::fs::write(package.join("package.json"), "{}").expect("package.json");
        std::fs::write(package.join("dist/cli.js"), "").expect("script");
        let system = System::new(Signatures::default(), vec![]);
        let me = std::process::id();
        let pin =
            system.script_pin(me, &package.join("dist/cli.js").to_string_lossy()).expect("a regular file");
        assert_eq!(pin.pin, package.canonicalize().expect("real"));
        assert_eq!(pin.file, package.join("dist/cli.js").canonicalize().expect("real"));
        // No package.json above: the file itself, not the folder it is in.
        let loose = dir.0.join("tools");
        std::fs::create_dir_all(&loose).expect("dir");
        std::fs::write(loose.join("run.js"), "").expect("script");
        let pin = system.script_pin(me, &loose.join("run.js").to_string_lossy()).expect("a regular file");
        assert_eq!(pin.pin, loose.join("run.js").canonicalize().expect("real"));
        // A folder, a missing file, and a link to a file elsewhere (it is what it points at).
        assert!(system.script_pin(me, &loose.to_string_lossy()).is_none());
        assert!(system.script_pin(me, &loose.join("nothing.js").to_string_lossy()).is_none());
        std::os::unix::fs::symlink(loose.join("run.js"), package.join("dist/link.js")).expect("link");
        let linked =
            system.script_pin(me, &package.join("dist/link.js").to_string_lossy()).expect("resolves");
        assert_eq!(linked.file, loose.join("run.js").canonicalize().expect("real"), "the file it really is");
        assert_eq!(linked.pin, linked.file, "and not the package the link sits in");
    }

    #[test]
    fn an_environment_that_loads_code_is_found_and_one_that_cannot_be_read_counts_as_loading() {
        let system = System::new(Signatures::default(), vec![]);
        let loads = std::env::vars_os()
            .any(|(name, _)| wormhole_core::agents::CODE_LOADING_VARIABLES.iter().any(|v| name == *v));
        assert_eq!(system.env_runs_code(std::process::id()), loads);
        assert!(system.env_runs_code(u32::MAX), "unreadable: not trusted");
    }

    #[test]
    fn this_process_is_read_once_and_the_shells_are_the_ones_given() {
        let system = System::new(Signatures::default(), vec![42]);
        assert!(system.is_terminal_shell(42) && !system.is_terminal_shell(43));
        let me = std::process::id();
        let first = system.signature(me);
        assert_eq!(system.signature(me), first, "from the cache the second time");
        assert_eq!(system.signatures.borrow().known.len(), 1);
        assert!(system.stamp(Path::new("/definitely/not/here")).is_none());
        assert!(system.executable(me).is_some());
    }
}
