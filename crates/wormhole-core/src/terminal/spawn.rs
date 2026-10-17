//! Starting a tab's shell: what it runs and with which environment (`process`), handed to the
//! terminal layer as a [`Launch`](super::Launch) with the options the engine is started with.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{GridSize, Pty, PtyEvents, StartOptions, process, types};

/// How to start a tab's shell.
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
    /// width) and read its title back (`allow_window_ops`); the engine ignores both otherwise.
    pub allow_window_ops: bool,
    /// Extra environment, e.g. from a trusted project's `config.json`.
    pub env: BTreeMap<String, String>,
    /// The program to run instead of the login shell: the tab's profile, a trusted project's
    /// shell or the default, decided by `shells::for_tab`.
    pub shell: Option<crate::shells::ShellCommand>,
    /// Start the program through `Wormhole welcome -- <program>`: the first-run welcome in this
    /// terminal, which becomes the program when it ends. The app also puts the welcome's token in
    /// `env` (`welcome::TOKEN_VAR`).
    pub welcome: bool,
    /// The theme's colors, which the engine answers a program's color queries (OSC 4, 10, 11, 12)
    /// with from its first byte; without them it answers with xterm's until
    /// [`Pty::set_default_colors`] is called, which a terminal that was never drawn has not yet.
    pub colors: Option<Box<[types::Rgb; types::COLOR_COUNT]>>,
}

impl SpawnOptions {
    /// Start this terminal as `profile`'s: its environment, git identity and agent logins
    /// (`Profile::terminal_env`), `WORMHOLE_PROFILE`, and its start folder when no folder was asked
    /// for. Call it after the project's environment is in: what the project already set (a trusted
    /// project knows its own needs) is kept, and only what is missing is filled in from the profile.
    /// The terminal's id, socket and binary are set at launch and can't be redirected by either.
    pub fn apply_profile(&mut self, profile: &crate::profile::Profile) {
        for (key, value) in profile.terminal_env() {
            self.env.entry(key).or_insert(value);
        }
        self.env.insert(crate::profile::PROFILE_VAR.into(), profile.id.to_string());
        if self.cwd.is_none() {
            self.cwd = profile.start_folder();
        }
    }

    /// The part of the options the engine is started with.
    fn start_options(&self) -> StartOptions<'_> {
        StartOptions {
            restore: self.restore.as_deref(),
            scrollback_lines: self.scrollback_lines,
            allow_screen_checksum: self.allow_screen_checksum,
            allow_window_ops: self.allow_window_ops,
            colors: self.colors.as_deref(),
        }
    }
}

/// Start a tab's shell in a PTY of `size`.
pub fn spawn(size: GridSize, options: &SpawnOptions) -> anyhow::Result<(Pty, PtyEvents)> {
    let launch = process::launch_options(options, process::shell(options));
    Pty::start(size, &options.start_options(), launch)
}

/// [`spawn`] with an explicit program instead of the user's shell.
#[cfg(test)]
pub(crate) fn spawn_program(
    size: GridSize,
    options: &SpawnOptions,
    program: &str,
    args: &[&str],
) -> anyhow::Result<(Pty, PtyEvents)> {
    let args = args.iter().map(|arg| arg.to_string()).collect();
    let launch = process::launch_options(options, process::launch(program.into(), args));
    Pty::start(size, &options.start_options(), launch)
}

/// What a tab's launch does to its shell's environment, seen from a program on a real PTY.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{SpawnOptions, spawn_program};
    use crate::terminal::{GridSize, process};

    const SIZE: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_project_env_cannot_redirect_the_hooks() {
        let mut options = SpawnOptions { tab_id: "real-tab".into(), ..SpawnOptions::default() };
        options.env.insert("WORMHOLE_TAB_ID".into(), "spoofed".into());
        options.env.insert("WORMHOLE_SOCKET".into(), "/tmp/elsewhere.sock".into());
        options.env.insert("PROJECT_FLAG".into(), "on".into());
        let script = r#"printf '[%s] [%s] [%s] [%s %s]\n' "$WORMHOLE_TAB_ID" "$PROJECT_FLAG" "$WORMHOLE_SOCKET" "$TERM_PROGRAM" "$TERM_PROGRAM_VERSION"; sleep 5"#;
        let (pty, _events) = spawn_program(SIZE, &options, "/bin/sh", &["-c", script]).expect("spawn");
        wait_until("the output", || pty.screen_text().contains("[real-tab] [on]"));
        let screen = pty.screen_text();
        assert!(!screen.contains("elsewhere"), "{screen}");
        let version = env!("CARGO_PKG_VERSION");
        assert!(screen.contains(&format!("[Wormhole {version}]")), "{screen}");
    }

    /// The variables that configure an instance (its data folder, its devctl folder, background mode, …)
    /// stay with the instance: `cargo run` in a test instance's tab would otherwise start a second
    /// instance on the first one's data and take commands meant for it. A process's environment can't be
    /// changed from inside a test, so the check runs again in a child process that has them all.
    #[test]
    fn a_shell_does_not_inherit_the_instances_own_settings() {
        const CHILD: &str = "WORMHOLE_SCRUB_TEST_CHILD";
        const TEST: &str = "terminal::spawn::tests::a_shell_does_not_inherit_the_instances_own_settings";
        if std::env::var_os(CHILD).is_none() {
            let exe = std::env::current_exe().expect("finds the test binary");
            let mut command = std::process::Command::new(exe);
            command.args([TEST, "--exact", "--test-threads=1"]).env(CHILD, "1");
            for var in process::INSTANCE_VARS {
                command.env(var, "/leaked");
            }
            // Not the instance's: a person's own log level reaches their shell.
            command.env("WORMHOLE_LOG", "debug");
            let output = command.output().expect("runs the test binary");
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "{TEST} failed in its child process:\n{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let options = SpawnOptions { tab_id: "real-tab".into(), ..SpawnOptions::default() };
        let shows: String = process::INSTANCE_VARS
            .iter()
            .chain(&["WORMHOLE_LOG", "WORMHOLE_TAB_ID"])
            .map(|var| format!(" \"{var}=${{{var}-unset}}\""))
            .collect();
        let script = format!("printf '%s\\n'{shows}; sleep 5");
        let (pty, _events) = spawn_program(SIZE, &options, "/bin/sh", &["-c", &script]).expect("spawn");
        wait_until("the output", || pty.screen_text().contains("WORMHOLE_TAB_ID=real-tab"));
        let screen = pty.screen_text();
        for var in process::INSTANCE_VARS {
            assert!(screen.contains(&format!("{var}=unset")), "{var} reached the shell:\n{screen}");
        }
        assert!(
            screen.contains("WORMHOLE_LOG=debug"),
            "a setting that isn't the instance's stays:\n{screen}"
        );
    }
}

#[cfg(test)]
mod profile_tests {
    use super::{SpawnOptions, spawn_program};
    use crate::agents::AgentId;
    use crate::profile::{AgentLogin, PROFILE_VAR, Profile, ProfileId};
    use crate::sidebar::GroupColor;
    use crate::terminal::GridSize;

    const SIZE: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

    fn work() -> Profile {
        let mut profile = Profile::new(ProfileId::parse("work").expect("an id"), "Work", GroupColor::Cyan);
        profile.env.insert("AWS_PROFILE".into(), "acme-prod".into());
        profile.env.insert("PROJECT_FLAG".into(), "from-profile".into());
        profile.git.name = "Benny".into();
        profile.git.email = "benny@acme.dev".into();
        profile.agent_logins = vec![AgentLogin { agent: AgentId::Claude, dir: "/logins/claude-work".into() }];
        profile.start_dir = "/tmp".into();
        profile
    }

    #[test]
    fn a_profile_fills_in_what_the_project_did_not_set() {
        let mut options = SpawnOptions { tab_id: "t1".into(), ..SpawnOptions::default() };
        options.env.insert("PROJECT_FLAG".into(), "from-project".into());
        options.apply_profile(&work());
        assert_eq!(options.env.get("PROJECT_FLAG").map(String::as_str), Some("from-project"));
        assert_eq!(options.env.get("AWS_PROFILE").map(String::as_str), Some("acme-prod"));
        assert_eq!(options.env.get(PROFILE_VAR).map(String::as_str), Some("work"));
        assert_eq!(options.cwd.as_deref(), Some(std::path::Path::new("/tmp")));
        let asked = SpawnOptions { cwd: Some("/var".into()), ..SpawnOptions::default() };
        let mut options = asked;
        options.apply_profile(&work());
        assert_eq!(options.cwd.as_deref(), Some(std::path::Path::new("/var")), "an asked-for folder wins");
    }

    /// The shell really sees them, next to the tab's own variables, which a profile can't redirect.
    #[test]
    fn a_shell_started_in_a_profile_sees_its_environment() {
        let mut options = SpawnOptions { tab_id: "real-tab".into(), ..SpawnOptions::default() };
        let mut profile = work();
        profile.env.insert("WORMHOLE_TAB_ID".into(), "spoofed".into());
        // Refused by `check`, but a hand-built profile must not get through either.
        options.apply_profile(&profile);
        let script = r#"printf '[%s] [%s] [%s] [%s] [%s] [%s]\n' "$AWS_PROFILE" "$GIT_AUTHOR_EMAIL" "$CLAUDE_CONFIG_DIR" "$WORMHOLE_PROFILE" "$WORMHOLE_TAB_ID" "$PWD"; sleep 5"#;
        let (pty, _events) = spawn_program(SIZE, &options, "/bin/sh", &["-c", script]).expect("spawn");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !pty.screen_text().contains("[real-tab]") {
            assert!(std::time::Instant::now() < deadline, "timed out:\n{}", pty.screen_text());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let screen = pty.screen_text();
        assert!(
            screen.contains("[acme-prod] [benny@acme.dev] [/logins/claude-work] [work] [real-tab]"),
            "{screen}"
        );
    }
}
