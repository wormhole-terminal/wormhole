//! The shell process behind a terminal: what it runs, with which environment, and
//! the per-process queries (current folder) the UI polls.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use alacritty_terminal::tty;

use super::SpawnOptions;
use crate::shells::{self, ShellCommand, program_name};

/// The settings that make a process one instance of Wormhole: where it keeps its data, what it
/// is told by devctl, whether it stays in the background, how it answers Touch ID and
/// notifications, what it runs for a shell, where it starts, which git and `gh`, agent home and
/// agent folders it uses and where it looks for updates. A tab's shell inherits none of them
/// (`launch`). (`WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and `WORMHOLE_BIN` are the tab's own
/// and are set in `tty_options`.)
pub(super) const INSTANCE_VARS: [&str; 12] = [
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
];

/// What a tab runs. WORMHOLE_SHELL replaces everything (e.g. `/bin/cat` for safe automated UI
/// tests, where typed text is echoed but never executed). Otherwise the program the app decided
/// on (`options.shell`: the tab's profile, a trusted project's shell or the default, see
/// `shells::for_tab`), else the user's login shell.
pub(super) fn shell(options: &SpawnOptions) -> tty::Shell {
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
        Ok(exe) if options.welcome => crate::welcome::command(&exe, program, args),
        _ => (program, args),
    };
    launch(program, args)
}

/// The program `options` ask for, the login shell without one.
fn command(options: &SpawnOptions) -> ShellCommand {
    options.shell.clone().unwrap_or_else(|| ShellCommand::login(&shells::login_shell()))
}

/// The program that clears variables and then becomes the shell.
const ENV: &str = "/usr/bin/env";

/// `program` as the PTY's shell, started so that none of [`INSTANCE_VARS`] reaches it. alacritty
/// can add variables to a child's environment but not remove one it inherits, and the app itself
/// reads them all the time, so they can't be removed from its own environment either: the shell
/// is started through `env -u …`, which replaces itself with the shell (same pid, same process
/// group, so agent detection and the foreground program are unaffected).
pub(super) fn launch(program: String, args: Vec<String>) -> tty::Shell {
    let (program, args) = without_instance_vars(program, args, std::path::Path::new(ENV).exists());
    tty::Shell::new(program, args)
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

/// The PTY options for running `shell`: folder and environment.
pub(super) fn tty_options(options: &SpawnOptions, shell: tty::Shell) -> tty::Options {
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
    // `escape_args` only exists on Windows, hence the struct update.
    #[allow(clippy::needless_update)]
    tty::Options {
        shell: Some(shell),
        working_directory: cwd.clone().or_else(|| std::env::var_os("HOME").map(PathBuf::from)),
        drain_on_exit: true,
        env,
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // libproc FFI; buffer sizes are checked below.
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_vnodepathinfo>() as i32;
    // SAFETY: `info` is a correctly sized, writable buffer for this flavor.
    let written = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut info as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if written != size {
        return None;
    }
    // SAFETY: the kernel NUL-terminates `vip_path`.
    let path = unsafe { std::ffi::CStr::from_ptr(info.pvi_cdir.vip_path.as_ptr().cast()) };
    Some(PathBuf::from(path.to_string_lossy().into_owned()))
}

#[cfg(target_os = "linux")]
pub(super) fn process_cwd(pid: u32) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn process_cwd(_pid: u32) -> Option<PathBuf> {
    None
}

/// Whether `host`, as a shell names it in OSC 7 (`file://host/path`), is this machine: no host,
/// `localhost`, the loopback addresses, or this machine's host name. Short names are compared,
/// case-insensitively: macOS reports `Studio.local` where a shell may say `studio`.
pub(super) fn is_this_host(host: &str) -> bool {
    if host.is_empty() || host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    let short = |name: &str| name.split('.').next().unwrap_or("").to_ascii_lowercase();
    hostname().is_some_and(|ours| short(ours) == short(host))
}

/// This machine's host name, read once.
fn hostname() -> Option<&'static str> {
    static HOSTNAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    HOSTNAME.get_or_init(read_hostname).as_deref()
}

#[allow(unsafe_code)] // libc FFI; the buffer's length is passed along with it.
fn read_hostname() -> Option<String> {
    // Longer than any host name POSIX allows, and zeroed: a name that fills it has no NUL.
    let mut buf = [0u8; 256];
    // SAFETY: `buf` is writable for the length given; gethostname writes at most that much.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8(buf[..end].to_vec()).ok().filter(|name| !name.is_empty())
}

/// How long a closed tab's shell gets to exit before it is killed.
pub(super) const HANGUP_GRACE: Duration = Duration::from_secs(3);

/// End a closed tab's shell. alacritty's PTY handle, dropped once its event
/// loop thread has ended, sends the shell one SIGHUP and then waits for it,
/// without escalating: a shell or program that ignores SIGHUP (`trap '' HUP`,
/// `nohup`) kept running, and kept that thread and the PTY alive with it. The
/// shell's process group (it leads one) gets the hangup, and SIGKILL when
/// anything is still there after `grace` ([`HANGUP_GRACE`]; a parameter so a
/// test need not wait seconds). `pid` is `None` once the shell has exited (its
/// pid may belong to someone else by then).
pub(super) fn close<T: Send + 'static>(event_loop: JoinHandle<T>, pid: Option<u32>, grace: Duration) {
    let Some(pid) = pid else { return };
    signal_group(pid, libc::SIGHUP);
    let reaped = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let reaped = reaped.clone();
        move || {
            std::thread::sleep(grace);
            // Until the wait below returns the shell's pid can't be reused, so
            // this can't reach anyone else's processes.
            if !reaped.load(Ordering::Acquire) {
                signal_group(pid, libc::SIGKILL);
            }
        }
    };
    let closer = move || {
        // Dropping what the loop returns drops alacritty's PTY: SIGHUP, then wait.
        let _ = event_loop.join();
        reaped.store(true, Ordering::Release);
    };
    let spawned = std::thread::Builder::new()
        .name("pty-watchdog".into())
        .spawn(watchdog)
        .and_then(|_| std::thread::Builder::new().name("pty-close".into()).spawn(closer));
    if let Err(err) = spawned {
        tracing::warn!("could not watch a closed tab's shell (pid {pid}): {err}");
    }
}

#[allow(unsafe_code)] // libc FFI; the arguments are plain integers.
fn signal_group(pid: u32, signal: libc::c_int) {
    let Ok(pgid) = libc::pid_t::try_from(pid) else { return };
    // SAFETY: killpg only sends a signal; a group that no longer exists fails with ESRCH.
    unsafe {
        libc::killpg(pgid, signal);
    }
}

#[cfg(test)]
mod name_tests {
    use super::{
        ENV, INSTANCE_VARS, ShellCommand, SpawnOptions, command, tty, tty_options, without_instance_vars,
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
        let env = tty_options(&options, tty::Shell::new("/bin/sh".into(), Vec::new())).env;
        for (name, old) in crate::ipc::LEGACY_TAB_VARS {
            assert_eq!(env.get(*old), env.get(*name), "{old}");
            assert!(env.contains_key(*name) || *name == "WORMHOLE_BIN", "{name}");
        }
        assert_eq!(env.get("ANDROMEDA_TAB_ID").map(String::as_str), Some("7"));
    }

    #[test]
    fn the_tabs_own_variables_are_not_cleared() {
        for own in ["WORMHOLE_TAB_ID", "WORMHOLE_SOCKET", "WORMHOLE_BIN"] {
            assert!(!INSTANCE_VARS.contains(&own), "{own} is set for the tab in tty_options");
        }
    }

    #[test]
    fn a_folder_counts_as_here_when_its_host_is_this_machine() {
        for local in ["", "localhost", "LOCALHOST", "127.0.0.1", "::1"] {
            assert!(super::is_this_host(local), "{local:?}");
        }
        assert!(!super::is_this_host("build-server.example.com"));
        assert!(!super::is_this_host("elsewhere"));
        if let Some(ours) = super::hostname() {
            assert!(super::is_this_host(ours));
            assert!(super::is_this_host(&ours.to_ascii_uppercase()));
            let short = ours.split('.').next().unwrap_or(ours);
            assert!(super::is_this_host(&format!("{short}.local")), "short names match");
        }
    }
}
