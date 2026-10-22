//! Starting a tab's shell: what it runs and with which environment (`process`), handed to the
//! terminal layer as a [`Launch`](wormhole_term::Launch) with the options the engine is started with.

use wormhole_term::{GridSize, Pty, PtyEvents, StartOptions};

use super::process::{self, SpawnOptions};

impl SpawnOptions {
    /// The part of the options the engine is started with.
    fn start_options(&self) -> StartOptions<'_> {
        StartOptions {
            restore: self.restore.as_deref(),
            scrollback_lines: self.scrollback_lines,
            allow_screen_checksum: self.allow_screen_checksum,
            allow_window_ops: self.allow_window_ops,
            allow_title_report: self.allow_title_report,
            colors: self.colors.as_deref(),
            color_scheme: self.color_scheme,
        }
    }
}

/// Start a tab's shell in a PTY of `size`.
pub fn spawn(size: GridSize, options: &SpawnOptions) -> anyhow::Result<(Pty, PtyEvents)> {
    let launch = process::launch_options(options, process::shell(options));
    Pty::start(size, &options.start_options(), launch)
}

/// [`spawn`] in a new session host (`wormhole-host`): the same program, environment and engine
/// options, with the program running in `host` instead of the app's own process. The saved text
/// (`options.restore`) is not replayed: a hosted terminal's history is the host's. Blocks for up
/// to a few seconds while the host starts: never on the UI thread.
pub fn spawn_hosted(
    host: &wormhole_term::client::HostCommand<'_>,
    size: GridSize,
    options: &SpawnOptions,
) -> anyhow::Result<(Pty, PtyEvents)> {
    let launch = process::launch_options(options, process::shell(options));
    Pty::start_hosted(host, size, &options.start_options(), launch)
}

/// Attach to the running session host `id` listening at `socket`, with the engine options the
/// terminal was started with (`options`, as [`spawn_hosted`] had them), fed a repaint at once when
/// `visible` and headless otherwise. Blocks for the handshake: never on the UI thread.
pub fn attach_hosted(
    socket: &std::path::Path,
    id: &str,
    options: &SpawnOptions,
    visible: bool,
) -> anyhow::Result<(Pty, PtyEvents)> {
    Pty::attach_hosted(socket, id, &options.start_options(), visible)
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
    use wormhole_term::GridSize;

    use crate::terminal::process;

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
    use crate::config::profile::{AgentLogin, PROFILE_VAR, Profile};
    use crate::storage::paths::ProfileId;
    use crate::view::sidebar::GroupColor;
    use wormhole_term::GridSize;

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
