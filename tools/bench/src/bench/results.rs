//! What a benchmark run produced, and its JSON file (`--save`, read back by `compare`).

use super::diag::{DIAG_VERSION, TestDiag};
use super::integrity::Check;
use super::json::{self, Value};
use super::procinfo::Terminal;
use super::sysinfo::Machine;
use super::workloads;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    MBps,
    Fps,
    Micros,
    /// Things done per second (`screenswap`'s swaps).
    PerSec,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::MBps => "MB/s",
            Unit::Fps => "fps",
            Unit::Micros => "µs",
            Unit::PerSec => "/s",
        }
    }

    /// The unit's name in a results file. The first three are what older files say, so they still
    /// read; `per_s` is new.
    pub fn json_name(self) -> &'static str {
        match self {
            Unit::PerSec => "per_s",
            other => other.label(),
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        [Unit::MBps, Unit::Fps, Unit::Micros, Unit::PerSec]
            .into_iter()
            .find(|u| u.json_name() == s || u.label() == s)
    }

    pub fn higher_is_better(self) -> bool {
        self != Unit::Micros
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestResult {
    pub name: String,
    pub unit: Unit,
    /// The median run, in `unit`. Zero when the test timed out.
    pub median: f64,
    pub min: f64,
    pub max: f64,
    /// Seconds the median run took (0 for latency).
    pub secs: f64,
    pub bytes: u64,
    pub timeout: bool,
    /// More numbers worth keeping: p90 and p99 for latency, MB/s for frames, writes per second for
    /// `agentstream`.
    pub extra: Vec<(String, f64)>,
    /// Where the time went, run by run (`None` for latency and for files saved before 1.2).
    pub diag: Option<TestDiag>,
    /// Whether the terminal parsed everything (`integrity`); `None` for latency, for a timeout and for
    /// files saved before the check existed.
    pub check: Option<Check>,
}

impl TestResult {
    /// Why this result has no number to compare: it timed out, or the terminal dropped input.
    pub fn unusable(&self) -> Option<&'static str> {
        if self.timeout {
            Some("timed out")
        } else if self.check.as_ref().is_some_and(Check::is_dropped) {
            Some("dropped input")
        } else {
            None
        }
    }

    pub fn extra(&self, key: &str) -> Option<f64> {
        self.extra.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }

    /// Whether the test sends a payload to be checked (`roundtrip` sends only questions).
    pub fn has_payload(&self) -> bool {
        self.unit != Unit::Micros
    }

    /// Whether the input check vouched for this number. `roundtrip` sends no payload, so there is
    /// nothing to vouch for; every other test is verified only when its check says so. A result the
    /// check could not decide for (it was off, the terminal can't answer, too few checkpoints), and one
    /// from a file saved before the check existed, is not.
    pub fn verified(&self) -> bool {
        !self.has_payload() || matches!(self.check, Some(Check::Verified { .. }))
    }

    /// The opposite of [`verified`](Self::verified), for a number that has one: shown with a dagger,
    /// and ranked only among results of the same kind.
    pub fn unverified(&self) -> bool {
        self.unusable().is_none() && !self.verified()
    }

    /// How many times better this is than `base` (above 1 is better, whichever way the unit goes).
    /// Only a number of the same kind can be compared: a verified one is never set against an unverified
    /// one, in either direction.
    pub fn better_than(&self, base: &TestResult) -> Option<f64> {
        let usable = |t: &TestResult| t.unusable().is_none() && t.median > 0.0;
        if !usable(self) || !usable(base) || self.verified() != base.verified() {
            return None;
        }
        Some(if self.unit.higher_is_better() { self.median / base.median } else { base.median / self.median })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Results {
    pub label: String,
    pub terminal: String,
    pub cols: u16,
    pub rows: u16,
    pub machine: Machine,
    pub date: String,
    pub version: String,
    pub size_mb: f64,
    pub runs: usize,
    /// The distance between checkpoints in bytes (`integrity`), `None` when the check was off or the
    /// file is older than the check.
    pub check_every: Option<u64>,
    /// The terminal's process, when it could be found.
    pub process: Option<Terminal>,
    pub tests: Vec<TestResult>,
}

impl Results {
    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        let mut field = |k: &str, v: String| s.push_str(&format!("  {}: {},\n", json::quote(k), v));
        field("format", json::quote("ttystorm"));
        field("format_version", FORMAT_VERSION.to_string());
        field("tool_version", json::quote(&self.version));
        field("label", json::quote(&self.label));
        field("terminal", json::quote(&self.terminal));
        field("cols", self.cols.to_string());
        field("rows", self.rows.to_string());
        let m = &self.machine;
        field(
            "machine",
            format!(
                "{{\"cpu\": {}, \"model\": {}, \"os\": {}, \"memory\": {}, \"cores\": {}}}",
                json::quote(&m.cpu),
                json::quote(&m.model),
                json::quote(&m.os),
                json::quote(&m.memory),
                m.cores
            ),
        );
        field("date", json::quote(&self.date));
        field("size_mb", json::number(self.size_mb));
        field("runs", self.runs.to_string());
        field("diag_version", DIAG_VERSION.to_string());
        field("check_every_bytes", self.check_every.map_or("null".to_string(), |n| n.to_string()));
        field("terminal_process", self.process.as_ref().map_or("null".to_string(), Terminal::to_json));
        s.push_str("  \"tests\": [\n");
        for (i, t) in self.tests.iter().enumerate() {
            let extra: Vec<String> =
                t.extra.iter().map(|(k, v)| format!("{}: {}", json::quote(k), json::number(*v))).collect();
            s.push_str(&format!(
                "    {{\"name\": {}, \"unit\": {}, \"median\": {}, \"min\": {}, \"max\": {}, \"secs\": {}, \
                 \"bytes\": {}, \"timeout\": {}, \"extra\": {{{}}}, \"diag\": {}, \"check\": {}}}{}\n",
                json::quote(&t.name),
                json::quote(t.unit.json_name()),
                json::number(t.median),
                json::number(t.min),
                json::number(t.max),
                json::number(t.secs),
                t.bytes,
                t.timeout,
                extra.join(", "),
                t.diag.as_ref().map_or("null".to_string(), TestDiag::to_json),
                t.check.as_ref().map_or("null".to_string(), Check::to_json),
                if i + 1 < self.tests.len() { "," } else { "" }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    pub fn from_json(text: &str) -> Result<Results, String> {
        let v = json::parse(text)?;
        // Files saved up to 0.1 say `termbench`, the tool's name then.
        if !matches!(v.get("format").and_then(Value::as_str), Some("ttystorm" | "termbench")) {
            return Err("not a ttystorm results file".to_string());
        }
        let text_of = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let num_of = |k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        let machine = match v.get("machine") {
            Some(m) => {
                let t = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                Machine {
                    cpu: t("cpu"),
                    model: t("model"),
                    os: t("os"),
                    memory: t("memory"),
                    cores: m.get("cores").and_then(Value::as_f64).unwrap_or(0.0) as usize,
                }
            }
            None => Machine::default(),
        };
        let mut tests = Vec::new();
        for t in v.get("tests").and_then(Value::as_arr).ok_or("no `tests` list")? {
            let n = |k: &str| t.get(k).and_then(Value::as_f64).unwrap_or(0.0);
            let unit_text = t.get("unit").and_then(Value::as_str).unwrap_or("");
            tests.push(TestResult {
                name: workloads::current_name(
                    t.get("name").and_then(Value::as_str).ok_or("a test has no name")?,
                )
                .to_string(),
                unit: Unit::parse(unit_text).ok_or_else(|| format!("unknown unit `{unit_text}`"))?,
                median: n("median"),
                min: n("min"),
                max: n("max"),
                secs: n("secs"),
                bytes: n("bytes") as u64,
                timeout: t.get("timeout").and_then(Value::as_bool).unwrap_or(false),
                extra: match t.get("extra") {
                    Some(Value::Obj(members)) => {
                        members.iter().filter_map(|(k, v)| v.as_f64().map(|x| (k.clone(), x))).collect()
                    }
                    _ => Vec::new(),
                },
                diag: t.get("diag").and_then(TestDiag::from_value),
                check: t.get("check").and_then(Check::from_value),
            });
        }
        let terminal = text_of("terminal");
        let label = Some(text_of("label")).filter(|l| !l.is_empty()).unwrap_or_else(|| terminal.clone());
        Ok(Results {
            label,
            terminal,
            cols: num_of("cols") as u16,
            rows: num_of("rows") as u16,
            machine,
            date: text_of("date"),
            version: text_of("tool_version"),
            size_mb: num_of("size_mb"),
            runs: num_of("runs") as usize,
            check_every: v.get("check_every_bytes").and_then(Value::as_f64).map(|n| n as u64),
            process: v.get("terminal_process").and_then(Terminal::from_value),
            tests,
        })
    }
}

#[cfg(test)]
pub fn sample(label: &str, scale: f64) -> Results {
    let t = |name: &str, unit: Unit, v: f64| TestResult {
        name: name.to_string(),
        unit,
        median: v * scale,
        min: v * scale * 0.95,
        max: v * scale * 1.05,
        secs: 0.5,
        bytes: 16_000_000,
        timeout: false,
        extra: vec![("p99".to_string(), v * 2.0)],
        diag: None,
        check: (unit != Unit::Micros).then_some(Check::Verified { checkpoints: 60 }),
    };
    Results {
        label: label.to_string(),
        terminal: "Term \"quoted\"".to_string(),
        cols: 120,
        rows: 40,
        machine: Machine {
            cpu: "Apple M3 Pro".into(),
            model: "Mac15,6".into(),
            os: "macOS 15.1".into(),
            memory: "36 GB".into(),
            cores: 12,
        },
        date: "2026-10-10".to_string(),
        version: "0.1.0".to_string(),
        size_mb: 16.0,
        runs: 3,
        check_every: None,
        process: None,
        tests: vec![
            t("logflood", Unit::MBps, 800.0),
            t("colorflood", Unit::MBps, 120.0),
            t("framerate", Unit::Fps, 90.0),
            t("roundtrip", Unit::Micros, 150.0),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_file_reads_back_the_same() {
        let r = sample("Wormhole", 1.0);
        let back = Results::from_json(&r.to_json()).expect("reads back");
        assert_eq!(back, r);
    }

    #[test]
    fn a_timeout_survives_the_file() {
        let mut r = sample("x", 1.0);
        r.tests[1].timeout = true;
        r.tests[1].median = 0.0;
        let back = Results::from_json(&r.to_json()).expect("reads back");
        assert!(back.tests[1].timeout);
    }

    #[test]
    fn other_json_is_refused() {
        assert!(Results::from_json("{\"a\": 1}").is_err());
        assert!(Results::from_json("nope").is_err());
        assert!(Results::from_json("{\"format\": \"ttystorm\"}").is_err());
        assert!(Results::from_json("{\"format\": \"termbench\"}").is_err());
    }

    #[test]
    fn per_second_has_its_own_name_and_old_names_still_read() {
        assert_eq!(Unit::PerSec.label(), "/s");
        for u in [Unit::MBps, Unit::Fps, Unit::Micros, Unit::PerSec] {
            assert_eq!(Unit::parse(u.json_name()), Some(u));
        }
        let mut r = sample("x", 1.0);
        r.tests[2].unit = Unit::PerSec;
        let json = r.to_json();
        assert!(json.contains("\"unit\": \"per_s\"") && json.contains("\"unit\": \"MB/s\""));
        assert_eq!(Results::from_json(&json).expect("reads back"), r);
        // A file from before the diagnostics has neither the process nor a diag.
        let old =
            json.replace("\"diag_version\"", "\"x_diag_version\"").replace("\"terminal_process\"", "\"x\"");
        assert!(Results::from_json(&old).expect("reads").process.is_none());
    }

    #[test]
    fn diagnostics_and_the_process_survive_the_file() {
        use super::super::compose::Composition;
        use super::super::diag::RunDiag;
        let mut r = sample("x", 1.0);
        r.process = Some(Terminal { pid: 9, command: "kitty".to_string(), note: None });
        r.tests[0].diag = Some(TestDiag {
            composition: Composition { bytes: 5, ..Composition::default() },
            runs: vec![RunDiag { secs: 1.0, timeline: vec![1.0, 2.0], ..RunDiag::default() }],
            median_run: 0,
        });
        assert_eq!(Results::from_json(&r.to_json()).expect("reads back"), r);
    }

    #[test]
    fn better_than_points_the_right_way() {
        let (a, b) = (sample("a", 1.0), sample("b", 2.0));
        assert_eq!(b.tests[0].better_than(&a.tests[0]), Some(2.0));
        // Latency: twice the microseconds is half as good.
        assert_eq!(b.tests[3].better_than(&a.tests[3]), Some(0.5));
    }
}
