//! ttystorm: stress tests, benchmarks and animations for any terminal. See README.md.

mod bench;
mod canvas;
mod demo;
mod demos;
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
  ttystorm compare <a.json> <b.json> …  results of several terminals side by side
  ttystorm list                      the animations and benchmarks
  ttystorm names                     every command name, one per line (to install links)

";

const ANIMATION_OPTIONS: &str = "\
ANIMATION OPTIONS
  --fps <n>       frames per second to aim for (default 60; 0 = as fast as the terminal reads)
  --seconds <n>   stop after n seconds (per animation in a showcase; default 10 there)
  --no-hud        hide the fps / MB/s line (toggle with h)
  --no-sync       don't use synchronized output (mode 2026): shows tearing
  --titles        show each animation's name as it starts (on in a showcase)

KEYS
  q / Esc         quit           space   pause        h   overlay
  ← →             previous/next  + - 0   speed
";

const BENCH_OPTIONS: &str = "\
BENCHMARK OPTIONS
  --plain         no status line and no result screen: the results as plain text
  --size <MB>     data per test (default 16)
  --runs <n>      runs per test; the median is reported (default 3)
  --save <file>   write the results as JSON, for `compare`
  --label <name>  the terminal's name in the results (default: $TERM_PROGRAM)
  --only <a,b>    (bench) run only these benchmarks
";

const FLOOD_OPTIONS: &str = "\
FLOOD OPTIONS
  --kind <name>   the workload: logflood (default), colorflood, glyphstorm, cursorstorm, ...
  --seconds <n>   stop after n seconds (default: until q / Ctrl-C)
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
    demos::ALL.iter().map(|e| e.name).chain(bench::NAMES).collect()
}

fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| matches!(a.as_str(), "-h" | "--help"))
}

fn run(args: &[String]) -> io::Result<()> {
    let Some(cmd) = args.first() else {
        return say(&format!("{USAGE}{ANIMATION_OPTIONS}\n{BENCH_OPTIONS}\n{FLOOD_OPTIONS}"));
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "-h" | "--help" | "help" => {
            say(&format!("{USAGE}{ANIMATION_OPTIONS}\n{BENCH_OPTIONS}\n{FLOOD_OPTIONS}"))
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
        "compare" => bench::compare(&rest.iter().map(PathBuf::from).collect::<Vec<_>>()),
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
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--size" => o.size_mb = number(value(args, &mut i)?)?,
            "--only" => {
                o.only = value(args, &mut i)?.split(',').map(|n| bench::current_name(n).to_string()).collect()
            }
            "--plain" => o.plain = true,
            "--save" => o.save = Some(PathBuf::from(value(args, &mut i)?)),
            "--runs" => o.runs = number(value(args, &mut i)?)? as usize,
            "--label" => o.label = Some(value(args, &mut i)?.to_string()),
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
