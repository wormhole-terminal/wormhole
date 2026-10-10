//! termbench: stress tests, benchmarks and animations for any terminal. See README.md.

mod bench;
mod canvas;
mod demo;
mod demos;
mod term;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use demo::Options;

const USAGE: &str = "\
termbench – stress tests, benchmarks and animations for terminals

USAGE
  termbench <demo> [options]          play one animation (see `termbench list`)
  termbench showcase [options]        play every animation in a row (← → to skip)
  termbench bench [options]           throughput and latency benchmarks
  termbench flood [options]           one benchmark workload, endlessly, for profiling a terminal
  termbench compare <a.json> <b.json> …  results of several terminals side by side
  termbench list                      the animations

ANIMATION OPTIONS
  --fps <n>       frames per second to aim for (default 60; 0 = as fast as the terminal reads)
  --seconds <n>   stop after n seconds (per animation in a showcase; default 10 there)
  --no-hud        hide the fps / MB/s line (toggle with h)
  --no-sync       don't use synchronized output (mode 2026): shows tearing
  --titles        show each animation's name as it starts (on in a showcase)

BENCH OPTIONS
  --size <MB>     data per test (default 16)
  --only <a,b>    run only these tests
  --runs <n>      runs per test; the median is reported (default 3)
  --save <file>   write the results as JSON, for `compare`
  --label <name>  the terminal's name in the results (default: $TERM_PROGRAM)

FLOOD OPTIONS
  --kind <name>   the workload: ascii (default), truecolor, unicode, cursor, scroll-region, ...
  --seconds <n>   stop after n seconds (default: until q / Ctrl-C)

KEYS
  q / Esc         quit           space   pause        h   overlay
  ← →             previous/next  + - 0   speed
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            let _ = writeln!(io::stderr(), "termbench: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> io::Result<()> {
    let Some(cmd) = args.first() else {
        return say(USAGE);
    };
    let rest = &args[1..];
    match cmd.as_str() {
        "-h" | "--help" | "help" => say(USAGE),
        "list" => {
            let mut s = String::new();
            for e in demos::ALL {
                s.push_str(&format!("  {:<12} {}\n", e.name, e.about));
            }
            say(&s)
        }
        "showcase" => {
            let mut opts = demo_options(rest)?;
            opts.titles = true;
            opts.seconds.get_or_insert(10.0);
            demo::run_list(demos::ALL, &opts)
        }
        "flood" => bench::flood(&flood_options(rest)?),
        "bench" => bench::run(&bench_options(rest)?),
        "compare" => bench::compare(&rest.iter().map(PathBuf::from).collect::<Vec<_>>()),
        name => match demos::find(name) {
            Some(entry) => demo::run(entry, &demo_options(rest)?),
            None => Err(io::Error::other(format!(
                "no command or animation called `{name}` (try `termbench list`)"
            ))),
        },
    }
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
    let mut o = bench::BenchOptions { size_mb: 16.0, only: Vec::new(), save: None, label: None, runs: 3 };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--size" => o.size_mb = number(value(args, &mut i)?)?,
            "--only" => o.only = value(args, &mut i)?.split(',').map(str::to_string).collect(),
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
    let mut o = bench::FloodOptions { kind: "ascii".to_string(), seconds: None };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kind" => o.kind = value(args, &mut i)?.to_string(),
            "--seconds" => o.seconds = Some(number(value(args, &mut i)?)?),
            other => return Err(io::Error::other(format!("unknown option {other}"))),
        }
        i += 1;
    }
    Ok(o)
}
