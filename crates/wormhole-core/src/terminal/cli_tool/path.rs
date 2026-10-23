//! `PATH`: the folder put first on a terminal's, and whether the folder Install used is on the
//! one a new terminal gets, with the line to add when it isn't.

use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use super::{LOGIN_PATH_VAR, Targets};
use crate::terminal::shells;

/// `PATH` for a terminal whose environment says `base`, with `dir` first. Unchanged when `dir`
/// can't be one entry of it (it has a `:` in it), and never ending in a bare `:`, which a shell
/// reads as "also the current folder".
pub fn path_with_first(dir: &Path, base: &str) -> String {
    let Some(dir_text) = dir.to_str().filter(|text| !text.is_empty() && !text.contains(':')) else {
        return base.to_string();
    };
    let entries: Vec<&str> = base.split(':').filter(|entry| !entry.is_empty()).collect();
    // Already first (a Wormhole started from one of its own terminals): nothing to add.
    if entries.first().is_some_and(|first| Path::new(first) == dir) {
        return base.to_string();
    }
    std::iter::once(dir_text).chain(entries).collect::<Vec<_>>().join(":")
}

/// Whether `dir` is one of the entries of `path` (a `PATH` value), compared as folders: a trailing
/// slash doesn't matter, nor does a link on the way.
pub fn on_path(dir: &Path, path: &str) -> bool {
    let real = std::fs::canonicalize(dir).ok();
    path.split(':').filter(|entry| !entry.is_empty()).map(Path::new).any(|entry| {
        entry == dir
            || real.as_deref().is_some_and(|real| std::fs::canonicalize(entry).is_ok_and(|e| e == real))
    })
}

/// What to add to a shell's startup file so that `dir` is on its `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    /// The file, as the person would say it (`~/.zshrc`).
    pub file: String,
    /// The line, ready to paste.
    pub line: String,
}

/// The line that puts `dir` on the `PATH` of `shell` (a path, `$SHELL`), and the file it goes in.
pub fn advice(dir: &Path, shell: &str, home: Option<&Path>) -> Advice {
    // `$HOME/…` so the line stays right if the account is renamed or the file is copied.
    let shown = match home.and_then(|home| dir.strip_prefix(home).ok()) {
        Some(rest) => format!("$HOME/{}", rest.display()),
        None => dir.display().to_string(),
    };
    let export = format!("export PATH=\"{shown}:$PATH\"");
    let (file, line) = match shells::program_name(shell) {
        "fish" => ("~/.config/fish/config.fish", format!("fish_add_path \"{shown}\"")),
        // A login shell on macOS reads `.bash_profile`, which is what a terminal's shell is.
        "bash" => ("~/.bash_profile", export),
        "zsh" => ("~/.zshrc", export),
        _ => ("your shell's startup file", export),
    };
    Advice { file: file.to_string(), line }
}

/// The `PATH` a new terminal's shell would have, or `None` when it can't be found out. A test or
/// devctl instance (`DIR_VAR` set) never starts the person's shell, which would read their startup
/// files: its stand-in is `LOGIN_PATH_VAR`, else its own `PATH`.
pub fn login_shell_path() -> Option<String> {
    if Targets::is_override() {
        return std::env::var(LOGIN_PATH_VAR).ok().or_else(|| std::env::var("PATH").ok());
    }
    login_path(&shells::login_shell(), LOGIN_WAIT)
}

/// How long a login shell may take to say its `PATH`: its startup files can be slow, and a stuck
/// one must not hold Settings' answer back for ever.
const LOGIN_WAIT: Duration = Duration::from_secs(5);

/// Words around the `PATH` in the shell's output, which a startup file may also print into.
const BEGIN: &str = "WORMHOLE_LOGIN_PATH_BEGIN:";
const END: &str = ":WORMHOLE_LOGIN_PATH_END";

/// What `shell`, started as a terminal starts it (a login shell, interactive: its startup files
/// run), says its `PATH` is. Runs a program that reads the person's startup files, so call it
/// from a background task.
pub fn login_path(shell: &str, wait: Duration) -> Option<String> {
    let script = format!("printf '\\n{BEGIN}%s{END}\\n' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    // The shell may leave something running that holds the pipe open, so the read can't be what
    // waits: it reports over a channel, and the wait is the channel's.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = (&mut out).take(1 << 16).read_to_end(&mut bytes);
        let _ = tx.send(bytes);
    });
    let said = rx.recv_timeout(wait);
    let _ = child.kill();
    let _ = child.wait();
    between_markers(&String::from_utf8_lossy(&said.ok()?))
}

/// The `PATH` between the markers in `output`: the last pair, in case the shell echoed the script.
fn between_markers(output: &str) -> Option<String> {
    let start = output.rfind(BEGIN)? + BEGIN.len();
    let len = output[start..].find(END)?;
    Some(output[start..start + len].to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{advice, between_markers, login_path, on_path, path_with_first};

    #[test]
    fn the_folder_goes_first_once() {
        let dir = Path::new("/data/Wormhole/bin");
        assert_eq!(path_with_first(dir, "/usr/bin:/bin"), "/data/Wormhole/bin:/usr/bin:/bin");
        assert_eq!(path_with_first(dir, "/data/Wormhole/bin:/usr/bin"), "/data/Wormhole/bin:/usr/bin");
        assert_eq!(path_with_first(dir, "/data/Wormhole/bin/:/usr/bin"), "/data/Wormhole/bin/:/usr/bin");
        // Elsewhere in it is not first: it goes first as well.
        assert_eq!(
            path_with_first(dir, "/usr/bin:/data/Wormhole/bin"),
            "/data/Wormhole/bin:/usr/bin:/data/Wormhole/bin"
        );
    }

    #[test]
    fn an_empty_path_never_leaves_a_bare_colon_that_means_the_current_folder() {
        let dir = Path::new("/data/bin");
        assert_eq!(path_with_first(dir, ""), "/data/bin");
        assert_eq!(path_with_first(dir, ":/usr/bin::"), "/data/bin:/usr/bin");
    }

    #[test]
    fn a_folder_that_cannot_be_an_entry_leaves_the_path_as_it_is() {
        assert_eq!(path_with_first(Path::new("/odd:folder/bin"), "/usr/bin"), "/usr/bin");
        assert_eq!(path_with_first(Path::new(""), "/usr/bin"), "/usr/bin");
    }

    #[test]
    fn a_folder_is_on_the_path_when_an_entry_names_it() {
        let root = tempfile::tempdir().expect("temp dir");
        let dir = root.path().join("bin");
        std::fs::create_dir(&dir).expect("folder");
        let text = |p: &Path| p.to_str().expect("utf-8").to_string();
        assert!(on_path(&dir, &format!("/usr/bin:{}:/bin", text(&dir))));
        assert!(on_path(&dir, &format!("{}/", text(&dir))), "a trailing slash is the same folder");
        assert!(!on_path(&dir, "/usr/bin:/bin"));
        assert!(!on_path(&dir, ""));
        // Through a link: the same folder by another name.
        let link = root.path().join("link");
        crate::platform::fs::symlink(&dir, &link).expect("link");
        assert!(on_path(&dir, &text(&link)));
    }

    #[test]
    fn the_line_to_add_fits_the_shell_and_keeps_the_home_folder_a_variable() {
        let home = Path::new("/Users/me");
        let dir = Path::new("/Users/me/.local/bin");
        let export = "export PATH=\"$HOME/.local/bin:$PATH\"";
        let zsh = advice(dir, "/bin/zsh", Some(home));
        assert_eq!((zsh.file.as_str(), zsh.line.as_str()), ("~/.zshrc", export));
        assert_eq!(advice(dir, "/opt/homebrew/bin/bash", Some(home)).file, "~/.bash_profile");
        let fish = advice(dir, "/opt/homebrew/bin/fish", Some(home));
        assert_eq!(fish.line, "fish_add_path \"$HOME/.local/bin\"");
        assert!(fish.file.ends_with("config.fish"));
        let other = advice(Path::new("/opt/tools/bin"), "/bin/tcsh", Some(home));
        assert_eq!(other.line, "export PATH=\"/opt/tools/bin:$PATH\"");
        assert_eq!(other.file, "your shell's startup file");
    }

    #[test]
    fn the_path_is_found_among_what_a_startup_file_prints() {
        let output = "Last login: Sat\nWelcome!\n\nWORMHOLE_LOGIN_PATH_BEGIN:/a:/b:WORMHOLE_LOGIN_PATH_END\n";
        assert_eq!(between_markers(output).as_deref(), Some("/a:/b"));
        assert_eq!(between_markers("nothing here"), None);
        let echoed = "printf WORMHOLE_LOGIN_PATH_BEGIN:%s:WORMHOLE_LOGIN_PATH_END\n\
                      WORMHOLE_LOGIN_PATH_BEGIN:/x:WORMHOLE_LOGIN_PATH_END";
        assert_eq!(between_markers(echoed).as_deref(), Some("/x"), "the last pair is the answer");
    }

    /// A program that stands in for the login shell: `body` runs as its script.
    fn fake_shell(dir: &Path, body: &str) -> String {
        let path = dir.join("fake-shell");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("script");
        std::fs::set_permissions(&path, crate::platform::fs::permissions(0o755)).expect("chmod");
        path.to_str().expect("utf-8").to_string()
    }

    #[test]
    fn a_login_shell_is_asked_for_its_path_the_way_a_terminal_starts_it() {
        let root = tempfile::tempdir().expect("temp dir");
        // Prints what its startup file would, then answers; and shows the flags it was given.
        let shell = fake_shell(
            root.path(),
            "echo 'motd: welcome back'; echo \"flags: $1 $2 $3\" >&2; PATH=/x:/y; eval \"$4\"",
        );
        assert_eq!(login_path(&shell, Duration::from_secs(10)).as_deref(), Some("/x:/y"));
    }

    #[test]
    fn a_shell_that_is_missing_fails_or_hangs_gives_no_answer_in_time() {
        let root = tempfile::tempdir().expect("temp dir");
        assert_eq!(login_path("/nonexistent/shell", Duration::from_secs(1)), None);
        let broken = fake_shell(root.path(), "exit 3");
        assert_eq!(login_path(&broken, Duration::from_secs(5)), None);
        let hangs = fake_shell(root.path(), "exec sleep 30");
        let started = Instant::now();
        assert_eq!(login_path(&hangs, Duration::from_millis(300)), None);
        assert!(started.elapsed() < Duration::from_secs(10), "it gave up");
    }
}
