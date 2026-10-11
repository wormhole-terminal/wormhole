//! What a benchmark run produced, and its JSON file (`--save`, read back by `compare`).

use super::json::{self, Value};
use super::sysinfo::Machine;
use super::workloads;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    MBps,
    Fps,
    Micros,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::MBps => "MB/s",
            Unit::Fps => "fps",
            Unit::Micros => "µs",
        }
    }

    pub fn parse(s: &str) -> Option<Unit> {
        [Unit::MBps, Unit::Fps, Unit::Micros].into_iter().find(|u| u.label() == s)
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
    /// More numbers worth keeping: p90 and p99 for latency, MB/s for frames.
    pub extra: Vec<(String, f64)>,
}

impl TestResult {
    pub fn extra(&self, key: &str) -> Option<f64> {
        self.extra.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
    }

    /// How many times better this is than `base` (above 1 is better, whichever way the unit goes).
    pub fn better_than(&self, base: &TestResult) -> Option<f64> {
        if self.timeout || base.timeout || self.median <= 0.0 || base.median <= 0.0 {
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
        s.push_str("  \"tests\": [\n");
        for (i, t) in self.tests.iter().enumerate() {
            let extra: Vec<String> =
                t.extra.iter().map(|(k, v)| format!("{}: {}", json::quote(k), json::number(*v))).collect();
            s.push_str(&format!(
                "    {{\"name\": {}, \"unit\": {}, \"median\": {}, \"min\": {}, \"max\": {}, \"secs\": {}, \
                 \"bytes\": {}, \"timeout\": {}, \"extra\": {{{}}}}}{}\n",
                json::quote(&t.name),
                json::quote(t.unit.label()),
                json::number(t.median),
                json::number(t.min),
                json::number(t.max),
                json::number(t.secs),
                t.bytes,
                t.timeout,
                extra.join(", "),
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
    fn better_than_points_the_right_way() {
        let (a, b) = (sample("a", 1.0), sample("b", 2.0));
        assert_eq!(b.tests[0].better_than(&a.tests[0]), Some(2.0));
        // Latency: twice the microseconds is half as good.
        assert_eq!(b.tests[3].better_than(&a.tests[3]), Some(0.5));
    }
}
