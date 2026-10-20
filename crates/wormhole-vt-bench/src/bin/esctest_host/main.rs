//! esctest-host: run esctest2 against Wormhole's engine without a window.
//!
//! The suite (`tools/vt-compare/suites/esctest_run.py`) runs on a pseudo-terminal that
//! `pty_relay.py` keeps; this program is the terminal at the other end. What the suite writes goes
//! through `wormhole_vt::Terminal` as the app feeds it, and the engine's replies go back, so
//! the results are the in-app run's without opening an app: same engine, same options (the screen
//! checksum on), the host side of the events (title, clipboard, window operations, colors) played
//! by `host.rs` on a fake window.
//!
//! ```sh
//! cargo run --release -p wormhole-vt-bench --bin esctest-host -- --out /tmp/wormhole.jsonl
//! cargo run --release -p wormhole-vt-bench --bin esctest-host -- --include 'BSTests' --out /tmp/bs.jsonl
//! python3 tools/vt-compare/suites/esctest_compare.py /tmp/wormhole.jsonl [other.jsonl]
//! ```
//!
//! The suite and its Python come from `tools/vt-compare/fetch-suites.sh` (cached under
//! `~/Library/Caches/wormhole/vt-compare/suites`).
// A command line tool: its output is the product, on stdout.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod host;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use host::Host;

const USAGE: &str = "\
usage: esctest-host --out <file.jsonl> [options]
  --out <file>        the runner's JSON lines (required)
  --include <regex>   only the tests whose Class.test_name matches (default all)
  --suites <dir>      where fetch-suites.sh put the suites (default: the cache fetch-suites.sh fills)
  --size <cols>x<rows>  the terminal's size (default 80x25, what esctest2 resets to)
  --log <file>        esctest2's own log (default <out>.log; failures' tracebacks go to <log>.trace)
  --no-window-ops     refuse the window operations and the title reports (allow_window_ops and allow_title_report off)
  --no-checksum       do not answer DECRQCRA (allow_screen_checksum off)
  --runner-arg <arg>  pass an argument on to esctest_run.py (repeatable)
  --verbose           print every chunk of the conversation to stderr
";

struct Options {
    out: PathBuf,
    include: String,
    suites: PathBuf,
    cols: usize,
    rows: usize,
    log: Option<PathBuf>,
    window_ops: bool,
    checksum: bool,
    verbose: bool,
    runner_args: Vec<String>,
}

fn parse_args() -> Result<Options, String> {
    // Where fetch-suites.sh puts them: its cache override, else the platform's cache folder.
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let cache = match std::env::var_os("WORMHOLE_VT_COMPARE_CACHE") {
        Some(cache) => PathBuf::from(cache),
        None if cfg!(target_os = "macos") => home.join("Library/Caches/wormhole/vt-compare"),
        None => std::env::var_os("XDG_CACHE_HOME")
            .map_or_else(|| home.join(".cache"), PathBuf::from)
            .join("wormhole/vt-compare"),
    };
    let mut options = Options {
        out: PathBuf::new(),
        include: ".*".into(),
        suites: cache.join("suites"),
        cols: 80,
        rows: 25,
        log: None,
        window_ops: true,
        checksum: true,
        verbose: false,
        runner_args: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--out" => options.out = value("--out")?.into(),
            "--include" => options.include = value("--include")?,
            "--suites" => options.suites = value("--suites")?.into(),
            "--log" => options.log = Some(value("--log")?.into()),
            "--size" => {
                let size = value("--size")?;
                let (cols, rows) = size.split_once('x').ok_or("--size is <cols>x<rows>")?;
                options.cols = cols.parse().map_err(|_| "--size is <cols>x<rows>")?;
                options.rows = rows.parse().map_err(|_| "--size is <cols>x<rows>")?;
            }
            "--no-window-ops" => options.window_ops = false,
            "--no-checksum" => options.checksum = false,
            "--verbose" => options.verbose = true,
            "--runner-arg" => options.runner_args.push(value("--runner-arg")?),
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    if options.out.as_os_str().is_empty() {
        return Err("--out is required".into());
    }
    Ok(options)
}

/// The first folder in `dir` that starts with `prefix` (the suites are cached under a name with
/// the pinned revision in it).
fn find(dir: &Path, prefix: &str) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e} (run tools/vt-compare/fetch-suites.sh)", dir.display()))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
        .collect();
    found.sort();
    found.pop().ok_or(format!("no {prefix}* in {}", dir.display()))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn main() -> ExitCode {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("{message}");
            }
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("esctest-host: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: &Options) -> Result<(), String> {
    let esctest = find(&options.suites, "esctest2-")?.join("esctest");
    let python = find(&options.suites, "venv-")?.join("bin/python");
    let runner = repo_root().join("tools/vt-compare/suites/esctest_run.py");
    let relay = repo_root().join("tools/vt-compare/suites/pty_relay.py");
    let log = options.log.clone().unwrap_or_else(|| options.out.with_extension("log"));
    let scratch = std::env::temp_dir().join(format!("esctest-host-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_file(log.with_extension("log.trace"));
    let mut trace = log.clone().into_os_string();
    trace.push(".trace");
    let _ = std::fs::remove_file(trace);

    let mut child = Command::new("python3")
        .current_dir(&scratch)
        .arg("-I")
        .arg(&relay)
        .arg(options.rows.to_string())
        .arg(options.cols.to_string())
        .arg("--")
        .arg(&python)
        .arg(&runner)
        .arg("--esctest")
        .arg(&esctest)
        .arg("--out")
        .arg(&options.out)
        .arg("--log")
        .arg(&log)
        .args(["--label", "wormhole-headless", "--clipboard", "--include"])
        .arg(&options.include)
        .args(&options.runner_args)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("could not start python3: {e}"))?;
    let mut to_child = child.stdin.take().ok_or("no stdin")?;
    let mut from_child = child.stdout.take().ok_or("no stdout")?;

    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 64 * 1024];
        while let Ok(n) = from_child.read(&mut buf) {
            if n == 0 || sender.send(buf[..n].to_vec()).is_err() {
                break;
            }
        }
    });

    let mut host = Host::new(options.cols, options.rows, options.window_ops, options.checksum);
    loop {
        match receiver.recv_timeout(Duration::from_secs(120)) {
            Ok(chunk) => {
                if options.verbose {
                    eprintln!("<< {:?}", String::from_utf8_lossy(&chunk));
                }
                let reply = host.feed(&chunk);
                if !reply.is_empty() {
                    if options.verbose {
                        eprintln!(">> {:?}", String::from_utf8_lossy(&reply));
                    }
                    if to_child.write_all(&reply).and_then(|()| to_child.flush()).is_err() {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let _ = child.kill();
                return Err("the suite went quiet for 120 s".into());
            }
        }
    }
    drop(to_child);
    let status = child.wait().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&scratch);
    if status.success() { Ok(()) } else { Err(format!("the runner ended with {status}")) }
}
