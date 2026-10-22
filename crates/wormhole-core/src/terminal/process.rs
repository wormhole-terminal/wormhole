//! The shell process behind a terminal: what it runs, with which environment, and
//! the per-process queries (current folder) the UI polls.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use wormhole_term::{Launch, Shell, types};

use crate::terminal::shells::{self, ShellCommand, program_name};

/// How to start a tab's shell. It lives here because what runs and its environment are read from
/// it here; `spawn` adds the part the engine is started with.
#[derive(Clone, Debug, Default)]
pub struct SpawnOptions {
    pub cwd: Option<PathBuf>,
    /// The tab this shell belongs to, as `WORMHOLE_TAB_ID`: what its agent hooks report.
    pub tab_id: String,
    /// Previous session text, replayed dimmed before the shell starts.
    pub restore: Option<String>,
    pub scrollback_lines: usize,
    /// Answer DECRQCRA, a program's request for the checksum of a rectangle of the screen
    /// (`allow_screen_checksum`); the engine ignores it otherwise.
    pub allow_screen_checksum: bool,
    /// Let the program act on the window (XTWINOPS: move, resize, minimise, …; DECCOLM changing the
    /// width) (`allow_window_ops`); the engine ignores both otherwise.
    pub allow_window_ops: bool,
    /// Answer the program's request for the window title and icon name (XTWINOPS 20 and 21)
    /// (`allow_title_report`); the engine ignores it otherwise.
    pub allow_title_report: bool,
    /// Extra environment, e.g. from a trusted project's `config.json`.
    pub env: BTreeMap<String, String>,
    /// The program to run instead of the login shell: the tab's profile, a trusted project's
    /// shell or the default, decided by `shells::for_tab`.
    pub shell: Option<crate::terminal::shells::ShellCommand>,
    /// Start the program through `Wormhole welcome -- <program>`: the first-run welcome in this
    /// terminal, which becomes the program when it ends. The app also puts the welcome's token in
    /// `env` (`welcome::TOKEN_VAR`).
    pub welcome: bool,
    /// The theme's colors, which the engine answers a program's color queries (OSC 4, 10, 11, 12)
    /// with from its first byte; without them it answers with xterm's until
    /// [`Pty::set_default_colors`](wormhole_term::Pty::set_default_colors) is called, which a terminal that was never drawn has not yet.
    pub colors: Option<Box<[types::Rgb; types::COLOR_COUNT]>>,
    /// Whether the theme is light or dark, which the engine answers a program's `CSI ? 996 n`
    /// with from its first byte; unanswered until [`Pty::set_color_scheme`](wormhole_term::Pty::set_color_scheme) says.
    pub color_scheme: Option<types::ColorScheme>,
}

impl SpawnOptions {
    /// Start this terminal as `profile`'s: its environment, git identity and agent logins
    /// (`Profile::terminal_env`), `WORMHOLE_PROFILE`, and its start folder when no folder was asked
    /// for. Call it after the project's environment is in: what the project already set (a trusted
    /// project knows its own needs) is kept, and only what is missing is filled in from the profile.
    /// The terminal's id, socket and binary are set at launch and can't be redirected by either.
    pub fn apply_profile(&mut self, profile: &crate::config::profile::Profile) {
        for (key, value) in profile.terminal_env() {
            self.env.entry(key).or_insert(value);
        }
        self.env.insert(crate::config::profile::PROFILE_VAR.into(), profile.id.to_string());
        if self.cwd.is_none() {
            self.cwd = profile.start_folder();
        }
    }
}

/// The settings that make a process one instance of Wormhole: where it keeps its data, what it
/// is told by devctl, whether it stays in the background, how it answers Touch ID and
/// notifications, what it runs for a shell, where it starts, which git and `gh`, agent home and
/// agent folders it uses, where it looks for updates and which folders its command line tool goes in.
/// A tab's shell inherits none of them (`launch`). (`WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and
/// `WORMHOLE_BIN` are the tab's own and are set in `launch_options`.)
pub(super) const INSTANCE_VARS: [&str; 14] = [
    "WORMHOLE_DATA_DIR",
    "WORMHOLE_DEVCTL",
    "WORMHOLE_BACKGROUND",
    "WORMHOLE_AUTH",
    "WORMHOLE_NOTIFY",
    "WORMHOLE_SHELL",
    "WORMHOLE_START_DIR",
    "WORMHOLE_AGENT_HOME",
    "WORMHOLE_AGENT_PATH",
    "WORMHOLE_GIT_BIN",
    "WORMHOLE_GH_BIN",
    "WORMHOLE_UPDATE_URL",
    crate::terminal::cli_tool::DIR_VAR,
    crate::terminal::cli_tool::LOGIN_PATH_VAR,
];

/// What a tab runs. WORMHOLE_SHELL replaces everything (e.g. `/bin/cat` for safe automated UI
/// tests, where typed text is echoed but never executed). Otherwise the program the app decided
/// on (`options.shell`: the tab's profile, a trusted project's shell or the default, see
/// `shells::for_tab`), else the user's login shell.
pub(super) fn shell(options: &SpawnOptions) -> Shell {
    let (program, args) = match std::env::var("WORMHOLE_SHELL") {
        Ok(program) => (program, Vec::new()),
        Err(_) => {
            let ShellCommand { program, args, .. } = command(options);
            (program, args)
        }
    };
    // The first run's welcome starts first and becomes whatever would have run (WORMHOLE_SHELL's
    // `/bin/cat` in a test instance included).
    let (program, args) = match std::env::current_exe() {
        Ok(exe) if options.welcome => welcome_command(&exe, program, args),
        _ => (program, args),
    };
    launch(program, args)
}

/// What to run in a terminal for the welcome: the app's own binary as `welcome -- <program> <args…>`,
/// which becomes `program` when the welcome ends (`welcome::parse_args` reads it back).
pub(crate) fn welcome_command(exe: &Path, program: String, args: Vec<String>) -> (String, Vec<String>) {
    let mut words = vec!["welcome".to_string(), "--".to_string(), program];
    words.extend(args);
    (exe.to_string_lossy().into_owned(), words)
}

/// The program `options` ask for, the login shell without one.
fn command(options: &SpawnOptions) -> ShellCommand {
    options.shell.clone().unwrap_or_else(|| ShellCommand::login(&shells::login_shell()))
}

/// The program that clears variables and then becomes the shell.
const ENV: &str = "/usr/bin/env";

/// The `PATH` a process gets from `launchd` (the Finder's), for an app that has none at all.
const FALLBACK_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

/// `program` as the PTY's shell, started so that none of [`INSTANCE_VARS`] reaches it. A child
/// gets the app's environment with additions, and the app itself reads these variables all the
/// time, so they can't be removed from its own environment either: the shell is started through
/// `env -u …`, which replaces itself with the shell (same pid, same process group, so agent
/// detection and the foreground program are unaffected).
pub(super) fn launch(program: String, args: Vec<String>) -> Shell {
    let (program, args) = without_instance_vars(program, args, std::path::Path::new(ENV).exists());
    Shell::new(program, args)
}

/// The program and arguments that start `program` without [`INSTANCE_VARS`]: `env -u … -- program
/// args`, or `program args` as they are when `env` can't carry them. `env` reads `NAME=VALUE` as an
/// assignment, so a program with a `=` in its path would be mistaken for one; no real shell is
/// called that, and it starts as it is, with a warning.
fn without_instance_vars(program: String, args: Vec<String>, has_env: bool) -> (String, Vec<String>) {
    if program.contains('=') || !has_env {
        tracing::warn!(
            "a tab's shell inherits the instance's own settings: {program} can't go through {ENV}"
        );
        return (program, args);
    }
    let mut wrapped = Vec::with_capacity(2 * INSTANCE_VARS.len() + 2 + args.len());
    for var in INSTANCE_VARS {
        wrapped.extend(["-u".to_string(), var.to_string()]);
    }
    wrapped.push("--".into());
    wrapped.push(program);
    wrapped.extend(args);
    (ENV.into(), wrapped)
}

/// The name of the program a tab runs ("zsh", "claude"), as the status bar and the tab show it:
/// what `shell` starts, named after the program asked for even when it runs through the login shell.
pub fn shell_name(options: &SpawnOptions) -> String {
    match std::env::var("WORMHOLE_SHELL") {
        Ok(program) => program_name(&program).to_string(),
        Err(_) => command(options).name,
    }
}

/// How to run `shell`: folder and environment.
pub(super) fn launch_options(options: &SpawnOptions, shell: Shell) -> Launch {
    launch_options_with(options, shell, crate::terminal::cli_tool::terminal_dir().as_deref())
}

/// [`launch_options`] with the folder of the `wh` command given: put first on the shell's `PATH` so the
/// command works in every terminal Wormhole starts. `None` when the script isn't there (its write
/// at launch failed): the terminal starts all the same, without the command.
fn launch_options_with(options: &SpawnOptions, shell: Shell, command_dir: Option<&Path>) -> Launch {
    let SpawnOptions { cwd, tab_id, env: extra_env, .. } = options;

    let mut env = HashMap::new();
    env.insert("TERM".into(), "xterm-256color".into());
    env.insert("COLORTERM".into(), "truecolor".into());
    env.insert("TERM_PROGRAM".into(), "Wormhole".into());
    // Core and the app share the workspace's version (`version.workspace = true` in both), so
    // this is the app's.
    env.insert("TERM_PROGRAM_VERSION".into(), env!("CARGO_PKG_VERSION").into());
    // Apps launched from Finder get no locale, which breaks UTF-8 in zsh.
    if std::env::var("LANG").map_or(true, |lang| lang.is_empty()) {
        env.insert("LANG".into(), "en_US.UTF-8".into());
    }
    for (key, value) in extra_env {
        env.insert(key.clone(), value.clone());
    }
    // On top of the project's `PATH` when it sets one, else the app's own. A shell whose startup
    // files rebuild `PATH` from scratch loses the folder; the docs name the alias as the way
    // around that.
    if let Some(dir) = command_dir {
        let base = env.get("PATH").cloned().or_else(|| std::env::var("PATH").ok()).unwrap_or_default();
        let base = if base.is_empty() { FALLBACK_PATH.to_string() } else { base };
        env.insert("PATH".into(), crate::terminal::cli_tool::path_with_first(dir, &base));
    }
    // After the project's: what the hooks need to reach this tab can't be
    // redirected by a project's `config.json`.
    env.insert("WORMHOLE_TAB_ID".into(), tab_id.clone());
    env.insert("WORMHOLE_SOCKET".into(), crate::ipc::socket_path().to_string_lossy().into_owned());
    if let Ok(exe) = std::env::current_exe() {
        env.insert("WORMHOLE_BIN".into(), exe.to_string_lossy().into_owned());
    }
    // The same under the names versions up to 0.2 used, so their hooks (until they are replaced
    // at launch) and a shell's `alias wormhole='"$ANDROMEDA_BIN"'` keep working.
    for (name, old) in crate::ipc::LEGACY_TAB_VARS {
        if let Some(value) = env.get(*name).cloned() {
            env.insert((*old).into(), value);
        }
    }
    Launch { shell, cwd: cwd.clone().or_else(|| std::env::var_os("HOME").map(PathBuf::from)), env }
}

#[cfg(test)]
mod name_tests {
    use super::{
        ENV, INSTANCE_VARS, Shell, ShellCommand, SpawnOptions, command, launch_options, launch_options_with,
        without_instance_vars,
    };

    /// The profile the app decided on is what starts, and what the tab is named after; without
    /// one the login shell (which `shell_name` can't be tested on: `WORMHOLE_SHELL` and `$SHELL`
    /// are the process's).
    #[test]
    fn a_tab_starts_the_program_its_options_ask_for() {
        let chosen = ShellCommand {
            program: "/bin/zsh".into(),
            args: vec!["-l".into(), "-c".into(), "exec claude".into()],
            name: "claude".into(),
        };
        let options = SpawnOptions { shell: Some(chosen.clone()), ..SpawnOptions::default() };
        assert_eq!(command(&options), chosen);
        let plain = command(&SpawnOptions::default());
        assert_eq!(plain.args, ["-l"], "the login shell, as a login shell");
        assert!(plain.program.starts_with('/'), "{}", plain.program);
    }

    #[test]
    fn the_shell_starts_through_env_without_the_instances_variables() {
        let (program, args) = without_instance_vars("/bin/zsh".into(), vec!["-l".into()], true);
        assert_eq!(program, ENV);
        let cleared: Vec<&str> = args
            .chunks(2)
            .take(INSTANCE_VARS.len())
            .filter(|pair| pair[0] == "-u")
            .map(|pair| &*pair[1])
            .collect();
        assert_eq!(cleared, INSTANCE_VARS);
        assert_eq!(
            &args[2 * INSTANCE_VARS.len()..],
            ["--", "/bin/zsh", "-l"],
            "the shell and its arguments follow"
        );
    }

    #[test]
    fn a_program_env_would_misread_or_cannot_reach_starts_as_it_is() {
        let args = vec!["-l".to_string()];
        for (program, has_env) in [("/opt/a=b/zsh", true), ("/bin/zsh", false)] {
            let (started, with) = without_instance_vars(program.into(), args.clone(), has_env);
            assert_eq!((started.as_str(), &with), (program, &args));
        }
    }

    #[test]
    fn a_tab_gets_its_variables_under_the_old_names_too() {
        let options = SpawnOptions { tab_id: "7".into(), ..SpawnOptions::default() };
        let env = launch_options(&options, Shell::new("/bin/sh".into(), Vec::new())).env;
        for (name, old) in crate::ipc::LEGACY_TAB_VARS {
            assert_eq!(env.get(*old), env.get(*name), "{old}");
            assert!(env.contains_key(*name) || *name == "WORMHOLE_BIN", "{name}");
        }
        assert_eq!(env.get("ANDROMEDA_TAB_ID").map(String::as_str), Some("7"));
    }

    fn env_with_command_dir(options: &SpawnOptions, dir: Option<&std::path::Path>) -> Env {
        launch_options_with(options, Shell::new("/bin/sh".into(), Vec::new()), dir).env
    }

    type Env = std::collections::HashMap<String, String>;

    #[test]
    fn the_commands_folder_is_first_on_a_terminals_path() {
        let dir = std::path::Path::new("/data/Wormhole/bin");
        let env = env_with_command_dir(&SpawnOptions::default(), Some(dir));
        let path = env.get("PATH").expect("a PATH is set for the terminal");
        assert!(path.starts_with("/data/Wormhole/bin:"), "{path}");
        let rest = &path["/data/Wormhole/bin:".len()..];
        // Behind it, the app's own PATH: the terminal finds what it found before.
        let own = std::env::var("PATH").unwrap_or_default();
        assert_eq!(rest, if own.is_empty() { super::FALLBACK_PATH } else { own.as_str() });
    }

    #[test]
    fn a_projects_own_path_keeps_its_entries_behind_the_commands_folder() {
        let dir = std::path::Path::new("/data/Wormhole/bin");
        let options = SpawnOptions {
            env: [("PATH".to_string(), "/project/tools:/usr/bin".to_string())].into(),
            ..SpawnOptions::default()
        };
        let env = env_with_command_dir(&options, Some(dir));
        assert_eq!(env.get("PATH").map(String::as_str), Some("/data/Wormhole/bin:/project/tools:/usr/bin"));
    }

    /// The script could not be written at launch (a read-only data folder, a full disk): the
    /// terminal opens all the same, inheriting the app's `PATH` as it always did.
    #[test]
    fn a_terminal_starts_without_the_command_when_its_script_is_not_there() {
        let blocked = tempfile::tempdir().expect("temp dir");
        std::fs::write(blocked.path().join("bin"), "a file where the folder should be").expect("file");
        let failed = crate::terminal::cli_tool::keep_current_in(
            &blocked.path().join("bin"),
            std::path::Path::new("/opt/Wormhole"),
        );
        assert!(failed.is_err(), "the write fails, and the launch only logs it");

        let env = env_with_command_dir(&SpawnOptions::default(), None);
        assert!(!env.contains_key("PATH"), "the PATH is the app's own, untouched");
        assert!(env.contains_key("TERM"), "everything else is set as usual");
    }

    #[test]
    fn the_tabs_own_variables_are_not_cleared() {
        for own in ["WORMHOLE_TAB_ID", "WORMHOLE_SOCKET", "WORMHOLE_BIN"] {
            assert!(!INSTANCE_VARS.contains(&own), "{own} is set for the tab in launch_options");
        }
    }
}
