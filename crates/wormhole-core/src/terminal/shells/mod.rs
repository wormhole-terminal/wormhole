//! Shell profiles: what a new terminal runs. `config.json` names a default
//! (`default_shell`) and may list profiles (`shells`: a program with its arguments); the UI offers
//! those next to the login shell and the shells `/etc/shells` lists that are installed.
//!
//! A key names a choice wherever it is stored or shown: `""` is the login shell (`$SHELL`), a
//! profile's name is that profile, anything else is a program's path, started as a login shell. A
//! terminal remembers its key, so a restart of its shell or a restored session runs the same thing.
//! What a terminal runs is decided here ([`for_terminal`]), in this order: the key chosen for it, a
//! trusted project's `shell`, `default_shell`, then the login shell. The engine's `WORMHOLE_SHELL`
//! (tests) comes before all of it, in `terminal::process`.

mod quote;

use std::path::Path;
use std::sync::LazyLock;

pub use quote::{quote, quote_all, single_quote};

use crate::config::{Settings, ShellProfile};
use crate::storage::fsutil::{self, LinkPolicy};

/// The key of the login shell.
pub const LOGIN: &str = "";

/// The shell to fall back on when `$SHELL` isn't set.
const FALLBACK: &str = "/bin/zsh";

/// `/etc/shells` is a few lines; a file that is far bigger isn't it.
const ETC_SHELLS_CAP: u64 = 64 * 1024;

/// What a terminal runs: a program with its arguments, and the name shown for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellCommand {
    pub program: String,
    pub args: Vec<String>,
    /// What the terminal and the status bar call it ("zsh", "fish", "claude"): the program asked for,
    /// also when it runs through the login shell.
    pub name: String,
}

impl ShellCommand {
    /// `program` started as a login shell (`-l`), as the user's shell always was.
    pub fn login(program: &str) -> Self {
        Self {
            program: program.to_string(),
            args: vec!["-l".into()],
            name: program_name(program).to_string(),
        }
    }

    /// The program and its arguments as one line, for a detail or a menu.
    pub fn command_line(&self) -> String {
        quote_all(std::iter::once(&self.program).chain(&self.args))
    }
}

/// Where a choice comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// `$SHELL`.
    Login,
    /// A profile from `config.json`.
    Profile,
    /// Listed in `/etc/shells` and installed.
    System,
}

/// A shell the menus and Settings offer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellChoice {
    /// What a terminal remembers and `default_shell` names (see the module notes).
    pub key: String,
    /// "zsh", "fish", or a profile's name.
    pub name: String,
    /// What it runs, as a command line ("/opt/homebrew/bin/fish -l").
    pub command_line: String,
    pub kind: ShellKind,
}

/// The user's login shell: `$SHELL`, else `/bin/zsh`.
pub fn login_shell() -> String {
    std::env::var("SHELL").ok().filter(|shell| !shell.trim().is_empty()).unwrap_or_else(|| FALLBACK.into())
}

/// The last component of a program's path ("/bin/zsh" → "zsh").
pub fn program_name(program: &str) -> &str {
    program.rsplit('/').find(|part| !part.is_empty()).unwrap_or(program)
}

/// What `key` runs, or `None` for a key that names nothing (a profile that is gone from
/// `config.json`).
pub fn resolve(key: &str, settings: &Settings) -> Option<ShellCommand> {
    resolve_with(key, &settings.shells, &login_shell())
}

fn resolve_with(key: &str, profiles: &[ShellProfile], login: &str) -> Option<ShellCommand> {
    let key = key.trim();
    if key == LOGIN {
        return Some(ShellCommand::login(login));
    }
    if let Some(profile) = profiles.iter().find(|profile| profile.name == key) {
        return Some(from_profile(profile, login));
    }
    key.contains('/').then(|| ShellCommand::login(key))
}

/// A profile's program with its arguments. A bare name (no `/`) runs through the login shell
/// (`$SHELL -l -c 'exec claude …'`): an app started from the Finder has a short PATH, and the
/// login shell's startup files set the person's real one.
fn from_profile(profile: &ShellProfile, login: &str) -> ShellCommand {
    let command = profile.command.trim();
    let args = profile.args.clone().unwrap_or_else(|| vec!["-l".into()]);
    let name = program_name(command).to_string();
    if command.contains('/') {
        return ShellCommand { program: command.to_string(), args, name };
    }
    let line = format!("exec {}", quote_all(std::iter::once(command).chain(args.iter().map(String::as_str))));
    ShellCommand { program: login.to_string(), args: vec!["-l".into(), "-c".into(), line], name }
}

/// The program for a terminal: the profile chosen for it (`chosen`, its remembered key), else a trusted
/// project's `shell`, else `default_shell`, else the login shell. A key that names nothing is said
/// in the log and passed over.
pub fn for_terminal(chosen: Option<&str>, project_shell: Option<&str>, settings: &Settings) -> ShellCommand {
    for_terminal_with(chosen, project_shell, settings, &login_shell())
}

fn for_terminal_with(
    chosen: Option<&str>,
    project_shell: Option<&str>,
    settings: &Settings,
    login: &str,
) -> ShellCommand {
    if let Some(key) = chosen {
        match resolve_with(key, &settings.shells, login) {
            Some(command) => return command,
            None => tracing::warn!("no shell profile {key:?}: the terminal starts the default shell instead"),
        }
    }
    if let Some(shell) = project_shell.map(str::trim).filter(|shell| !shell.is_empty()) {
        return ShellCommand::login(shell);
    }
    let default = default_key(settings);
    if default != LOGIN {
        match resolve_with(default, &settings.shells, login) {
            Some(command) => return command,
            None => tracing::warn!(
                "default_shell names no profile or program ({default:?}): using the login shell"
            ),
        }
    }
    ShellCommand::login(login)
}

/// The key `default_shell` names, as the choices spell it: what a plain + opens.
pub fn default_key(settings: &Settings) -> &str {
    settings.default_shell.trim()
}

/// The shells to offer, in order: the login shell, the profiles of `config.json`, then the
/// system's. A system shell that is the login shell or a profile's program is listed once.
pub fn choices(settings: &Settings) -> Vec<ShellChoice> {
    choices_with(settings, &login_shell(), system_shells())
}

fn choices_with(settings: &Settings, login: &str, system: &[String]) -> Vec<ShellChoice> {
    let mut out = vec![ShellChoice {
        key: LOGIN.into(),
        name: program_name(login).to_string(),
        command_line: ShellCommand::login(login).command_line(),
        kind: ShellKind::Login,
    }];
    for profile in settings.shells.iter().filter(|profile| profile.is_usable()) {
        if out.iter().any(|choice| choice.key == profile.name) {
            continue;
        }
        out.push(ShellChoice {
            key: profile.name.clone(),
            name: profile.name.clone(),
            command_line: from_profile(profile, login).command_line(),
            kind: ShellKind::Profile,
        });
    }
    for path in system {
        let taken = path == login || settings.shells.iter().any(|profile| profile.command.trim() == path);
        if taken || out.iter().any(|choice| choice.key == *path) {
            continue;
        }
        out.push(ShellChoice {
            key: path.clone(),
            name: program_name(path).to_string(),
            command_line: ShellCommand::login(path).command_line(),
            kind: ShellKind::System,
        });
    }
    out
}

/// The shells `/etc/shells` lists that are installed, read once: the menus ask on every tick and
/// the file changes about never while the app runs.
pub fn system_shells() -> &'static [String] {
    static SYSTEM: LazyLock<Vec<String>> = LazyLock::new(|| etc_shells(Path::new("/etc/shells")));
    &SYSTEM
}

/// The shells a file in `/etc/shells`' format lists that exist: one absolute path per line,
/// `#` comments and blank lines skipped. A missing or unreadable file lists nothing.
pub fn etc_shells(path: &Path) -> Vec<String> {
    let text = match fsutil::read_capped(path, ETC_SHELLS_CAP, LinkPolicy::Follow) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) => {
            tracing::debug!("{}: {err}", path.display());
            return Vec::new();
        }
    };
    parse_etc_shells(&text).into_iter().filter(|shell| Path::new(shell).is_file()).collect()
}

/// The paths a `/etc/shells` text lists, in order, each once.
fn parse_etc_shells(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('/') {
            continue;
        }
        if !out.iter().any(|known| known == line) {
            out.push(line.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        LOGIN, ShellChoice, ShellCommand, ShellKind, choices_with, etc_shells, for_terminal_with,
        parse_etc_shells, program_name, resolve_with,
    };
    use crate::config::{Settings, ShellProfile};

    fn profile(name: &str, command: &str, args: Option<&[&str]>) -> ShellProfile {
        ShellProfile {
            name: name.into(),
            command: command.into(),
            args: args.map(|args| args.iter().map(|a| a.to_string()).collect()),
        }
    }

    fn settings(default: &str) -> Settings {
        Settings {
            default_shell: default.into(),
            shells: vec![
                profile("fish", "/opt/homebrew/bin/fish", None),
                profile("Claude", "claude", Some(&[])),
                profile("Dev", "npm", Some(&["run", "dev server"])),
            ],
            ..Settings::default()
        }
    }

    const LOGIN_SHELL: &str = "/bin/zsh";

    fn line(command: &ShellCommand) -> String {
        command.command_line()
    }

    #[test]
    fn a_program_is_named_by_its_last_component() {
        assert_eq!(program_name("/bin/zsh"), "zsh");
        assert_eq!(program_name("/opt/homebrew/bin/fish"), "fish");
        assert_eq!(program_name("bash"), "bash");
        assert_eq!(program_name("/usr/local/bin/nu/"), "nu");
    }

    #[test]
    fn a_key_names_the_login_shell_a_profile_or_a_program() {
        let s = settings(LOGIN);
        let resolve = |key: &str| resolve_with(key, &s.shells, LOGIN_SHELL);
        assert_eq!(resolve(LOGIN).map(|c| line(&c)).as_deref(), Some("/bin/zsh -l"));
        assert_eq!(
            resolve(" ").map(|c| line(&c)).as_deref(),
            Some("/bin/zsh -l"),
            "blank is the login shell"
        );
        // A profile without args is a login shell; with `[]` the program runs bare.
        let fish = resolve("fish").expect("a profile");
        assert_eq!((line(&fish), fish.name.as_str()), ("/opt/homebrew/bin/fish -l".into(), "fish"));
        // A bare name goes through the login shell, which knows the person's PATH; its arguments
        // arrive as the words they were.
        let claude = resolve("Claude").expect("a profile");
        assert_eq!(line(&claude), "/bin/zsh -l -c 'exec claude'");
        assert_eq!(claude.name, "claude", "named after what runs, not the wrapper");
        let dev = resolve("Dev").expect("a profile");
        assert_eq!(dev.args, ["-l", "-c", "exec npm run 'dev server'"]);
        assert_eq!(dev.name, "npm");
        // A path is a program to start as a login shell; a bare unknown name is nothing.
        assert_eq!(resolve("/bin/bash").map(|c| line(&c)).as_deref(), Some("/bin/bash -l"));
        assert_eq!(resolve("nope"), None);
    }

    #[test]
    fn a_terminal_runs_its_choice_then_the_projects_then_the_default_then_the_login_shell() {
        let s = settings("fish");
        let run = |chosen: Option<&str>, project: Option<&str>, s: &Settings| {
            line(&for_terminal_with(chosen, project, s, LOGIN_SHELL))
        };
        assert_eq!(run(Some("Claude"), Some("/bin/bash"), &s), "/bin/zsh -l -c 'exec claude'");
        assert_eq!(run(Some(LOGIN), Some("/bin/bash"), &s), "/bin/zsh -l", "chosen: the login shell");
        assert_eq!(run(None, Some("/bin/bash"), &s), "/bin/bash -l", "the project's shell over the default");
        assert_eq!(run(None, Some("  "), &s), "/opt/homebrew/bin/fish -l", "a blank project shell is none");
        assert_eq!(run(None, None, &s), "/opt/homebrew/bin/fish -l", "the default");
        assert_eq!(run(None, None, &settings(LOGIN)), "/bin/zsh -l");
        assert_eq!(run(None, None, &settings("/bin/bash")), "/bin/bash -l", "a program as the default");
        // What names nothing is passed over, to the next in line.
        assert_eq!(run(Some("gone"), None, &s), "/opt/homebrew/bin/fish -l");
        assert_eq!(run(Some("gone"), Some("/bin/bash"), &s), "/bin/bash -l");
        assert_eq!(run(None, None, &settings("gone")), "/bin/zsh -l");
    }

    #[test]
    fn the_choices_are_the_login_shell_the_profiles_and_the_systems_once_each() {
        let system: Vec<String> =
            ["/bin/zsh", "/bin/bash", "/opt/homebrew/bin/fish", "/bin/sh"].map(String::from).to_vec();
        let found = choices_with(&settings(LOGIN), LOGIN_SHELL, &system);
        let shown: Vec<(&str, &str, &str, ShellKind)> = found
            .iter()
            .map(|c| (c.key.as_str(), c.name.as_str(), c.command_line.as_str(), c.kind))
            .collect();
        assert_eq!(
            shown,
            [
                ("", "zsh", "/bin/zsh -l", ShellKind::Login),
                ("fish", "fish", "/opt/homebrew/bin/fish -l", ShellKind::Profile),
                ("Claude", "Claude", "/bin/zsh -l -c 'exec claude'", ShellKind::Profile),
                ("Dev", "Dev", "/bin/zsh -l -c 'exec npm run '\\''dev server'\\'''", ShellKind::Profile),
                ("/bin/bash", "bash", "/bin/bash -l", ShellKind::System),
                ("/bin/sh", "sh", "/bin/sh -l", ShellKind::System),
            ]
        );
        // Nothing configured: the login shell and the system's.
        let plain = choices_with(&Settings::default(), LOGIN_SHELL, &system);
        let keys: Vec<&str> = plain.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["", "/bin/bash", "/opt/homebrew/bin/fish", "/bin/sh"]);
        // Every key resolves to what its line says.
        for ShellChoice { key, command_line, .. } in &found {
            let command = resolve_with(key, &settings(LOGIN).shells, LOGIN_SHELL).expect("a choice resolves");
            assert_eq!(&line(&command), command_line, "{key:?}");
        }
    }

    #[test]
    fn etc_shells_lists_absolute_paths_skipping_comments_blanks_and_repeats() {
        let text = "# List of acceptable shells for chpass(1).\n\n/bin/bash\n  /bin/zsh  \n/bin/bash\nrelative\n\
                    # /bin/csh\n/opt/homebrew/bin/fish\n";
        assert_eq!(parse_etc_shells(text), ["/bin/bash", "/bin/zsh", "/opt/homebrew/bin/fish"]);
        assert!(parse_etc_shells("").is_empty());
    }

    #[test]
    fn only_installed_shells_are_offered_and_a_missing_file_offers_none() {
        let dir = std::env::temp_dir().join(format!("wormhole-shells-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("shells");
        let present = std::env::current_exe().expect("this test binary exists");
        let text = format!("# shells\n{}\n/definitely/not/here\n", present.display());
        std::fs::write(&file, text).expect("write");
        assert_eq!(etc_shells(&file), [present.to_string_lossy().into_owned()]);
        assert!(etc_shells(&dir.join("absent")).is_empty());
        assert!(etc_shells(Path::new("/dev/null")).is_empty(), "nothing listed");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
