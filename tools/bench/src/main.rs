//! ttystorm: stress tests, benchmarks and animations for any terminal. See README.md.

mod bench;
mod canvas;
mod demo;
mod demos;
mod probe;
mod term;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use demo::Options;

const USAGE: &str = "\
ttystorm – stress tests, benchmarks and animations for terminals

Every animation and benchmark is also a command of its own: `matrix` is `ttystorm matrix`,
`logflood` is `ttystorm logflood` (see `ttystorm list`).

USAGE
  ttystorm <animation> [options]     play one animation
  ttystorm <benchmark> [options]     run one benchmark
  ttystorm showcase [options]        play every animation in a row (← → to skip)
  ttystorm bench [options]           every benchmark, one result screen
  ttystorm flood [options]           one benchmark workload, endlessly, for profiling a terminal
  ttystorm ttyprobe [options]        what the terminal supports and how wide it draws text
  ttystorm compare [--markdown] <a.json> <b.json> …  results of several terminals side by side
                                     (benchmark files or ttyprobe files, not mixed)
  ttystorm list                      the animations and benchmarks
  ttystorm names                     every command name, one per line (to install links)

";

const ANIMATION_OPTIONS: &str = "\
ANIMATION OPTIONS
  --fps <n>       frames per second to aim for (default 60; 0 = as fast as the terminal reads)
  --seconds <n>   stop after n seconds (per animation in a showcase; default 10 there)
  --hud           start with the fps / MB/s overlay on (h shows and hides it)
  --no-sync       don't use synchronized output (mode 2026): shows tearing
  --titles        show each animation's name as it starts (on in a showcase)

KEYS
  q / Esc         quit           space   pause        h   fps / MB/s overlay
  ← →             previous/next  + - 0   speed
";

const BENCH_OPTIONS: &str = "\
BENCHMARK OPTIONS
  --plain         no status line and no result screen: the results as plain text
  --size <MB>     data per test (default 16)
  --runs <n>      runs per test; the median is reported (default 3)
  --save <file>   write the results as JSON, for `compare`
  --label <name>  the terminal's name in the results (default: $TERM_PROGRAM)
  --details       after the results, why: stalls, read vs parse time, the terminal's CPU and memory,
                  what the workload contains, and findings in plain words
  --trace <file>  every write as a JSON line (test, run, time, bytes, how long it blocked)
  --no-verify     don't check that the terminal parsed everything: no checkpoints in the payload
                  (the numbers are then marked unverified)
  --verify-every <KiB>  about how far apart the input check's checkpoints are (default 256)
  --only <a,b>    (bench) run only these benchmarks, or a group: realistic, stress, all
  --suite <name>  (bench) the same as --only <name>: realistic, stress or all
";

const FLOOD_OPTIONS: &str = "\
FLOOD OPTIONS
  --kind <name>   the workload: logflood (default), colorflood, glyphstorm, cursorstorm, ...
  --seconds <n>   stop after n seconds (default: until q / Ctrl-C)
";

const PROBE_OPTIONS: &str = "\
FEATURE CHECK OPTIONS
  --plain         the results as plain text (the queries are still sent); no sample page
  --save <file>   write the results, with every query's transcript, as JSON, for `compare`
  --label <name>  the terminal's name in the results (default: $TERM_PROGRAM)
  --details       after the results: every query sent and received, how it was decided, findings
  --raw [file]    also dump every byte received, escaped, to stderr or to a file (to debug the probe)
  --no-sample     leave out the sample page (underlines, links, glyphs on a ruler)
";

fn main() -> ExitCode {
    let mut argv = std::env::args();
    let program = argv.next().unwrap_or_default();
    let mut args: Vec<String> = argv.collect();
    // Started through a link named after an animation or benchmark: that is the command.
    let invoked = command_name(&program);
    if let Some(name) = invoked {
        args.insert(0, name.to_string());
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = writeln!(io::stderr(), "{}: {e}", invoked.unwrap_or("ttystorm"));
            ExitCode::FAILURE
        }
    }
}

/// The animation or benchmark a program path names, when it isn't ttystorm itself.
fn command_name(program: &str) -> Option<&'static str> {
    let file = Path::new(program).file_name()?.to_str()?;
    names().into_iter().find(|n| *n == file)
}

/// Every animation and benchmark, each a command of its own.
fn names() -> Vec<&'static str> {
    demos::ALL.iter().map(|e| e.name).chain(bench::NAMES).chain(["ttyprobe"]).collect()
}

fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| matches!(a.as_str(), "-h" | "--help"))
}

fn run(args: &[String]) -> io::Result<()> {
    let Some(cmd) = args.first() else {
        return say(&format!(
            "{USAGE}{ANIMATION_OPTIONS}\n{BENCH_OPTIONS}\n{FLOOD_OPTIONS}\n{PROBE_OPTIONS}"
        ));
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "-h" | "--help" | "help" => {
            say(&format!("{USAGE}{ANIMATION_OPTIONS}\n{BENCH_OPTIONS}\n{FLOOD_OPTIONS}\n{PROBE_OPTIONS}"))
        }
        "list" => say(&list()),
        "names" => say(&names().iter().map(|n| format!("{n}\n")).collect::<String>()),
        "showcase" => {
            let mut opts = demo_options(rest)?;
            opts.titles = true;
            opts.seconds.get_or_insert(10.0);
            demo::run_list(demos::ALL, &opts)
        }
        "flood" => bench::flood(&flood_options(rest)?),
        "bench" => bench::run(&bench_options(rest)?),
        "compare" => compare(rest),
        "ttyprobe" => {
            if wants_help(rest) {
                return say(&command_help("ttyprobe", probe::ABOUT, PROBE_OPTIONS));
            }
            probe::run(&probe_options(rest)?)
        }
        name if bench::NAMES.contains(&name) => {
            if wants_help(rest) {
                return say(&command_help(name, bench::about(name), BENCH_OPTIONS));
            }
            let mut opts = bench_options(rest)?;
            opts.only = vec![name.to_string()];
            bench::run(&opts)
        }
        name => match demos::find(name) {
            Some(entry) if wants_help(rest) => say(&command_help(name, entry.about, ANIMATION_OPTIONS)),
            Some(entry) => demo::run(entry, &demo_options(rest)?),
            None => Err(io::Error::other(format!(
                "no command, animation or benchmark called `{name}` (try `ttystorm list`)"
            ))),
        },
    }
}

/// `compare`: probe files get the feature matrix, benchmark files the bars; a mix is an error.
fn compare(args: &[String]) -> io::Result<()> {
    let mut markdown = false;
    let mut files = Vec::new();
    for a in args {
        match a.as_str() {
            "--markdown" => markdown = true,
            other if other.starts_with("--") => {
                return Err(io::Error::other(format!("unknown option {other}")));
            }
            f => files.push(PathBuf::from(f)),
        }
    }
    let probe_file: Vec<bool> = files
        .iter()
        .map(|f| std::fs::read_to_string(f).map(|t| probe::is_probe_file(&t)).unwrap_or(false))
        .collect();
    if !files.is_empty() && probe_file.iter().all(|p| *p) {
        return probe::compare(&files, markdown);
    }
    if probe_file.iter().any(|p| *p) {
        let named = |want: bool| {
            files
                .iter()
                .zip(&probe_file)
                .filter(|(_, p)| **p == want)
                .map(|(f, _)| f.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(io::Error::other(format!(
            "can't compare ttyprobe files ({}) with benchmark files ({}): compare one kind at a time",
            named(true),
            named(false)
        )));
    }
    bench::compare(&files, markdown)
}

fn command_help(name: &str, about: &str, options: &str) -> String {
    format!("{name} – {about}\n\nUSAGE\n  {name} [options]          (or ttystorm {name})\n\n{options}")
}

fn list() -> String {
    let mut s = String::from("ANIMATIONS\n");
    for e in demos::ALL {
        s.push_str(&format!("  {:<12} {}\n", e.name, e.about));
    }
    s.push_str("\nBENCHMARKS\n");
    for n in bench::NAMES {
        s.push_str(&format!("  {:<12} {}\n", n, bench::about(n)));
    }
    s.push_str(&format!("\nFEATURE CHECK\n  {:<12} {}\n", "ttyprobe", probe::ABOUT));
    s
}

fn say(s: &str) -> io::Result<()> {
    io::stdout().write_all(s.as_bytes())
}

fn value<'a>(args: &'a [String], i: &mut usize) -> io::Result<&'a str> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| io::Error::other(format!("{} needs a value", args[*i - 1])))
}

fn number(s: &str) -> io::Result<f64> {
    s.parse().map_err(|_| io::Error::other(format!("not a number: {s}")))
}

fn demo_options(args: &[String]) -> io::Result<Options> {
    let mut o = Options::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--fps" => o.fps = number(value(args, &mut i)?)?,
            "--seconds" => o.seconds = Some(number(value(args, &mut i)?)?),
            "--hud" => o.hud = true,
            // The default since 1.1; still taken so older scripts keep working.
            "--no-hud" => o.hud = false,
            "--no-sync" => o.sync = false,
            "--titles" => o.titles = true,
            other => return Err(io::Error::other(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

fn bench_options(args: &[String]) -> io::Result<bench::BenchOptions> {
    let mut o = bench::BenchOptions {
        size_mb: 16.0,
        only: Vec::new(),
        save: None,
        label: None,
        runs: 3,
        plain: false,
        details: false,
        trace: None,
        verify_every: Some(bench::DEFAULT_VERIFY_EVERY),
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--size" => o.size_mb = number(value(args, &mut i)?)?,
            "--only" => {
                o.only = value(args, &mut i)?.split(',').map(|n| bench::current_name(n).to_string()).collect()
            }
            "--plain" => o.plain = true,
            "--details" => o.details = true,
            "--no-verify" => o.verify_every = None,
            "--verify-every" => {
                let kib = number(value(args, &mut i)?)?;
                if !(4.0..=65536.0).contains(&kib) {
                    return Err(io::Error::other("--verify-every must be between 4 and 65536 KiB"));
                }
                o.verify_every = Some((kib * 1024.0) as usize);
            }
            "--suite" => o.only = vec![value(args, &mut i)?.to_string()],
            "--trace" => o.trace = Some(PathBuf::from(value(args, &mut i)?)),
            "--save" => o.save = Some(PathBuf::from(value(args, &mut i)?)),
            "--runs" => o.runs = number(value(args, &mut i)?)? as usize,
            "--label" => o.label = Some(value(args, &mut i)?.to_string()),
            other => return Err(io::Error::other(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

fn probe_options(args: &[String]) -> io::Result<probe::ProbeOptions> {
    let mut o = probe::ProbeOptions {
        plain: false,
        save: None,
        label: None,
        sample: true,
        details: false,
        raw: None,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--plain" => o.plain = true,
            "--save" => o.save = Some(PathBuf::from(value(args, &mut i)?)),
            "--label" => o.label = Some(value(args, &mut i)?.to_string()),
            "--no-sample" => o.sample = false,
            "--details" => o.details = true,
            // A file name follows only when the next argument isn't another option.
            "--raw" => match args.get(i + 1).filter(|a| !a.starts_with('-')) {
                Some(file) => {
                    o.raw = Some(probe::RawTarget::File(PathBuf::from(file)));
                    i += 1;
                }
                None => o.raw = Some(probe::RawTarget::Stderr),
            },
            other => return Err(io::Error::other(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

fn flood_options(args: &[String]) -> io::Result<bench::FloodOptions> {
    let mut o = bench::FloodOptions { kind: "logflood".to_string(), seconds: None };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kind" => o.kind = bench::current_name(value(args, &mut i)?).to_string(),
            "--seconds" => o.seconds = Some(number(value(args, &mut i)?)?),
            other => return Err(io::Error::other(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::{command_name, names};

    /// The commands `run` matches before any animation or benchmark.
    const BUILT_IN: [&str; 7] = ["showcase", "bench", "flood", "compare", "list", "names", "help"];

    #[test]
    fn every_command_name_is_its_own() {
        let all = names();
        for (i, n) in all.iter().enumerate() {
            assert!(!all[i + 1..].contains(n), "{n} is both an animation and a benchmark");
            assert!(!BUILT_IN.contains(n), "{n} is a built-in command");
        }
    }

    #[test]
    fn a_link_named_after_a_command_runs_it() {
        assert_eq!(command_name("/opt/homebrew/bin/matrix"), Some("matrix"));
        assert_eq!(command_name("roundtrip"), Some("roundtrip"));
        assert_eq!(command_name("/usr/local/bin/ttystorm"), None);
        assert_eq!(command_name("target/release/bench"), None);
    }
}
