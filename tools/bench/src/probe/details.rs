//! `--details`: the transcript of every query as plain text, how each was decided, and the
//! findings. Made to be pasted to a person or handed to a coding agent as it is.

use super::model::{Decision, ProbeResults, Trace, WidthPass};

/// Microseconds as a short duration: `420 µs`, `1.2 ms`, `1.50 s`.
pub fn latency(micros: Option<u64>) -> String {
    match micros {
        None => "-".to_string(),
        Some(m) if m < 1000 => format!("{m} µs"),
        Some(m) if m < 1_000_000 => format!("{:.1} ms", m as f64 / 1000.0),
        Some(m) => format!("{:.2} s", m as f64 / 1e6),
    }
}

/// One query on one line: `name | sent … | received … | latency | decided`.
fn line(name: &str, t: &Trace, how: &str) -> String {
    if matches!(t.decided, Decision::Environment | Decision::Derived) && t.sent.is_empty() {
        return format!("{name} | not asked ({how}) | - | - | {}\n", t.decided.label());
    }
    let received = if t.received.is_empty() { "nothing" } else { t.received.as_str() };
    format!(
        "{name} | sent {} | received {received} | {} | {}\n",
        t.sent,
        latency(t.micros),
        t.decided.label()
    )
}

fn notes(t: &Trace) -> String {
    t.notes.iter().map(|n| format!("  ! {n}\n")).collect()
}

fn width_line(name: &str, which: &str, expected: u8, p: &WidthPass) -> String {
    let t = &p.trace;
    let got = p.width.map_or("no width".to_string(), |w| format!("width {w} (expected {expected})"));
    let received = if t.received.is_empty() { "nothing" } else { t.received.as_str() };
    format!(
        "{name} ({which}) | sent {} | before {} | after {} | received {received} | {got} | {} | {}\n{}",
        t.sent,
        if p.dsr_before.is_empty() { "-" } else { &p.dsr_before },
        if p.dsr_after.is_empty() { "-" } else { &p.dsr_after },
        latency(t.micros),
        t.decided.label(),
        notes(t)
    )
}

pub fn details(r: &ProbeResults) -> String {
    let mut s = String::new();
    s.push_str("DETAILS\n");
    s.push_str(
        "One line per query. Escapes: \\x1b is ESC, \\\\ is a backslash, \\u{..} a non-ASCII character. Every query\n\
         is sent with DA1 (\\x1b[c) after it. Latency runs from the request to the reply that decided it.\n\n",
    );
    for f in &r.features {
        s.push_str(&line(&f.name, &f.trace, &f.how));
        s.push_str(&notes(&f.trace));
    }
    s.push_str("\nUNICODE WIDTHS\n");
    for w in &r.widths {
        let name = format!("{} [{}]", w.name, w.codepoints);
        s.push_str(&width_line(&name, "as-is", w.expected, &w.as_is));
        if let Some(p) = &w.with_2027 {
            s.push_str(&width_line(&name, "mode 2027", w.expected, p));
        }
    }
    s.push_str("\nHOW A RESULT IS DECIDED\n");
    for d in Decision::ALL {
        s.push_str(&format!("  {:<12} {}\n", d.label(), d.explain()));
    }
    s.push_str("\nFINDINGS\n");
    for f in &r.findings {
        s.push_str(&format!("  - {f}\n"));
    }
    s
}

/// The bytes read from the terminal during the probe, one chunk per line (`--raw`).
pub fn raw_dump(chunks: &[String]) -> String {
    let mut s =
        String::from("# bytes received from the terminal, escaped; the time is since the probe started\n");
    for c in chunks {
        s.push_str(c);
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::super::model::sample;
    use super::*;

    #[test]
    fn latencies_pick_a_unit() {
        assert_eq!(latency(None), "-");
        assert_eq!(latency(Some(420)), "420 µs");
        assert_eq!(latency(Some(1234)), "1.2 ms");
        assert_eq!(latency(Some(1_500_000)), "1.50 s");
    }

    #[test]
    fn every_query_is_one_line_with_its_bytes() {
        let r = sample("a", &[(1, Some(1)), (2, Some(2))]);
        let text = details(&r);
        assert!(text.lines().any(|l| l.starts_with("synchronized output (2026) | sent \\x1b[?2026$p\\x1b[c | received \\x1b[?2026;2$y\\x1b[?64;4c | 420 µs | reply")), "{text}");
        assert!(text.contains("  ! a note"), "{text}");
        assert!(
            text.contains("case 0 [61] (as-is) | sent")
                && text.contains("before \\x1b[2;1R | after \\x1b[2;3R"),
            "{text}"
        );
        assert!(text.contains("case 0 [61] (mode 2027)"), "{text}");
        assert!(text.contains("FINDINGS\n  - a finding"), "{text}");
        assert!(!text.contains('\x1b'), "details are escaped, never raw");
    }

    #[test]
    fn raw_dumps_keep_each_chunk() {
        let d = raw_dump(&["+1.000 ms  \\x1b[c".to_string()]);
        assert!(d.starts_with('#') && d.ends_with("\\x1b[c\n"));
    }
}
