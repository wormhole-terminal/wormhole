//! Starting a program and ending it: output, exit codes, the folder, the environment, signals and
//! descriptors.

use std::sync::Arc;
use std::time::Duration;

use super::{Collect, Run, SIZE, kill, launch, run, run_launch};
use crate::terminal::events::EventProxy;
use crate::terminal::pty::{FairLock, start};

#[test]
fn a_program_s_output_is_read_and_its_exit_reported() {
    let run = run("/bin/echo", &["hello"]);
    // The line discipline turns the newline into a carriage return and a newline.
    run.wait_for("hello\r\n");
    assert_eq!(run.exit(Duration::from_secs(10)), Some(Some(0)));
    assert_eq!(run.text(), "hello\r\n");
}

#[test]
fn the_exit_code_comes_with_the_exit() {
    let run = run("/bin/sh", &["-c", "exit 7"]);
    assert_eq!(run.exit(Duration::from_secs(10)), Some(Some(7)));
}

#[test]
fn a_program_killed_by_a_signal_exits_without_a_code() {
    let run = run("/bin/sh", &["-c", "kill -9 $$"]);
    assert_eq!(run.exit(Duration::from_secs(10)), Some(None));
}

#[test]
fn a_program_that_ends_is_not_waited_for_to_close_its_terminal() {
    // The shell exits at once while a background job keeps the slave open, so the master never
    // reports the end of the output: only the child's exit says it is over.
    let dir = tempfile::tempdir().expect("temp dir");
    let pid_file = dir.path().join("job.pid");
    let script = format!("sleep 20 & echo $! > {}; exit 5", pid_file.display());
    let run = run("/bin/sh", &["-c", &script]);
    let reported = run.exit(Duration::from_secs(5));
    let job = std::fs::read_to_string(&pid_file).unwrap_or_default();
    kill(job.trim());
    assert_eq!(reported, Some(Some(5)));
}

#[test]
fn the_exit_is_reported_after_the_last_of_its_output_is_in_the_screen() {
    let run = run("/bin/sh", &["-c", "i=0; while [ $i -lt 2000 ]; do echo line $i; i=$((i+1)); done"]);
    assert_eq!(run.exit(Duration::from_secs(10)), Some(Some(0)));
    assert!(
        run.text().ends_with("line 1999\r\n"),
        "{:?}",
        &run.text()[run.text().len().saturating_sub(40)..]
    );
}

#[test]
fn a_program_that_cannot_start_is_an_error() {
    let (proxy, _events) = EventProxy::new();
    let screen = Arc::new(FairLock::new(Vec::new()));
    let result = start(&launch("/no/such/program", &[]), SIZE, screen, Collect, proxy);
    assert!(result.is_err());
}

#[test]
fn names_that_cannot_be_passed_to_a_program_are_errors_not_panics() {
    // A NUL ends a C string; a program, an argument, a variable or a folder from a project file
    // can have one in it.
    let try_start = |launch: &crate::terminal::pty::Launch| {
        let (proxy, _events) = EventProxy::new();
        start(launch, SIZE, Arc::new(FairLock::new(Vec::new())), Collect, proxy).map(|_| ())
    };
    assert!(try_start(&launch("/bin/echo\0", &[])).is_err());
    assert!(try_start(&launch("/bin/echo", &["a\0b"])).is_err());
    let mut env = launch("/bin/echo", &[]);
    env.env.insert("A\0B".into(), "x".into());
    assert!(try_start(&env).is_err());
    env.env.clear();
    env.env.insert("NAME".into(), "va\0lue".into());
    assert!(try_start(&env).is_err());
    // A folder with one cannot be entered, which is no reason not to start (see `Launch::cwd`).
    let mut folder = launch("/bin/echo", &["started"]);
    folder.cwd = Some("/tmp\0/x".into());
    run_launch(&folder, SIZE).wait_for("started");
}

#[test]
fn the_program_starts_in_the_folder_asked_for_or_where_the_app_is() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut asked = launch("/bin/sh", &["-c", "pwd -P"]);
    asked.cwd = Some(dir.path().to_path_buf());
    let wanted = std::fs::canonicalize(dir.path()).expect("exists");
    run_launch(&asked, SIZE).wait_for(&format!("{}\r\n", wanted.display()));

    // A folder that was deleted meanwhile does not keep the terminal from opening.
    let mut gone = launch("/bin/sh", &["-c", "pwd -P"]);
    gone.cwd = Some(dir.path().join("deleted"));
    let here = std::fs::canonicalize(std::env::current_dir().expect("a folder")).expect("exists");
    run_launch(&gone, SIZE).wait_for(&format!("{}\r\n", here.display()));
}

#[test]
#[allow(unsafe_code)]
fn the_program_gets_default_signal_handling_whatever_the_app_ignores_or_blocks() {
    // SIGUSR1 ignored and SIGUSR2 blocked in the thread that starts the program: both stay so
    // across fork and exec unless the child resets them. A shell that kills itself with each
    // dies of it only when they were reset. Nothing else in the test binary uses these signals.
    // SAFETY: plain signal(3) and pthread_sigmask(3) calls; the dispositions and the mask are
    // put back before the test ends.
    let (mut blocked, mut before): (libc::sigset_t, libc::sigset_t) =
        unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let run = unsafe {
        libc::signal(libc::SIGUSR1, libc::SIG_IGN);
        libc::sigemptyset(&mut blocked);
        libc::sigaddset(&mut blocked, libc::SIGUSR2);
        libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, &mut before);
        let ignored = run("/bin/sh", &["-c", "kill -USR1 $$; echo survived-usr1"]);
        let masked = run("/bin/sh", &["-c", "kill -USR2 $$; echo survived-usr2"]);
        libc::pthread_sigmask(libc::SIG_SETMASK, &before, std::ptr::null_mut());
        libc::signal(libc::SIGUSR1, libc::SIG_DFL);
        (ignored, masked)
    };
    let (ignored, masked) = run;
    assert_eq!(ignored.exit(Duration::from_secs(10)), Some(None), "SIGUSR1 killed it");
    assert_eq!(masked.exit(Duration::from_secs(10)), Some(None), "SIGUSR2 killed it");
    assert!(!ignored.text().contains("survived") && !masked.text().contains("survived"));
}

#[test]
fn the_program_inherits_no_descriptor_but_its_terminal() {
    // The masters and pollers of this and every other terminal are close-on-exec; the program
    // has its three standard streams, and `ls` the descriptor it lists the directory with.
    let others: Vec<Run> = (0..3).map(|_| run("/bin/cat", &[])).collect();
    let text = run("/bin/sh", &["-c", "ls /dev/fd; echo listed"]).wait_for("listed");
    let fds: Vec<&str> = text.lines().take_while(|line| *line != "listed").map(str::trim).collect();
    assert!(fds.len() <= 4, "descriptors in the program: {fds:?}");
    drop(others);
}

#[test]
fn the_environment_is_the_apps_plus_what_is_asked_for() {
    let mut asked = launch("/bin/sh", &["-c", "echo [$FROM_THE_TEST] [$PATH]"]);
    asked.env.insert("FROM_THE_TEST".into(), "yes".into());
    let text = run_launch(&asked, SIZE).wait_for("[yes] [/");
    assert!(text.contains(&std::env::var("PATH").unwrap_or_default()), "{text}");
}

#[test]
fn a_program_that_is_gone_before_the_thread_starts_is_still_reported() {
    // `true` exits before the reader has even registered the child with the poller.
    for _ in 0..20 {
        let run = run("/usr/bin/true", &[]);
        assert_eq!(run.exit(Duration::from_secs(10)), Some(Some(0)));
    }
}
