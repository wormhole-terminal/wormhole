//! What a probe found, and its JSON file (`--save`, read back by `compare`). Every row and width
//! case keeps its transcript – what was sent, the raw reply, how long it took and how the answer was
//! decided – so a surprising result can be traced without running the probe again.

use crate::bench::json::{self, Value};
use crate::bench::sysinfo::Machine;

pub const FORMAT: &str = "ttystorm-probe";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Identity,
    Modes,
    Graphics,
    Colors,
    Sizes,
}

impl Section {
    pub const ALL: [Section; 5] =
        [Section::Identity, Section::Modes, Section::Graphics, Section::Colors, Section::Sizes];

    pub fn label(self) -> &'static str {
        match self {
            Section::Identity => "Identity",
            Section::Modes => "Modes",
            Section::Graphics => "Graphics",
            Section::Colors => "Colors",
            Section::Sizes => "Sizes",
        }
    }

    pub fn parse(s: &str) -> Option<Section> {
        Section::ALL.into_iter().find(|x| x.label() == s)
    }
}

/// What a row found out. `Unknown` means the terminal never answered, so nothing is known; `No` is
/// a terminal that answered (at least DA1) without the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Yes,
    No,
    Value(String),
    Unknown,
}

impl Outcome {
    pub fn status(&self) -> &'static str {
        match self {
            Outcome::Yes => "yes",
            Outcome::No => "no",
            Outcome::Value(_) => "value",
            Outcome::Unknown => "unknown",
        }
    }
}

/// How a row was decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The terminal's reply arrived and was read.
    Reply,
    /// The DA1 reply came first, without the answer: the terminal doesn't know the query.
    Sentinel,
    /// Neither the answer nor DA1 came in time.
    Timeout,
    /// Something came back but not in the shape the query expects.
    Unparsed,
    /// Not asked: an earlier query had already gone unanswered.
    Skipped,
    /// Read from the environment, not asked.
    Environment,
    /// Worked out from another row's reply.
    Derived,
}

impl Decision {
    pub const ALL: [Decision; 7] = [
        Decision::Reply,
        Decision::Sentinel,
        Decision::Timeout,
        Decision::Unparsed,
        Decision::Skipped,
        Decision::Environment,
        Decision::Derived,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Decision::Reply => "reply",
            Decision::Sentinel => "sentinel",
            Decision::Timeout => "timeout",
            Decision::Unparsed => "unparsed",
            Decision::Skipped => "skipped",
            Decision::Environment => "environment",
            Decision::Derived => "derived",
        }
    }

    pub fn parse(s: &str) -> Option<Decision> {
        Decision::ALL.into_iter().find(|d| d.label() == s)
    }

    /// The same, in words.
    pub fn explain(self) -> &'static str {
        match self {
            Decision::Reply => "the terminal's reply arrived and was read",
            Decision::Sentinel => "DA1 came back first, without an answer: not supported",
            Decision::Timeout => "no answer and no DA1 within the time limit",
            Decision::Unparsed => "a reply arrived but not in the expected shape",
            Decision::Skipped => "not asked: an earlier query had no answer",
            Decision::Environment => "read from the environment",
            Decision::Derived => "worked out from another row",
        }
    }
}

/// The exchange behind one result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    /// The bytes sent, escaped (`\x1b[?2026$p\x1b[c`).
    pub sent: String,
    /// Everything read while waiting, escaped.
    pub received: String,
    /// Microseconds from the request to the reply that decided it (the timeout, when none did).
    pub micros: Option<u64>,
    pub decided: Decision,
    /// Anything unexpected: out-of-order or unrequested replies, bytes left over from earlier.
    pub notes: Vec<String>,
}

impl Trace {
    /// A result that needed no exchange.
    pub fn without_query(decided: Decision) -> Trace {
        Trace { sent: String::new(), received: String::new(), micros: None, decided, notes: Vec::new() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub section: Section,
    pub name: String,
    pub outcome: Outcome,
    /// A detail shown beside a ✓ or ✗: the mode's state, the Kitty flags.
    pub note: String,
    /// How it was asked, in short (`DECRQM 2026`).
    pub how: String,
    pub trace: Trace,
}

/// One measurement of a width case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidthPass {
    /// Columns the text advanced the cursor, `None` when the cursor reports didn't come.
    pub width: Option<u8>,
    /// The raw cursor reports before and after writing the text, escaped.
    pub dsr_before: String,
    pub dsr_after: String,
    pub trace: Trace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidthResult {
    pub name: String,
    /// `1F468 200D 1F469`: what the text is made of.
    pub codepoints: String,
    pub expected: u8,
    /// Not counted in the score: Unicode and terminals don't agree on it.
    pub disputed: bool,
    pub as_is: WidthPass,
    /// The same with mode 2027 set, when the terminal has that mode.
    pub with_2027: Option<WidthPass>,
}

impl WidthResult {
    /// The first measurement, or the one with mode 2027 set.
    pub fn pass(&self, with_2027: bool) -> Option<&WidthPass> {
        if with_2027 { self.with_2027.as_ref() } else { Some(&self.as_is) }
    }

    pub fn matches(width: Option<u8>, expected: u8) -> bool {
        width == Some(expected)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProbeResults {
    pub label: String,
    pub terminal: String,
    pub cols: u16,
    pub rows: u16,
    pub machine: Machine,
    pub date: String,
    pub version: String,
    pub features: Vec<Row>,
    pub widths: Vec<WidthResult>,
    /// What the results mean, in plain words.
    pub findings: Vec<String>,
    /// Unexpected things not tied to one row.
    pub events: Vec<String>,
}

impl ProbeResults {
    pub fn feature(&self, name: &str) -> Option<&Row> {
        self.features.iter().find(|r| r.name == name)
    }

    /// How many of the counted width cases match, as `(matched, counted)`; `None` when nothing was
    /// measured. `with_2027` scores the second pass.
    pub fn score(&self, with_2027: bool) -> Option<(usize, usize)> {
        if !self.widths.iter().any(|w| w.pass(with_2027).is_some_and(|p| p.width.is_some())) {
            return None;
        }
        let counted: Vec<&WidthResult> = self.widths.iter().filter(|w| !w.disputed).collect();
        let matched = counted
            .iter()
            .filter(|w| WidthResult::matches(w.pass(with_2027).and_then(|p| p.width), w.expected))
            .count();
        Some((matched, counted.len()))
    }

    pub fn has_2027_pass(&self) -> bool {
        self.widths.iter().any(|w| w.with_2027.is_some())
    }

    pub fn to_json(&self) -> String {
        let mut s = String::from("{\n");
        let mut field = |k: &str, v: String| s.push_str(&format!("  {}: {},\n", json::quote(k), v));
        field("format", json::quote(FORMAT));
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
        field("score_as_is", score_json(self.score(false)));
        field("score_2027", score_json(self.score(true)));
        field("findings", string_list(&self.findings));
        field("events", string_list(&self.events));
        s.push_str("  \"features\": [\n");
        for (i, r) in self.features.iter().enumerate() {
            let value = if let Outcome::Value(v) = &r.outcome { v.as_str() } else { "" };
            s.push_str(&format!(
                "    {{\"section\": {}, \"name\": {}, \"status\": {}, \"value\": {}, \"note\": {}, \"how\": {}, {}}}{}\n",
                json::quote(r.section.label()),
                json::quote(&r.name),
                json::quote(r.outcome.status()),
                json::quote(value),
                json::quote(&r.note),
                json::quote(&r.how),
                trace_fields(&r.trace),
                if i + 1 < self.features.len() { "," } else { "" }
            ));
        }
        s.push_str("  ],\n  \"widths\": [\n");
        for (i, w) in self.widths.iter().enumerate() {
            let second = w.with_2027.as_ref().map_or("null".to_string(), pass_json);
            s.push_str(&format!(
                "    {{\"name\": {}, \"codepoints\": {}, \"expected\": {}, \"disputed\": {}, \"measured\": {}, \
                 \"measured_2027\": {}, \"as_is\": {}, \"with_2027\": {}}}{}\n",
                json::quote(&w.name),
                json::quote(&w.codepoints),
                w.expected,
                w.disputed,
                width_json(w.as_is.width),
                width_json(w.with_2027.as_ref().and_then(|p| p.width)),
                pass_json(&w.as_is),
                second,
                if i + 1 < self.widths.len() { "," } else { "" }
            ));
        }
        s.push_str("  ]\n}\n");
        s
    }

    pub fn from_json(text: &str) -> Result<ProbeResults, String> {
        let v = json::parse(text)?;
        if v.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err("not a ttystorm probe file".to_string());
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
        let mut features = Vec::new();
        for r in v.get("features").and_then(Value::as_arr).ok_or("no `features` list")? {
            let t = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let name = r.get("name").and_then(Value::as_str).ok_or("a feature has no name")?;
            let section =
                Section::parse(&t("section")).ok_or_else(|| format!("unknown section in `{name}`"))?;
            let outcome = match t("status").as_str() {
                "yes" => Outcome::Yes,
                "no" => Outcome::No,
                "value" => Outcome::Value(t("value")),
                "unknown" => Outcome::Unknown,
                other => return Err(format!("unknown status `{other}` in `{name}`")),
            };
            features.push(Row {
                section,
                name: name.to_string(),
                outcome,
                note: t("note"),
                how: t("how"),
                trace: trace_of(r),
            });
        }
        let mut widths = Vec::new();
        for w in v.get("widths").and_then(Value::as_arr).ok_or("no `widths` list")? {
            let t = |k: &str| w.get(k).and_then(Value::as_str).unwrap_or("").to_string();
            let as_is =
                w.get("as_is").map(pass_of).unwrap_or_else(|| empty_pass(width_of(w.get("measured"))));
            let with_2027 = match w.get("with_2027") {
                Some(p @ Value::Obj(_)) => Some(pass_of(p)),
                _ => None,
            };
            widths.push(WidthResult {
                name: w.get("name").and_then(Value::as_str).ok_or("a width case has no name")?.to_string(),
                codepoints: t("codepoints"),
                expected: w.get("expected").and_then(Value::as_f64).unwrap_or(0.0) as u8,
                disputed: w.get("disputed").and_then(Value::as_bool).unwrap_or(false),
                as_is,
                with_2027,
            });
        }
        let strings = |k: &str| -> Vec<String> {
            v.get(k)
                .and_then(Value::as_arr)
                .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default()
        };
        let terminal = text_of("terminal");
        let label = Some(text_of("label")).filter(|l| !l.is_empty()).unwrap_or_else(|| terminal.clone());
        Ok(ProbeResults {
            label,
            terminal,
            cols: num_of("cols") as u16,
            rows: num_of("rows") as u16,
            machine,
            date: text_of("date"),
            version: text_of("tool_version"),
            features,
            widths,
            findings: strings("findings"),
            events: strings("events"),
        })
    }
}

fn score_json(score: Option<(usize, usize)>) -> String {
    match score {
        Some((m, n)) => format!("{{\"matched\": {m}, \"counted\": {n}}}"),
        None => "null".to_string(),
    }
}

fn string_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|s| json::quote(s)).collect::<Vec<_>>().join(", "))
}

fn width_json(w: Option<u8>) -> String {
    w.map_or("null".to_string(), |n| n.to_string())
}

fn width_of(v: Option<&Value>) -> Option<u8> {
    v.and_then(Value::as_f64).map(|n| n as u8)
}

/// The fields every transcript has, as the inside of a JSON object.
fn trace_fields(t: &Trace) -> String {
    format!(
        "\"decided\": {}, \"sent\": {}, \"received\": {}, \"micros\": {}, \"notes\": {}",
        json::quote(t.decided.label()),
        json::quote(&t.sent),
        json::quote(&t.received),
        t.micros.map_or("null".to_string(), |m| m.to_string()),
        string_list(&t.notes)
    )
}

fn trace_of(v: &Value) -> Trace {
    let t = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    Trace {
        sent: t("sent"),
        received: t("received"),
        micros: v.get("micros").and_then(Value::as_f64).map(|m| m as u64),
        decided: Decision::parse(&t("decided")).unwrap_or(Decision::Unparsed),
        notes: v
            .get("notes")
            .and_then(Value::as_arr)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    }
}

fn pass_json(p: &WidthPass) -> String {
    format!(
        "{{\"width\": {}, \"dsr_before\": {}, \"dsr_after\": {}, {}}}",
        width_json(p.width),
        json::quote(&p.dsr_before),
        json::quote(&p.dsr_after),
        trace_fields(&p.trace)
    )
}

fn pass_of(v: &Value) -> WidthPass {
    let t = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    WidthPass {
        width: width_of(v.get("width")),
        dsr_before: t("dsr_before"),
        dsr_after: t("dsr_after"),
        trace: trace_of(v),
    }
}

fn empty_pass(width: Option<u8>) -> WidthPass {
    WidthPass {
        width,
        dsr_before: String::new(),
        dsr_after: String::new(),
        trace: Trace::without_query(Decision::Derived),
    }
}

#[cfg(test)]
pub fn sample(label: &str, widths: &[(u8, Option<u8>)]) -> ProbeResults {
    let trace = |d| Trace {
        sent: "\\x1b[?2026$p\\x1b[c".to_string(),
        received: "\\x1b[?2026;2$y\\x1b[?64;4c".to_string(),
        micros: Some(420),
        decided: d,
        notes: vec!["a note".to_string()],
    };
    let row = |section, name: &str, outcome| Row {
        section,
        name: name.to_string(),
        outcome,
        note: String::new(),
        how: "DECRQM".to_string(),
        trace: trace(Decision::Reply),
    };
    let pass = |width| WidthPass {
        width,
        dsr_before: "\\x1b[2;1R".to_string(),
        dsr_after: "\\x1b[2;3R".to_string(),
        trace: trace(Decision::Reply),
    };
    ProbeResults {
        label: label.to_string(),
        terminal: "TestTerm 1.0".to_string(),
        cols: 120,
        rows: 40,
        machine: Machine {
            cpu: "Apple M3".into(),
            model: "Mac".into(),
            os: "macOS 15".into(),
            memory: "16 GB".into(),
            cores: 8,
        },
        date: "2026-10-11".to_string(),
        version: "1.2.0".to_string(),
        features: vec![
            row(Section::Identity, "terminal (XTVERSION)", Outcome::Value(format!("{label} 1.0"))),
            row(Section::Modes, "synchronized output (2026)", Outcome::Yes),
            row(
                Section::Modes,
                "grapheme clustering (2027)",
                if label == "b" { Outcome::No } else { Outcome::Yes },
            ),
            row(Section::Graphics, "Sixel", Outcome::No),
            row(
                Section::Colors,
                "background (OSC 11)",
                Outcome::Value(if label == "b" { "#ffffff" } else { "#101010" }.into()),
            ),
            row(Section::Sizes, "size in cells (CSI 18 t)", Outcome::Unknown),
        ],
        widths: widths
            .iter()
            .enumerate()
            .map(|(i, (expected, measured))| WidthResult {
                name: format!("case {i}"),
                codepoints: format!("{:X}", 0x61 + i),
                expected: *expected,
                disputed: i == 1,
                as_is: pass(*measured),
                with_2027: Some(pass(Some(*expected))),
            })
            .collect(),
        findings: vec!["a finding".to_string()],
        events: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_probe_reads_back_with_its_transcripts() {
        let r = sample("a", &[(1, Some(1)), (2, Some(1)), (2, Some(2)), (0, None)]);
        let back = ProbeResults::from_json(&r.to_json()).expect("valid");
        assert_eq!(back, r);
    }

    #[test]
    fn the_score_skips_disputed_cases() {
        let r = sample("a", &[(1, Some(1)), (2, Some(1)), (2, Some(2)), (0, None)]);
        // Case 1 is disputed; of the other three two match as-is.
        assert_eq!(r.score(false), Some((2, 3)));
        assert_eq!(r.score(true), Some((3, 3)));
        let none = sample("a", &[(1, None), (2, None)]);
        assert_eq!(none.score(false), None);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(ProbeResults::from_json("{\"format\": \"ttystorm\"}").is_err());
        assert!(ProbeResults::from_json("[]").is_err());
    }
}
