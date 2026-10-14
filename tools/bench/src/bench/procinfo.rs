//! The terminal's own cost: which process it is, and how much CPU time and memory it used during a
//! test. Best effort: any step that fails (no `ps`, no such process, a sandbox) just leaves the
//! numbers out; nothing here is ever an error for the benchmark.

use std::process::Command;

use super::json::{self, Value};

/// The process the benchmark's terminal runs as.
#[derive(Clone, Debug, PartialEq)]
pub struct Terminal {
    pub pid: u32,
    /// The program's file name (`ghostty`, `kitty`, `Terminal`).
    pub command: String,
    /// When the first process without a terminal of its own is not the terminal but something in
    /// front of it (`tmux`, `screen`, `ssh`): then the numbers describe that, not the terminal.
    pub note: Option<String>,
}

impl Terminal {
    pub fn to_json(&self) -> String {
        let note = self.note.as_deref().map_or("null".to_string(), json::quote);
        format!("{{\"pid\": {}, \"command\": {}, \"note\": {note}}}", self.pid, json::quote(&self.command))
    }

    pub fn from_value(v: &Value) -> Option<Terminal> {
        Some(Terminal {
            pid: v.get("pid")?.as_f64()? as u32,
            command: v.get("command")?.as_str()?.to_string(),
            note: v.get("note").and_then(Value::as_str).map(str::to_string),
        })
    }
}

/// A process's CPU time (all its threads) and resident memory at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub cpu_s: f64,
    pub rss_kb: u64,
}

/// One line of `ps -o ppid=,tty=,comm=`: the parent, the terminal device and the command.
pub fn parse_ps_line(line: &str) -> Option<(u32, String, String)> {
    let line = line.trim();
    let (ppid, rest) = line.split_once(char::is_whitespace)?;
    let (tty, comm) = rest.trim_start().split_once(char::is_whitespace)?;
    let comm = comm.trim();
    if comm.is_empty() {
        return None;
    }
    Some((ppid.parse().ok()?, tty.to_string(), comm.to_string()))
}

/// `ps` shows no controlling terminal as `??` (macOS), `?` (Linux) or `-`.
fn has_no_tty(tty: &str) -> bool {
    matches!(tty, "??" | "?" | "-")
}

fn base_name(comm: &str) -> &str {
    comm.rsplit('/').next().unwrap_or(comm)
}

/// What stands between the benchmark and the terminal, from the process found.
fn in_front(command: &str) -> Option<String> {
    let c = command.to_ascii_lowercase();
    ["tmux", "screen", "ssh", "mosh"]
        .into_iter()
        .find(|name| c.starts_with(name))
        .map(|name| if name == "mosh" { "ssh" } else { name }.to_string())
}

/// Walks up from `start` to the first ancestor that has no controlling terminal, which is the
/// terminal emulator (or whatever launched the shell). `query` answers `(parent, tty, command)` for a
/// pid; it is a parameter so the walk can be tested.
pub fn walk(start: u32, mut query: impl FnMut(u32) -> Option<(u32, String, String)>) -> Option<Terminal> {
    let (mut pid, mut hops) = (start, 0);
    loop {
        let (ppid, tty, comm) = query(pid)?;
        if pid != start && has_no_tty(&tty) {
            let command = base_name(&comm).to_string();
            let note = in_front(&command);
            return Some(Terminal { pid, command, note });
        }
        hops += 1;
        if ppid <= 1 || hops > 32 {
            return None;
        }
        pid = ppid;
    }
}

fn ps_query(pid: u32) -> Option<(u32, String, String)> {
    let out = Command::new("ps").args(["-o", "ppid=,tty=,comm=", "-p", &pid.to_string()]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_ps_line(String::from_utf8_lossy(&out.stdout).lines().next()?)
}

/// The terminal this process runs in, or `None` when the tree can't be read.
pub fn find_terminal() -> Option<Terminal> {
    walk(std::process::id(), ps_query)
}

/// `ps` CPU time: `M:SS.hh`, `H:MM:SS` or `D-HH:MM:SS`, in seconds.
pub fn parse_cpu_time(s: &str) -> Option<f64> {
    let s = s.trim();
    let (days, rest) = match s.split_once('-') {
        Some((d, r)) => (d.parse::<f64>().ok()?, r),
        None => (0.0, s),
    };
    let mut total = 0.0;
    for part in rest.split(':') {
        total = total * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(days * 86_400.0 + total)
}

/// `ps -o time=,rss=`: `  0:12.34  123456`.
pub fn parse_time_rss(line: &str) -> Option<Sample> {
    let mut it = line.split_whitespace();
    let cpu_s = parse_cpu_time(it.next()?)?;
    let rss_kb = it.next()?.parse().ok()?;
    Some(Sample { cpu_s, rss_kb })
}

/// `/proc/<pid>/stat`: user and system time, in clock ticks (100 a second on every Linux in use).
pub fn parse_proc_stat(stat: &str, statm: &str) -> Option<Sample> {
    // The command is in parentheses and may hold spaces: count the fields after the last `)`.
    let after = &stat[stat.rfind(')')? + 1..];
    let f: Vec<&str> = after.split_whitespace().collect();
    let ticks: f64 = f.get(11)?.parse::<f64>().ok()? + f.get(12)?.parse::<f64>().ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    Some(Sample { cpu_s: ticks / 100.0, rss_kb: pages * 4 })
}

pub fn sample(pid: u32) -> Option<Sample> {
    if cfg!(target_os = "linux")
        && let (Ok(stat), Ok(statm)) = (
            std::fs::read_to_string(format!("/proc/{pid}/stat")),
            std::fs::read_to_string(format!("/proc/{pid}/statm")),
        )
    {
        return parse_proc_stat(&stat, &statm);
    }
    let out = Command::new("ps").args(["-o", "time=,rss=", "-p", &pid.to_string()]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    parse_time_rss(String::from_utf8_lossy(&out.stdout).lines().next()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_lines_parse_with_spaces_in_the_command() {
        assert_eq!(
            parse_ps_line("  4242 ttys003  /bin/zsh"),
            Some((4242, "ttys003".to_string(), "/bin/zsh".to_string()))
        );
        assert_eq!(
            parse_ps_line("   1 ??  /Applications/Visual Studio Code.app/Contents/MacOS/Electron\n"),
            Some((
                1,
                "??".to_string(),
                "/Applications/Visual Studio Code.app/Contents/MacOS/Electron".to_string()
            ))
        );
        assert_eq!(parse_ps_line(""), None);
        assert_eq!(parse_ps_line("12 ttys001"), None);
        assert_eq!(parse_ps_line("x ?? zsh"), None);
    }

    fn tree(pid: u32) -> Option<(u32, String, String)> {
        let row = |p: u32, t: &str, c: &str| Some((p, t.to_string(), c.to_string()));
        match pid {
            100 => row(90, "ttys003", "ttystorm"),
            90 => row(80, "ttys003", "/bin/zsh"),
            80 => row(70, "ttys003", "/usr/bin/login"),
            70 => row(1, "??", "/Applications/Ghostty.app/Contents/MacOS/ghostty"),
            // Under tmux.
            200 => row(190, "pts/1", "ttystorm"),
            190 => row(180, "pts/1", "bash"),
            180 => row(1, "?", "tmux: server"),
            _ => None,
        }
    }

    #[test]
    fn the_walk_stops_at_the_first_process_without_a_terminal() {
        let t = walk(100, tree);
        assert_eq!(t, Some(Terminal { pid: 70, command: "ghostty".to_string(), note: None }));
    }

    #[test]
    fn tmux_is_named_instead_of_passed_off_as_the_terminal() {
        let t = walk(200, tree);
        assert_eq!(t.as_ref().map(|t| (t.pid, t.note.as_deref())), Some((180, Some("tmux"))));
    }

    #[test]
    fn a_broken_tree_gives_nothing() {
        assert_eq!(walk(999, tree), None);
        // A shell whose parent is init has no terminal to find.
        assert_eq!(walk(100, |p| Some((1, "ttys001".to_string(), format!("p{p}")))), None);
    }

    #[test]
    fn cpu_times_in_every_ps_format() {
        assert_eq!(parse_cpu_time("0:12.34"), Some(12.34));
        assert_eq!(parse_cpu_time("1:02:03"), Some(3723.0));
        assert_eq!(parse_cpu_time("2-01:00:00"), Some(2.0 * 86_400.0 + 3600.0));
        assert_eq!(parse_cpu_time("abc"), None);
        assert_eq!(parse_time_rss("  0:01.50 204800"), Some(Sample { cpu_s: 1.5, rss_kb: 204_800 }));
        assert_eq!(parse_time_rss("0:01.50"), None);
    }

    #[test]
    fn proc_stat_has_user_and_system_time() {
        let stat = "1234 (my (term) x) S 1 1234 1234 0 -1 4194560 100 0 0 0 250 50 0 0 20 0 8 0 1000 1 2";
        let s = parse_proc_stat(stat, "5000 2500 100 1 0 300 0");
        assert_eq!(s, Some(Sample { cpu_s: 3.0, rss_kb: 10_000 }));
        assert_eq!(parse_proc_stat("nonsense", "1 2"), None);
    }

    #[test]
    fn the_process_survives_the_file() {
        let t = Terminal { pid: 7, command: "kitty".to_string(), note: Some("tmux".to_string()) };
        let v = json::parse(&t.to_json()).expect("valid json");
        assert_eq!(Terminal::from_value(&v), Some(t));
        let plain = Terminal { pid: 8, command: "wezterm-gui".to_string(), note: None };
        assert_eq!(Terminal::from_value(&json::parse(&plain.to_json()).expect("valid json")), Some(plain));
    }
}
