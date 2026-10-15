//! The PTY layer on real pseudo-terminals, with small programs (`/bin/echo`, `/bin/cat`,
//! `/bin/sh -c …`, never the user's shell) and a screen that is only the bytes it was given.
//! `process`: starting, the environment, exit; `terminal`: what the program sees of its terminal;
//! `flood`: input and output at volume; `measure`: numbers behind the layer's constants.

mod flood;
mod measure;
mod process;
mod terminal;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{Control, FairLock, Feed, Launch, Shell, start};
use crate::terminal::WindowSize;
use crate::terminal::events::{EventProxy, PtyEvent, PtyEvents};

pub(super) const SIZE: WindowSize =
    WindowSize { num_lines: 24, num_cols: 80, cell_width: 8, cell_height: 16 };

/// A screen that keeps every byte it is fed.
struct Collect;

impl Feed<Vec<u8>> for Collect {
    fn feed(&mut self, screen: &mut Vec<u8>, bytes: &[u8]) {
        screen.extend_from_slice(bytes);
    }
}

/// A terminal started by [`run`].
pub(super) struct Run {
    pub(super) control: Control,
    pub(super) screen: Arc<FairLock<Vec<u8>>>,
    pub(super) events: PtyEvents,
    pub(super) pid: u32,
}

pub(super) fn launch(program: &str, args: &[&str]) -> Launch {
    Launch {
        shell: Shell::new(program.into(), args.iter().map(|arg| arg.to_string()).collect()),
        cwd: None,
        env: HashMap::new(),
    }
}

pub(super) fn run(program: &str, args: &[&str]) -> Run {
    run_launch(&launch(program, args), SIZE)
}

pub(super) fn run_launch(launch: &Launch, size: WindowSize) -> Run {
    let (proxy, events) = EventProxy::new();
    let screen = Arc::new(FairLock::new(Vec::new()));
    let started = start(launch, size, screen.clone(), Collect, proxy).expect("starts");
    Run { control: started.control, screen, events, pid: started.pid }
}

impl Run {
    /// What the program printed so far.
    pub(super) fn text(&self) -> String {
        String::from_utf8_lossy(&self.screen.lock()).into_owned()
    }

    pub(super) fn wait_for(&self, needle: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let text = self.text();
            if text.contains(needle) {
                return text;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {needle:?}; got {text:?}");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// The exit the terminal reported: `Some(code)` for an exit, `Some(None)` for one without a
    /// code (a signal), `None` when nothing was reported within `within`.
    pub(super) fn exit(&self, within: Duration) -> Option<Option<i32>> {
        let deadline = Instant::now() + within;
        loop {
            match self.events.try_recv() {
                Some(PtyEvent::Exited(code)) => return Some(code),
                Some(_) => {}
                None if Instant::now() >= deadline => return None,
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}

/// `kill` for a test that started a process of its own, without unsafe code.
pub(super) fn kill(pid: &str) {
    let _ = std::process::Command::new("kill").args(["-9", pid]).status();
}
