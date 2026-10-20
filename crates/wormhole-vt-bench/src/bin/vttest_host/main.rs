//! vttest-host: run Thomas Dickey's vttest against Wormhole's engine without a window.
//!
//! vttest draws a screen per test and prints beside it what the screen should look like. This host
//! is the terminal at the other end of a pseudo-terminal: it feeds what vttest writes into the
//! engine, writes the engine's replies back, walks the menus (`walk.rs`) and writes the engine's
//! screen at every `Push <RETURN>` as a text file (`capture.rs`). Each top-level menu runs in a
//! vttest of its own, in parallel. `scripts/vttest-gate.sh` compares the files with the committed
//! ones (`tools/vt-compare/suites/vttest/`).
//!
//! ```sh
//! cargo run --release -p wormhole-vt-bench --bin vttest-host -- --out /tmp/vttest
//! cargo run --release -p wormhole-vt-bench --bin vttest-host -- --menu 8 --menu 11 --out /tmp/vttest --trace
//! ```
//!
//! vttest comes from `tools/vt-compare/fetch-suites.sh` (built in the cache, never committed).
// A command line tool: its output is the product, on stdout.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod capture;
mod menu;
mod plan;
mod session;
mod walk;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const USAGE: &str = "\
usage: vttest-host --out <dir> [options]
  --out <dir>        where the screens go: <dir>/<menu>/<nn>.txt, and <dir>/notes.txt (required)
  --vttest <binary>  the built vttest (default: the one fetch-suites.sh built in the cache)
  --menu <n>         only this top-level menu, 1 to 11 (repeatable; default all)
  --only <a.b.c>     walk only the way to this entry, as vttest numbers it (11.3.7.3.10): a test on its own
  --timing           say how long each part of a menu took (to see where to split the work)
  --serial           one menu after the other (default: all at once)
  --trace            print the whole conversation to stderr (use with one --menu)
";

struct Options {
    out: PathBuf,
    vttest: PathBuf,
    menus: Vec<u32>,
    only: Option<Vec<u32>>,
    timing: bool,
    serial: bool,
    trace: bool,
}

fn parse_args() -> Result<Options, String> {
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
        vttest: find_vttest(&cache.join("suites")).unwrap_or_default(),
        menus: Vec::new(),
        only: None,
        timing: false,
        serial: false,
        trace: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--out" => options.out = value("--out")?.into(),
            "--vttest" => options.vttest = value("--vttest")?.into(),
            "--menu" => options.menus.push(value("--menu")?.parse().map_err(|_| "--menu is a number")?),
            "--only" => {
                let path = value("--only")?;
                let numbers: Result<Vec<u32>, _> = path.split('.').map(str::parse).collect();
                let numbers = numbers.map_err(|_| "--only is menu numbers joined by dots, 11.3.7")?;
                options.only = Some(numbers);
            }
            "--timing" => options.timing = true,
            "--serial" => options.serial = true,
            "--trace" => options.trace = true,
            "-h" | "--help" => return Err(String::new()),
            other => return Err(format!("unknown option {other}")),
        }
    }
    if options.out.as_os_str().is_empty() {
        return Err("--out is required".into());
    }
    if !options.vttest.is_file() {
        return Err("no vttest binary (run tools/vt-compare/fetch-suites.sh, or pass --vttest)".into());
    }
    if options.menus.is_empty() {
        options.menus = (1..=11).collect();
    }
    Ok(options)
}

/// The newest `vttest-<version>/vttest` in the suites cache.
fn find_vttest(suites: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(suites)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("vttest-")))
        .map(|p| p.join("vttest"))
        .filter(|p| p.is_file())
        .collect();
    found.sort();
    found.pop()
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
            eprintln!("vttest-host: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(options: &Options) -> Result<(), String> {
    let relay = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vt-compare/suites/pty_relay.py");
    let version = vttest_version(&options.vttest);
    // The parts to walk: a top-level menu is one part or several (plan::units), all at once.
    let units: Vec<Vec<u32>> = match &options.only {
        Some(only) => vec![only.clone()],
        None => options.menus.iter().flat_map(|&menu| plan::units(menu)).collect(),
    };
    let walked = walk_all(&units, options, &relay, &version);
    if walked.len() != units.len() {
        return Err("a walk panicked".into());
    }
    if options.timing {
        for (unit, walked) in units.iter().zip(&walked) {
            let at = unit.iter().map(u32::to_string).collect::<Vec<_>>().join(".");
            println!("{at:>10}: {:>5.1} s, {} screens", walked.took.as_secs_f32(), walked.screens.len());
        }
    }
    // The screens of a menu's parts, in the order of the parts, numbered together.
    let mut notes = String::new();
    let mut menus: Vec<u32> = units.iter().filter_map(|u| u.first().copied()).collect();
    menus.dedup();
    for menu in menus {
        let dir = plan::menu_dir(menu);
        let folder = options.out.join(&dir);
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let mine = units.iter().zip(&walked).filter(|(unit, _)| unit.first() == Some(&menu)).map(|(_, w)| w);
        let mut count = 0;
        for walk in mine {
            for text in &walk.screens {
                count += 1;
                let path = folder.join(format!("{count:03}.txt"));
                let text = text.replacen("screen @@", &format!("screen {count:03}"), 1);
                std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            for note in &walk.notes {
                notes.push_str(&format!("{dir}: {note}\n"));
            }
        }
        println!("menu {menu:>2}: {count} screens");
    }
    std::fs::write(options.out.join("notes.txt"), notes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Walk every part, a few at a time (each is a vttest on a pseudo-terminal of its own, and the
/// machine's pseudo-terminals are shared with every terminal running on it), the slowest first so
/// they do not end up last. The results come back in the order of `units`.
fn walk_all(units: &[Vec<u32>], options: &Options, relay: &Path, version: &str) -> Vec<walk::Walked> {
    const WORKERS: usize = 16;
    let workers = if options.serial || options.trace { 1 } else { WORKERS.min(units.len()) };
    let mut schedule: Vec<usize> = (0..units.len()).collect();
    schedule.sort_by_key(|&i| std::cmp::Reverse(plan::slow(&units[i])));
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<walk::Walked>>> = Mutex::new((0..units.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(&i) = schedule.get(next.fetch_add(1, Ordering::SeqCst)) {
                    let walked = walk::walk(&units[i], &options.vttest, relay, version, options.trace);
                    if let Ok(mut results) = results.lock() {
                        results[i] = Some(walked);
                    }
                }
            });
        }
    });
    results.into_inner().unwrap_or_default().into_iter().flatten().collect()
}

/// The version in the folder name the cache keeps it under (`vttest-20251205/vttest`).
fn vttest_version(binary: &Path) -> String {
    binary
        .parent()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().trim_start_matches("vttest-").to_string())
        .unwrap_or_default()
}
