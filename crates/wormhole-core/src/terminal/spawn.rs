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
