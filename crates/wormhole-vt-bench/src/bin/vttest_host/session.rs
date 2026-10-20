//! One vttest process on a pseudo-terminal, with the engine as its terminal.
//!
//! `pty_relay.py` keeps the PTY (the Rust side needs no unsafe code); this side feeds what vttest
//! writes into the engine, writes the engine's replies back at once (vttest waits only a tenth of
//! a second for them, a fortieth in the build `fetch-suites.sh` makes) and says what vttest is doing:
//! holding for a key, asking for a menu choice, or quiet. Every wait has a limit, so a screen that
//! needs a human ends the walk and never hangs it.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use wormhole_vt::input::{InputModes, Key, KeyEvent, encode_key};
use wormhole_vt::{Line, Terminal};
use wormhole_vt_bench::window_host::Host;

/// What vttest is doing, as far as its output says.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    /// `Push <RETURN>`: a test screen is complete.
    Hold,
    /// `Enter choice number (0 - N):`.
    Menu,
    /// `Bad choice, try again:`.
    BadChoice,
    /// Nothing written for the quiet time and no prompt vttest prints itself: a screen that asks for
    /// a key or a human.
    Quiet,
    /// vttest ended.
    Closed,
    /// The overall limit passed.
    TimedOut,
}

pub struct Session {
    child: Child,
    to_child: ChildStdin,
    from_child: Receiver<Vec<u8>>,
    host: Host,
    /// The last bytes vttest wrote, to see which prompt it ended on.
    tail: Vec<u8>,
    started: Instant,
    deadline: Instant,
    /// Every chunk and reply, when asked for (`--trace`).
    trace: bool,
}

/// How long one walk may take, however many screens it has.
const OVERALL: Duration = Duration::from_secs(120);
/// The prompt `holdit` writes before it reads a line.
const HOLD: &[u8] = b"Push <RETURN>";
const MENU: &[u8] = b"Enter choice number (0 - ";
const BAD: &[u8] = b"Bad choice, try again: ";

impl Session {
    /// Start `vttest` (the binary) at 80x24 in a scratch folder (it writes a log there when asked).
    pub fn start(vttest: &Path, relay: &Path, scratch: &Path, trace: bool) -> Result<Session, String> {
        std::fs::create_dir_all(scratch).map_err(|e| e.to_string())?;
        let mut child = Command::new("python3")
            .current_dir(scratch)
            .arg("-I")
            .arg(relay)
            .args(["24", "80", "--"])
            .arg(vttest)
            // 24 lines, 80 columns, 132 the widest: what the DECCOLM tests may switch to.
            .arg("24x80.132")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("TERM", "xterm")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start python3: {e}"))?;
        let to_child = child.stdin.take().ok_or("no stdin")?;
        let mut from = child.stdout.take().ok_or("no stdout")?;
        let (sender, from_child) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 64 * 1024];
            while let Ok(n) = from.read(&mut buf) {
                if n == 0 || sender.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        // The engine as the esctest host has it: window operations on (DECCOLM resizes the window),
        // the screen checksum on, the same fake window.
        let host = Host::new(80, 24, true, true);
        Ok(Session {
            child,
            to_child,
            from_child,
            host,
            tail: Vec::new(),
            started: Instant::now(),
            deadline: Instant::now() + OVERALL,
            trace,
        })
    }

    pub fn term(&self) -> &Terminal {
        self.host.term()
    }

    /// Type `text` (a menu choice, a line) into vttest.
    pub fn send(&mut self, text: &str) {
        if self.trace {
            eprintln!("{:>6} -> {text:?}", self.started.elapsed().as_millis());
        }
        // A closed pipe is seen by the next `wait` as `Closed`.
        let _ = self.to_child.write_all(text.as_bytes()).and_then(|()| self.to_child.flush());
        self.tail.clear();
    }

    /// Press a key as the engine's keyboard encoder does for the terminal's modes now.
    pub fn press(&mut self, key: Key) {
        let modes = InputModes::from(self.term().modes());
        let mut bytes = Vec::new();
        encode_key(&KeyEvent::new(key), &modes, &mut bytes);
        self.send(&String::from_utf8_lossy(&bytes));
    }

    /// Feed what vttest writes into the engine, answering it, until it reaches a prompt, goes
    /// quiet for `quiet`, or ends.
    pub fn wait(&mut self, quiet: Duration) -> State {
        loop {
            let left = self.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return State::TimedOut;
            }
            match self.from_child.recv_timeout(quiet.min(left)) {
                Ok(chunk) => {
                    if self.trace {
                        eprintln!(
                            "{:>6} <- {:?}",
                            self.started.elapsed().as_millis(),
                            String::from_utf8_lossy(&chunk)
                        );
                    }
                    let reply = self.host.feed(&chunk);
                    if !reply.is_empty() {
                        if self.trace {
                            eprintln!(
                                "{:>6} -> {:?}",
                                self.started.elapsed().as_millis(),
                                String::from_utf8_lossy(&reply)
                            );
                        }
                        let _ = self.to_child.write_all(&reply).and_then(|()| self.to_child.flush());
                    }
                    self.tail.extend_from_slice(&chunk);
                    if self.tail.len() > 256 {
                        let cut = self.tail.len() - 128;
                        self.tail.drain(..cut);
                    }
                    if let Some(state) = self.prompt() {
                        return state;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if Instant::now() >= self.deadline {
                        return State::TimedOut;
                    }
                    return State::Quiet;
                }
                Err(RecvTimeoutError::Disconnected) => return State::Closed,
            }
        }
    }

    /// The prompt the last bytes end on. vttest writes each prompt last before it reads, so the
    /// screen is complete the moment it is there.
    fn prompt(&self) -> Option<State> {
        let tail = self.tail.as_slice();
        if tail.ends_with(HOLD) {
            return Some(State::Hold);
        }
        if tail.ends_with(BAD) {
            return Some(State::BadChoice);
        }
        // "Enter choice number (0 - 11): ": the highest choice, a parenthesis and a colon follow.
        if let Some(text) = tail.strip_suffix(b"): ") {
            let digits = text.iter().rev().take_while(|b| b.is_ascii_digit()).count();
            if digits > 0 && text[..text.len() - digits].ends_with(MENU) {
                return Some(State::Menu);
            }
        }
        None
    }

    /// The screen as lines: the text of each row, trailing blanks cut.
    pub fn rows(&self) -> Vec<String> {
        let term = self.term();
        (0..term.rows()).map(|row| term.row_text(Line(row as i32), true)).collect()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
