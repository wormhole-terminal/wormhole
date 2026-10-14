//! The shell process behind a terminal: what it runs, with which environment, and
//! the per-process queries (current folder) the UI polls.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use alacritty_terminal::tty;

use super::SpawnOptions;
use crate::shells::{self, ShellCommand, program_name};

/// The settings that make a process one instance of Wormhole: where it keeps its data, what it
/// is told by devctl, whether it stays in the background, how it answers Touch ID and
/// notifications, what it runs for a shell, where it starts, which git and `gh`, agent home and
/// agent folders it uses, where it looks for updates and which folders its command line tool goes in.
/// A tab's shell inherits none of them (`launch`). (`WORMHOLE_TAB_ID`, `WORMHOLE_SOCKET` and
/// `WORMHOLE_BIN` are the tab's own and are set in `tty_options`.)
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
    crate::cli_tool::DIR_VAR,
    crate::cli_tool::LOGIN_PATH_VAR,
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

/// The `PATH` a process gets from `launchd` (the Finder's), for an app that has none at all.
const FALLBACK_PATH: &str = "/usr/bin:/bin:/usr/sbin:/sbin";

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
    tty_options_with(options, shell, crate::cli_tool::terminal_dir().as_deref())
}

/// [`tty_options`] with the folder of the `wh` command given: put first on the shell's `PATH` so the
/// command works in every terminal Wormhole starts. `None` when the script isn't there (its write
/// at launch failed): the terminal starts all the same, without the command.
fn tty_options_with(options: &SpawnOptions, shell: tty::Shell, command_dir: Option<&Path>) -> tty::Options {
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
        env.insert("PATH".into(), crate::cli_tool::path_with_first(dir, &base));
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

/// How long [`close`] waits for output before it looks at the clock and the groups again; also
/// how late the dup of the PTY's master is let go once the shell is reaped.
const WATCH_STEP: Duration = Duration::from_millis(100);

/// How long [`close`] keeps watching after the SIGKILL. A process that is still there by then is
/// stuck in the kernel, where nothing it is sent or read can help it.
const WATCH_AFTER_KILL: Duration = Duration::from_secs(10);

/// End a closed tab's shell and the program running in it. alacritty's PTY handle, dropped once
/// its event loop thread has ended, sends the shell one SIGHUP and then waits for it, without
/// escalating, and two things kept that wait from ever ending:
///
/// - A shell or program that ignores SIGHUP (`trap '' HUP`, `nohup`) kept running. The shell's
///   process group (it leads one) gets the hangup, and SIGKILL when anything is still there
///   after `grace` ([`HANGUP_GRACE`]; a parameter so a test need not wait seconds). A shell with
///   job control runs the program in the foreground in a group of its own, which the shell's
///   group never reaches, so the terminal's foreground group gets both signals too. While the
///   shell lives that group is asked for (`tcgetpgrp` on the master) right before each signal:
///   the kernel only reports a group of this terminal's own session, so a group id that was
///   reused meanwhile cannot be hit. A shell that goes at the hangup takes the terminal's answer
///   with it, so the group from the hangup is then killed after `grace` if it is still there: a
///   group id is not given out again while the group has members, and the watch checks every
///   [`WATCH_STEP`] that it has.
/// - Nobody reads the PTY any more, and a session leader that exits waits in the kernel for its
///   terminal's output to drain: with the master open and unread that never ends (the shell sits
///   in state E), and a program that prints blocks on the full buffer. So `master`, a dup of the
///   PTY's master ([`drain_handle`]), is read and thrown away until the shell is reaped. The dup
///   is let go then, since an open master keeps the terminal from hanging up.
///
/// `pid` is `None` once the shell has exited (its pid may belong to someone else by then).
pub(super) fn close<T: Send + 'static>(
    event_loop: JoinHandle<T>,
    pid: Option<u32>,
    master: Option<File>,
    grace: Duration,
) {
    let Some(pid) = pid else { return };
    let mut foreground = signal_session(pid, master.as_ref(), libc::SIGHUP);
    let reaped = Arc::new(AtomicBool::new(false));
    let watchdog = {
        let reaped = reaped.clone();
        move || {
            let mut master = master;
            let started = Instant::now();
            let mut killed = false;
            loop {
                let shell_reaped = reaped.load(Ordering::Acquire);
                if shell_reaped {
                    master = None;
                }
                foreground = foreground.filter(|&group| group_exists(group));
                let waited = started.elapsed();
                if (shell_reaped && foreground.is_none()) || waited >= grace + WATCH_AFTER_KILL {
                    return;
                }
                if !killed && waited >= grace {
                    killed = true;
                    if !shell_reaped {
                        // Until the wait in the closer returns the shell's pid can't be reused,
                        // so this can't reach anyone else's processes.
                        signal_session(pid, master.as_ref(), libc::SIGKILL);
                    }
                    if let Some(group) = foreground {
                        signal_group(group, libc::SIGKILL);
                    }
                }
                let step = if killed { WATCH_STEP } else { WATCH_STEP.min(grace.saturating_sub(waited)) };
                // A hung-up terminal has nothing left to read (and polls as ready at once).
                if !master.as_ref().is_some_and(|master| discard_output(master, step)) {
                    std::thread::sleep(step);
                }
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

/// A second handle on the PTY's master for [`close`] to drain and to ask for the foreground
/// group. Taken while the terminal runs, because the master's descriptor number may belong to
/// something else once alacritty has dropped its PTY. Non-blocking, so a read never waits for
/// output (alacritty sets the same flag on the master, which the handle shares).
pub(super) fn drain_handle(master: &File) -> Option<File> {
    let kept = master.try_clone().and_then(|handle| set_nonblocking(&handle).map(|()| handle));
    kept.inspect_err(|err| tracing::warn!("could not keep a handle on the PTY: {err}")).ok()
}

#[allow(unsafe_code)] // libc FFI; the descriptor is open as long as `file` is.
fn set_nonblocking(file: &File) -> std::io::Result<()> {
    let fd = file.as_raw_fd();
    // SAFETY: fcntl with F_GETFL and F_SETFL only reads and writes the descriptor's status flags.
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

/// Read and throw away what the terminal prints, waiting up to `wait` for it. `false` when
/// there will be nothing more: the terminal hung up or could not be polled.
#[allow(unsafe_code)] // libc FFI; the pollfd lives through the call.
fn discard_output(master: &File, wait: Duration) -> bool {
    let mut poll = libc::pollfd { fd: master.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    // Rounded up: a wait of under a millisecond is still a wait, not a spin.
    let millis = libc::c_int::try_from(wait.as_millis() + 1).unwrap_or(libc::c_int::MAX);
    // SAFETY: one valid pollfd, and the count says so.
    let ready = unsafe { libc::poll(&mut poll, 1, millis) };
    if ready < 0 {
        return std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted;
    }
    if ready == 0 {
        return true;
    }
    let mut scratch = [0u8; 16 * 1024];
    match (&*master).read(&mut scratch) {
        Ok(read) => read > 0,
        Err(err) => matches!(err.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted),
    }
}

/// Signal the shell's process group and the terminal's foreground group, when that is another;
/// the foreground group is returned. It is read first, while the shell can still be asked.
fn signal_session(pid: u32, master: Option<&File>, signal: libc::c_int) -> Option<libc::pid_t> {
    let shell = libc::pid_t::try_from(pid).ok()?;
    let foreground = master.and_then(foreground_group).filter(|&group| group != shell);
    signal_group(shell, signal);
    if let Some(group) = foreground {
        signal_group(group, signal);
    }
    foreground
}

/// The group in the foreground of the terminal whose master this is, if it says.
#[allow(unsafe_code)] // libc FFI; the descriptor is open as long as `master` is.
fn foreground_group(master: &File) -> Option<libc::pid_t> {
    // SAFETY: tcgetpgrp only reads the terminal's state.
    let group = unsafe { libc::tcgetpgrp(master.as_raw_fd()) };
    // Not 0, 1 or below: killpg(0) would be our own group.
    (group > 1).then_some(group)
}

/// Whether a process group still has a member (`killpg` with no signal), a zombie included.
#[allow(unsafe_code)] // libc FFI; the arguments are plain integers.
fn group_exists(pgid: libc::pid_t) -> bool {
    // SAFETY: signal 0 only checks that the group can be signalled.
    let sent = unsafe { libc::killpg(pgid, 0) } == 0;
    // EPERM: it exists, and is not ours to signal.
    sent || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[allow(unsafe_code)] // libc FFI; the arguments are plain integers.
fn signal_group(pgid: libc::pid_t, signal: libc::c_int) {
    if pgid <= 1 {
        return;
    }
    // SAFETY: killpg only sends a signal; a group that no longer exists fails with ESRCH.
    unsafe {
        libc::killpg(pgid, signal);
    }
}

#[cfg(test)]
mod name_tests {
    use super::{
        ENV, INSTANCE_VARS, ShellCommand, SpawnOptions, command, tty, tty_options, tty_options_with,
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
        let env = tty_options(&options, tty::Shell::new("/bin/sh".into(), Vec::new())).env;
        for (name, old) in crate::ipc::LEGACY_TAB_VARS {
            assert_eq!(env.get(*old), env.get(*name), "{old}");
            assert!(env.contains_key(*name) || *name == "WORMHOLE_BIN", "{name}");
        }
        assert_eq!(env.get("ANDROMEDA_TAB_ID").map(String::as_str), Some("7"));
    }

    fn env_with_command_dir(options: &SpawnOptions, dir: Option<&std::path::Path>) -> Env {
        tty_options_with(options, tty::Shell::new("/bin/sh".into(), Vec::new()), dir).env
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
        let failed = crate::cli_tool::keep_current_in(
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
