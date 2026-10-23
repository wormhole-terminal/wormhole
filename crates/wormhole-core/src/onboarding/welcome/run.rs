//! `Wormhole welcome [-- <shell argv…>]`: the hands of the program. It puts the terminal in raw mode,
//! feeds `driver` the keys and the app's answers, writes what it says to draw, and at the end clears the
//! screen, prints the banner and becomes the shell (`exec`, where the terminal started), so the terminal
//! that showed the welcome goes on as the person's own, the same process.
//!
//! It never traps the terminal: whatever goes wrong (no terminal, no raw mode, a panic) the shell is
//! started or the program ends with the terminal as it was.

use std::io::{self, Write as _};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::banner;
use super::driver::{Answer, Driver};
use super::gate::TOKEN_VAR;
use super::render::Size;
use crate::ipc::control::{Action, Request, WelcomeOp, exchange};
use crate::platform::process::{self, exec};
use crate::platform::tty::{self, RawMode};

pub const HELP: &str = "\
Wormhole welcome: the first-run welcome, in this terminal

  Wormhole welcome [-- <program> [<args>…]]

Walks through how it looks, what should reach you, the sidebar with your coding agents and what else is
here, then clears the screen and starts <program> (your shell) in this terminal. Without <program> it
only shows the pages. Run by Wormhole itself for the first terminal; Help → Welcome to Wormhole opens
it again.
";

/// What the command line asked for.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// The program to become at the end (after `--`), with its arguments.
    pub shell: Option<Vec<String>>,
    pub help: bool,
}

/// Read the words after `welcome`.
pub fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args { shell: None, help: false };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--" => {
                let shell: Vec<String> = rest.by_ref().cloned().collect();
                if shell.first().is_none_or(|program| program.is_empty()) {
                    return Err("`--` needs the program to start after the welcome".to_string());
                }
                parsed.shell = Some(shell);
            }
            "-h" | "--help" | "help" => parsed.help = true,
            other => return Err(format!("unknown option {other:?}")),
        }
    }
    Ok(parsed)
}

/// How to reach the app that started this terminal for the welcome.
struct Link {
    socket: PathBuf,
    token: String,
    tab: String,
}

impl Link {
    fn from_env() -> Option<Link> {
        let token = std::env::var(TOKEN_VAR).ok().filter(|token| !token.is_empty())?;
        let tab = crate::ipc::tab_var("WORMHOLE_TAB_ID").filter(|tab| !tab.trim().is_empty())?;
        let socket = crate::ipc::tab_var("WORMHOLE_SOCKET").map(PathBuf::from)?;
        Some(Link { socket, token, tab })
    }

    /// One request and its answer: the reply's result, or what went wrong.
    fn call(&self, op: WelcomeOp) -> Answer {
        let name = op.name();
        let request = Request::new(Action::Welcome { token: self.token.clone(), op }, Some(self.tab.clone()));
        let result = match exchange(&self.socket, &request, REPLY_WAIT) {
            Ok(reply) => match reply.error {
                Some(error) => Err(error.message),
                None => Ok(reply.result),
            },
            Err(why) => Err(why.words()),
        };
        Answer { op: name, result }
    }
}

/// How long the app gets to answer: it answers on its UI thread, in a moment.
const REPLY_WAIT: Duration = Duration::from_secs(2);

/// How long the end waits for the app to take the `Finish` before the shell starts anyway.
const FINISH_WAIT: Duration = Duration::from_millis(1500);

/// The requests go to the app from a thread of their own, so a slow answer never stops the page.
struct Worker {
    ops: mpsc::Sender<WelcomeOp>,
    answers: mpsc::Receiver<Answer>,
}

impl Worker {
    fn start(link: Link) -> Option<Worker> {
        let (ops, op_rx) = mpsc::channel::<WelcomeOp>();
        let (answer_tx, answers) = mpsc::channel::<Answer>();
        std::thread::Builder::new()
            .name("welcome-link".to_string())
            .spawn(move || {
                while let Ok(op) = op_rx.recv() {
                    if answer_tx.send(link.call(op)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Worker { ops, answers })
    }
}

/// How long one turn of the loop waits for a key: about 30 frames a second while the text types.
const TICK_MS: i32 = 33;

/// `Wormhole welcome`. Returns the exit code when it comes back at all; with a program to start, a
/// success never returns (the process becomes it).
pub fn run(args: &[String]) -> i32 {
    let args = match parse_args(args) {
        Ok(args) => args,
        Err(mistake) => {
            let _ = writeln!(io::stderr(), "Wormhole: {mistake}\n\n{HELP}");
            return 2;
        }
    };
    if args.help {
        let _ = io::stdout().write_all(HELP.as_bytes());
        return 0;
    }
    if !tty::is_terminal() {
        // Nothing to show on: never leave a terminal without its shell over it.
        if let Some(shell) = &args.shell {
            return hand_over(shell);
        }
        let _ = writeln!(io::stderr(), "Wormhole: welcome runs in a terminal");
        return 2;
    }
    let Ok(mut raw) = RawMode::enter() else {
        return args.shell.as_ref().map_or(1, |shell| hand_over(shell));
    };
    // A bug in the welcome must never leave a terminal without its shell: a panic ends the welcome as
    // a skip, and the shell starts all the same (the screen guard puts the screen back on the way out).
    {
        let _screen = Screen::take();
        let _ = std::panic::catch_unwind(session);
    }
    let (cols, _) = tty::size().unwrap_or((80, 24));
    let _ = io::stdout().write_all(b"\x1b[2J\x1b[3J\x1b[H");
    raw.restore();
    let mut stdout = io::stdout();
    let _ = stdout.write_all(&banner::bytes(cols));
    let _ = stdout.write_all(b"\x1b[?25h");
    let _ = stdout.flush();
    match &args.shell {
        Some(shell) => hand_over(shell),
        None => 0,
    }
}

/// The screen as the welcome has it: a hidden cursor, no wrapping, clicks reported, titled "Welcome"
/// for the sidebar's row, on the main screen (not the alternate one). Dropping it puts all of that
/// back, on every way out, a panic included.
struct Screen;

impl Screen {
    fn take() -> Screen {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b]2;Welcome\x07\x1b[?25l\x1b[?7l\x1b[?1000h\x1b[?1006h\x1b[H\x1b[J");
        let _ = stdout.flush();
        Screen
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[0m\x1b[?1006l\x1b[?1000l\x1b[?7h\x1b]2;\x07\x1b[?25h");
        let _ = stdout.flush();
    }
}

/// The welcome itself, in raw mode: up to the moment it is over (or the terminal went away, which
/// ends it quietly the same way).
fn session() {
    let mut stdout = io::stdout();

    let worker = Link::from_env().and_then(Worker::start);
    let home = super::facts::home();
    let mut driver = Driver::new(worker.is_some(), home);
    let begin = Instant::now();
    let mut buf = [0u8; 256];
    loop {
        let now = begin.elapsed();
        let answers: Vec<Answer> = worker.iter().flat_map(|worker| worker.answers.try_iter()).collect();
        let (cols, rows) = tty::size().unwrap_or((80, 24));
        let wait = if driver.started(now) { TICK_MS } else { 50 };
        let Some(n) = tty::read_input(&mut buf, wait) else { return };
        let tick = driver.tick(begin.elapsed(), Size { cols, rows }, &buf[..n], answers);
        if !tick.draw.is_empty() {
            let _ = stdout.write_all(&tick.draw);
            let _ = stdout.flush();
        }
        if let Some(worker) = &worker {
            for op in tick.ask {
                let _ = worker.ops.send(op);
            }
        }
        if tick.end.is_some() {
            if let Some(worker) = &worker {
                wait_for_finish(worker);
            }
            return;
        }
    }
}

/// Give the app a moment to take the `Finish` (it remembers the first run there).
fn wait_for_finish(worker: &Worker) {
    let deadline = Instant::now() + FINISH_WAIT;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match worker.answers.recv_timeout(left) {
            Ok(answer) if answer.op == "finish" => return,
            Ok(_) => continue,
            Err(_) => return,
        }
    }
}

/// Become the shell: `shell[0]` with the rest as its arguments, where the terminal started (the
/// welcome never moves it), without the welcome's token. Only comes back when no program could be
/// started; then it says so and falls back to the login shell, so the terminal is never left dead.
fn hand_over(shell: &[String]) -> i32 {
    let start =
        |program: &str, args: &[String]| exec(process::command(program).args(args).env_remove(TOKEN_VAR));
    let Some((program, args)) = shell.split_first() else { return 1 };
    let err = start(program, args);
    let _ = writeln!(io::stderr(), "Wormhole: couldn't start {program}: {err}");
    let fallback = crate::terminal::shells::login_shell();
    let err = start(&fallback, &["-l".to_string()]);
    let _ = writeln!(io::stderr(), "Wormhole: couldn't start {fallback}: {err}");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(words: &[&str]) -> Vec<String> {
        words.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn the_shell_is_what_comes_after_two_dashes() {
        assert_eq!(parse_args(&[]), Ok(Args { shell: None, help: false }));
        let parsed = parse_args(&words(&["--", "/bin/zsh", "-l"])).expect("parses");
        assert_eq!(parsed.shell, Some(words(&["/bin/zsh", "-l"])));
        // What follows `--` is the program's, flags and all.
        let parsed = parse_args(&words(&["--", "fish", "--help"])).expect("parses");
        assert_eq!(parsed.shell, Some(words(&["fish", "--help"])));
        assert!(!parsed.help);
    }

    #[test]
    fn the_terminal_runs_the_welcome_with_the_shell_after_two_dashes() {
        let (program, args) = crate::terminal::welcome_command(
            std::path::Path::new("/Apps/Wormhole"),
            "/bin/zsh".to_string(),
            words(&["-l"]),
        );
        assert_eq!(program, "/Apps/Wormhole");
        assert_eq!(args, words(&["welcome", "--", "/bin/zsh", "-l"]));
        // And the program reads exactly that back.
        let parsed = parse_args(&args[1..]).expect("parses");
        assert_eq!(parsed.shell, Some(words(&["/bin/zsh", "-l"])));
    }

    #[test]
    fn mistakes_are_said() {
        assert!(parse_args(&words(&["--"])).is_err(), "nothing to start");
        assert!(parse_args(&words(&["--", ""])).is_err());
        assert!(parse_args(&words(&["--bogus"])).is_err());
        assert!(parse_args(&words(&["zsh"])).is_err(), "the program comes after `--`");
        assert!(parse_args(&words(&["--help"])).expect("parses").help);
    }
}
