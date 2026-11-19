//! Opening one file in the app the person chose, at a line where that app can (`wormhole_core::platform::open_file`): the core
//! decides which mechanism the app takes, this turns it into the process the GPUI app starts (`platform::apps::command`: `open`
//! aimed at the app, or the command line the app brings) and starts it. Swift only says where the app is (Launch Services
//! knows) and never spawns anything itself.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use wormhole_core::platform::open_file::{self, Launch};
use wormhole_core::storage::files;
use wormhole_term::sys::process;

/// The program and arguments for `launch` aimed at the app `bundle`, which lives in `app` when it is installed.
fn command(bundle: &str, launch: Launch, app: Option<&Path>) -> (OsString, Vec<OsString>) {
    let open = || OsString::from("/usr/bin/open");
    let strings = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
    let in_app = |what: OsString| {
        let mut args = strings(&["-b", bundle]);
        args.push(what);
        (open(), args)
    };
    match launch {
        Launch::Open { path } => in_app(path.into_os_string()),
        Launch::Url { url } => in_app(url.into()),
        // Finder as the chosen app: `open -R`, like every other launch a process to start.
        Launch::Reveal { path } => (open(), vec!["-R".into(), path.into_os_string()]),
        Launch::Args { args } => {
            let mut all = strings(&["-n", "-b", bundle, "--args"]);
            all.extend(args.into_iter().map(OsString::from));
            (open(), all)
        }
        Launch::Bundled { relative, arg } => {
            match app.map(|app| app.join(relative)).filter(|program| program.is_file()) {
                Some(program) => (program.into_os_string(), vec![arg.into()]),
                // An older version of the app without it: the file, without the line.
                None => in_app(arg.rsplit_once(':').map_or(arg.as_str(), |(path, _)| path).into()),
            }
        }
    }
}

/// Open `path` (absolute, already found inside the repository; a key of `files_panel::names` where it is not text) in the app with this bundle identifier at `line`, as the GPUI
/// app does; `app_folder` is where Launch Services says the app is. Whether a process was started; nobody waits for it.
#[uniffi::export]
pub fn open_file_run(bundle: String, path: String, line: Option<u32>, app_folder: Option<String>) -> bool {
    let Some(path) = crate::files_panel::names::path(&path) else { return false };
    run(&bundle, &path, line, app_folder.as_deref().map(Path::new))
}

/// Where a test instance's launches are written instead of started (`DesignSystem.ExternalOpen` writes its own there too).
static RECORD: Mutex<Option<PathBuf>> = Mutex::new(None);

/// From now on a launch is a line appended to `file` (`run <program> <arguments>`) and nothing is started; `None` starts
/// processes again. A `--background` instance never opens the person's editor or browser.
#[uniffi::export]
pub fn open_file_record_to(file: Option<String>) {
    *RECORD.lock().unwrap_or_else(PoisonError::into_inner) = file.map(PathBuf::from);
}

/// The tooltip and menu words of the open action: "Open in Zed at line 12", "Reveal in Finder".
#[uniffi::export]
pub fn open_file_tooltip(bundle: String, app: String, line: Option<u32>) -> String {
    open_file::tooltip(&bundle, &app, line)
}

/// Where `relative` is inside the repository at `root`, every link resolved: nil for a file that is gone, a path that
/// leads out of the repository (a planted link) or one that is not a file. Nothing is opened or revealed that this refuses.
/// `root` and the answer are keys of `files_panel::names` where a name is not text, so a repository in such a folder opens,
/// reveals and copies its files (`open_file_run`, `DiskName.url`), and a link resolved to such a name is never cut to text.
#[uniffi::export]
pub fn repo_file_resolve(root: String, relative: String) -> Option<String> {
    use crate::files_panel::names;
    let real = files::resolve_within(&names::path(&root)?, &relative).ok()?;
    Some(names::path_key(&real))
}

fn run(bundle: &str, path: &Path, line: Option<u32>, app: Option<&Path>) -> bool {
    let Some(launch) = open_file::launch(bundle, path, line) else { return false };
    let (program, args) = command(bundle, launch, app);
    if let Some(file) = RECORD.lock().unwrap_or_else(PoisonError::into_inner).as_ref() {
        let line =
            std::iter::once(&program).chain(&args).map(|word| word.to_string_lossy()).collect::<Vec<_>>();
        let appended = std::fs::OpenOptions::new().create(true).append(true).open(file);
        return appended.and_then(|mut out| writeln!(out, "run {}", line.join(" "))).is_ok();
    }
    let Ok(mut child) = process::command(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    // Reaped by a thread of its own, so the child never lingers as a zombie.
    std::thread::spawn(move || drop(child.wait()));
    true
}

#[cfg(test)]
mod tests {
    use std::fs;

    use wormhole_core::storage::files::{LinkPolicy, write_atomic_executable};

    use super::*;

    fn words(bundle: &str, path: &str, line: Option<u32>, app: Option<&Path>) -> Vec<String> {
        let launch = open_file::launch(bundle, Path::new(path), line).expect("absolute");
        let (program, args) = command(bundle, launch, app);
        std::iter::once(program).chain(args).map(|word| word.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn each_app_is_asked_the_way_the_core_says() {
        let file = "/repo/a b.rs";
        assert_eq!(
            words("com.microsoft.VSCode", file, Some(3), None),
            ["/usr/bin/open", "-b", "com.microsoft.VSCode", "vscode://file/repo/a%20b.rs:3"]
        );
        assert_eq!(
            words("com.jetbrains.rustrover", file, Some(3), None),
            ["/usr/bin/open", "-n", "-b", "com.jetbrains.rustrover", "--args", "--line", "3", file]
        );
        assert_eq!(
            words("com.apple.TextEdit", file, Some(3), None),
            ["/usr/bin/open", "-b", "com.apple.TextEdit", file],
            "an app with no way to take a line opens the file"
        );
        assert_eq!(words("com.apple.finder", file, None, None), ["/usr/bin/open", "-R", file]);
        // Zed's command line is in its bundle; a version without it opens the file, without the line.
        assert_eq!(words("dev.zed.Zed", file, Some(3), None), ["/usr/bin/open", "-b", "dev.zed.Zed", file]);
    }

    #[test]
    fn the_apps_own_command_line_really_runs_with_the_line() {
        let dir = tempfile::tempdir().expect("scratch folder");
        let app = dir.path().join("Zed.app");
        let program = app.join("Contents/MacOS/cli");
        fs::create_dir_all(program.parent().expect("folder")).expect("bundle");
        let out = dir.path().join("argument");
        let script = format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", out.display());
        write_atomic_executable(&program, script.as_bytes(), LinkPolicy::Follow).expect("script");
        assert_eq!(
            words("dev.zed.Zed", "/repo/a.rs", Some(3), Some(&app)),
            [program.to_str().expect("utf-8"), "/repo/a.rs:3"]
        );
        assert!(run("dev.zed.Zed", Path::new("/repo/a.rs"), Some(3), Some(&app)));
        let mut written = String::new();
        for _ in 0..500 {
            written = fs::read_to_string(&out).unwrap_or_default();
            if !written.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(written, "/repo/a.rs:3");
        assert!(
            !run("dev.zed.Zed", Path::new("rel.rs"), None, Some(&app)),
            "a relative path is never opened"
        );

        // A recorded launch is written and never started (the global is the reason this is one test, not two).
        let log = dir.path().join("launches.txt");
        open_file_record_to(Some(log.to_string_lossy().into_owned()));
        let started = run("com.microsoft.VSCode", Path::new("/repo/a.rs"), Some(7), None);
        open_file_record_to(None);
        assert!(started);
        assert_eq!(
            fs::read_to_string(&log).expect("log"),
            "run /usr/bin/open -b com.microsoft.VSCode vscode://file/repo/a.rs:7\n"
        );
    }

    /// The root comes as a key and the file goes back as one (a name that is text is its own key); a key that does not
    /// read back names no repository, and a path out of it is refused.
    #[test]
    fn a_file_is_resolved_from_the_roots_key_and_a_broken_key_resolves_nothing() {
        use crate::files_panel::names;
        let repo = tempfile::tempdir().expect("repository");
        let root = repo.path().canonicalize().expect("canonical");
        write_atomic_executable(&root.join("a.sh"), b"#!/bin/sh\n", LinkPolicy::Follow).expect("file");
        let resolved = repo_file_resolve(names::path_key(&root), "a.sh".into());
        assert_eq!(resolved.as_deref().and_then(names::path), Some(root.join("a.sh")));
        assert_eq!(repo_file_resolve("/tmp/caf\0zz".into(), "a.sh".into()), None, "a key cut short");
        assert_eq!(
            repo_file_resolve(names::path_key(&root), "../a.sh".into()),
            None,
            "out of the repository"
        );
    }
}

/// Whether Wormhole knows the app as an editor (or Finder): the Files tab offers a file only these
/// (`features/files/open.rs` `may_open_in`).
#[uniffi::export]
pub fn open_in_known(bundle: String) -> bool {
    wormhole_core::platform::open_in::known(&bundle).is_some()
}
