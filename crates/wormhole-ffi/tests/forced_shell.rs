//! A forced shell runs in a terminal that was opened with a shell of its own, on a real PTY: the Swift app hands
//! its `--shell` to every start as the request's program (and sets core's `WORMHOLE_SHELL`, which replaces every
//! choice once more where the program starts, `terminal::process::shell`). It is what keeps a test instance started
//! with `/bin/cat` from ever running a real shell. The variable itself is the process's and is not set here.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_ffi::{HostedStart, Terminal, TerminalListener};

struct Quiet;

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

#[test]
fn a_forced_shell_runs_in_a_terminal_opened_with_a_shell_of_its_own() {
    let dir = tempfile::Builder::new().prefix("Wormhole-swift-forced-").tempdir_in("/tmp").expect("scratch");
    let forced = dir.path().join("forced");
    std::fs::write(&forced, "#!/bin/sh\nprintf 'forced-ran\\n'\nexec cat\n").expect("script");
    std::fs::set_permissions(&forced, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("executable");

    // Opened with a shell of its own, as New Terminal with ▸ sh does: a program's path is a `terminal::shells`
    // key, which without the forced one starts `/bin/sh -l` (a prompt, and never the script's line).
    let what = HostedStart {
        tab: 7,
        shell: Some("/bin/sh".into()),
        program: Some(forced.display().to_string()),
        args: Vec::new(),
        cwd: None,
        cols: 60,
        rows: 5,
        cell_width: 8.0,
        cell_height: 16.0,
        old_host: None,
        saved_text: None,
        profile: None,
    };
    let terminal = Terminal::start_planned(what, Arc::new(Quiet)).expect("starts");
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        terminal.frame();
        let text = terminal.screen_text();
        if text.contains("forced-ran") {
            break;
        }
        assert!(Instant::now() < until, "the terminal's own shell ran, not the forced one; screen:\n{text}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
