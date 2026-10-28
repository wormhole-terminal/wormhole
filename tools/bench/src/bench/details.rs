//! `--details`: after the results screen, a plain-text section per test on why it was as fast or as
//! slow as it was, for a person or a coding agent to read. The same numbers are in the `--save`
//! file; the words here are `diag::findings`.

use super::compose::Composition;
use super::diag::{self, RunDiag, STALL_MS};
use super::integrity::{Check, QUERY_BYTES};
use super::results::{Results, TestResult};

const SPARK_WIDTH: usize = 60;

fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn share(x: f64, total: f64) -> u32 {
    if total > 0.0 { (x / total * 100.0).round() as u32 } else { 0 }
}

fn composition_lines(c: &Composition) -> Vec<String> {
    let text_pct = share(c.text_bytes as f64, c.bytes as f64);
    let kinds = [
        ("SGR", c.sgr),
        ("cursor moves", c.cursor),
        ("erases", c.erase),
        ("scroll/edit", c.scroll),
        ("OSC", c.osc),
        ("DCS", c.dcs),
        ("mode sets", c.modes),
        ("other", c.other),
    ];
    let seq: Vec<String> =
        kinds.iter().filter(|(_, n)| *n > 0).map(|(k, n)| format!("{} {k}", thousands(*n))).collect();
    vec![
        format!(
            "payload: {} bytes in {} writes (avg {:.0} bytes): {}% text, {}% escape sequences",
            thousands(c.bytes),
            thousands(c.writes),
            c.avg_write(),
            text_pct,
            100 - text_pct.min(100)
        ),
        format!(
            "         {} lines, {} characters ({} wide); {} sequences: {}",
            thousands(c.lines),
            thousands(c.chars),
            thousands(c.wide),
            thousands(c.sequences()),
            if seq.is_empty() { "no sequences".to_string() } else { seq.join(", ") }
        ),
    ]
}

fn run_lines(d: &RunDiag, mb: f64) -> Vec<String> {
    let other = (d.secs - d.write_blocked_s - d.dsr_wait_s).max(0.0);
    let mut v = vec![
        format!(
            "time: {:.3} s = writing {:.3} s ({}%) + waiting for the answer after the last byte {:.3} s ({}%) + other {:.3} s",
            d.secs,
            d.write_blocked_s,
            share(d.write_blocked_s, d.secs),
            d.dsr_wait_s,
            share(d.dsr_wait_s, d.secs),
            other
        ),
        format!(
            "writes: {} calls; each took p50 {:.0} us, p99 {:.0} us, max {:.1} ms",
            thousands(d.writes as u64),
            d.write_p50_us,
            d.write_p99_us,
            d.write_max_us / 1e3
        ),
    ];
    if d.stalls == 0 {
        v.push(format!("stalls: none over {STALL_MS:.0} ms"));
    } else {
        let at: Vec<String> =
            d.longest.iter().map(|s| format!("{:.0} ms at {:.2} s", s.ms, s.at_s)).collect();
        v.push(format!(
            "stalls: {} writes over {STALL_MS:.0} ms, {:.3} s in all; longest: {}",
            d.stalls,
            d.stalled_s,
            at.join(", ")
        ));
    }
    if !d.timeline.is_empty() {
        let top = d.timeline.iter().cloned().fold(0.0f64, f64::max);
        v.push(format!(
            "timeline: {} (MB/s per {:.0} ms, peak {:.0}, {} buckets)",
            diag::sparkline(&d.timeline, SPARK_WIDTH),
            diag::BUCKET_MS,
            top,
            d.timeline.len()
        ));
    }
    match (d.cpu_s, d.cpu_share()) {
        (Some(cpu), Some(sh)) => {
            let growth = d.rss_growth_kb().map_or(String::new(), |kb| format!(", memory {:+} MB", kb / 1024));
            v.push(format!(
                "terminal: {cpu:.2} s of CPU = {:.0}% of wall time, {:.3} s per MB{growth}",
                sh * 100.0,
                cpu / mb.max(1e-9)
            ));
        }
        _ => v.push("terminal: CPU and memory not measured".to_string()),
    }
    v
}

/// What the input check found for a test, in words, and what it cost.
fn check_lines(t: &TestResult, r: &Results) -> Vec<String> {
    let Some(check) = &t.check else {
        return vec![
            "input check: not made (this test has none, or the file is from before the check)".to_string(),
        ];
    };
    // The distance the checkpoints really had, which a small payload or one with few places for a query
    // makes shorter or longer than the one asked for.
    let every = match (t.extra("checkpoint_distance"), r.check_every) {
        (Some(d), _) => format!(", about one per {:.0} KiB", d / 1024.0),
        (None, Some(n)) => format!(", asked for one per {} KiB", n / 1024),
        (None, None) => String::new(),
    };
    let mut lines = match check {
        Check::Verified { checkpoints } => vec![
            format!(
                "input check: verified, {checkpoints} checkpoints in {} runs{every}, all answered",
                r.runs
            ),
            format!(
                "  cost: {} bytes sent and {} received per run besides the payload, one terminal reply each; \
                 to see what it costs this terminal, run again with --no-verify and compare",
                QUERY_BYTES * checkpoints / r.runs.max(1) as u64,
                QUERY_BYTES * checkpoints / r.runs.max(1) as u64,
            ),
        ],
        Check::Dropped { answered, checkpoints } => vec![
            format!("input check: DROPPED INPUT, {answered} of {checkpoints} checkpoints answered{every}"),
            "  The terminal read the bytes but did not parse all of them (or did not answer for them), so the time \
             measured how fast it discarded input, and no number is given."
                .to_string(),
        ],
        Check::Unchecked(why) => vec![format!("input check: not verified, {}", why.sentence())],
    };
    if let Some(n) = t.extra("checkpoints_unwritten") {
        lines.push(format!(
            "  note: {n:.0} planned checkpoints were never written and are not held against the terminal"
        ));
    }
    lines
}

fn test_section(t: &TestResult, r: &Results) -> Option<String> {
    let d = t.diag.as_ref()?;
    let run = d.median()?;
    let mb = t.bytes as f64 / 1e6;
    let mut s = format!(
        "{}  {:.1} {}  (run {} of {} shown, the median)\n",
        t.name,
        t.median,
        t.unit.label(),
        d.median_run + 1,
        d.runs.len()
    );
    for l in composition_lines(&d.composition).into_iter().chain(run_lines(run, mb)).chain(check_lines(t, r))
    {
        s.push_str("  ");
        s.push_str(&l);
        s.push('\n');
    }
    s.push_str("  findings:\n");
    for f in diag::findings(run, &d.composition, r.process.as_ref()) {
        s.push_str("    - ");
        s.push_str(&f);
        s.push('\n');
    }
    Some(s)
}

/// The diagnostics of every test that has them, as text.
pub fn details(r: &Results) -> String {
    let mut s = String::from("\nDETAILS\n");
    match &r.process {
        Some(p) => s.push_str(&format!(
            "terminal process: {} (pid {}){}\n",
            p.command,
            p.pid,
            p.note
                .as_ref()
                .map_or(String::new(), |n| format!(", behind {n}: the numbers below describe {n}"))
        )),
        None => s.push_str("terminal process: not found, so no CPU or memory figures\n"),
    }
    s.push_str("The per-write timings are the writer's view: a write returns when the terminal has made room in the\n");
    s.push_str("PTY buffer, so time blocked in write() is time the terminal was not reading.\n");
    for t in &r.tests {
        if let Some(section) = test_section(t, r) {
            s.push('\n');
            s.push_str(&section);
        } else if t.check.as_ref().is_some_and(Check::is_dropped) {
            // No diagnostics for a run that was stopped, but the finding is the point.
            s.push_str(&format!("\n{}  dropped input\n", t.name));
            for l in check_lines(t, r) {
                s.push_str(&format!("  {l}\n"));
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::super::diag::{Stall, TestDiag};
    use super::super::procinfo::Terminal;
    use super::super::results::sample;
    use super::*;

    fn with_diag() -> Results {
        let mut r = sample("Wormhole", 1.0);
        r.process = Some(Terminal { pid: 77, command: "wormhole".into(), note: None });
        r.tests[0].diag = Some(TestDiag {
            composition: Composition {
                bytes: 1000,
                text_bytes: 700,
                escape_bytes: 300,
                sgr: 40,
                writes: 2,
                lines: 9,
                ..Composition::default()
            },
            runs: vec![RunDiag {
                secs: 2.0,
                write_blocked_s: 1.7,
                dsr_wait_s: 0.1,
                writes: 250,
                stalls: 1,
                stalled_s: 0.3,
                longest: vec![Stall { at_s: 1.0, ms: 300.0 }],
                timeline: vec![10.0, 20.0, 40.0],
                cpu_s: Some(0.5),
                rss_start_kb: Some(1024),
                rss_end_kb: Some(3072),
                ..RunDiag::default()
            }],
            median_run: 0,
        });
        r
    }

    #[test]
    fn a_test_with_diagnostics_gets_a_section_with_findings() {
        let text = details(&with_diag());
        assert!(text.contains("terminal process: wormhole (pid 77)"), "{text}");
        assert!(text.contains("logflood  800.0 MB/s"), "{text}");
        assert!(text.contains("70% text, 30% escape sequences") && text.contains("40 SGR"), "{text}");
        assert!(
            text.contains("writing 1.700 s (85%)") && text.contains("longest: 300 ms at 1.00 s"),
            "{text}"
        );
        assert!(text.contains("timeline: ▃▅█") && text.contains("memory +2 MB"), "{text}");
        assert!(text.contains("read-bound: the writer waited for the terminal 85% of the time"), "{text}");
        // Tests without diagnostics are left out.
        assert!(!text.contains("roundtrip"), "{text}");
    }

    #[test]
    fn numbers_get_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(100), "100");
    }

    #[test]
    fn a_missing_process_is_said() {
        let mut r = with_diag();
        r.process = None;
        assert!(details(&r).contains("not found"));
    }

    #[test]
    fn unwritten_checkpoints_are_noted_not_held_against_the_terminal() {
        let mut r = with_diag();
        r.tests[0].check = Some(Check::Verified { checkpoints: 48 });
        r.tests[0].extra.push(("checkpoints_unwritten".to_string(), 2.0));
        assert!(details(&r).contains("2 planned checkpoints were never written"));
    }

    #[test]
    fn the_input_check_is_told_in_words_with_its_cost() {
        use super::super::integrity::Why;
        let mut r = with_diag();
        r.check_every = Some(256 * 1024);
        r.tests[0].check = Some(Check::Verified { checkpoints: 48 });
        r.tests[0].extra.push(("checkpoint_distance".to_string(), 300_000.0));
        let text = details(&r);
        assert!(
            text.contains("input check: verified, 48 checkpoints in 3 runs, about one per 293 KiB"),
            "{text}"
        );
        assert!(text.contains("--no-verify"), "{text}");
        r.tests[0].check = Some(Check::Unchecked(Why::Silent));
        assert!(details(&r).contains("not verified, the terminal does not answer DSR 5"));
        // A dropped run has no diagnostics but is still told.
        r.tests[0].diag = None;
        r.tests[0].check = Some(Check::Dropped { answered: 3, checkpoints: 48 });
        let text = details(&r);
        assert!(text.contains("logflood  dropped input") && text.contains("3 of 48"), "{text}");
    }
}
