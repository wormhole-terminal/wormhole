//! What the results mean, in plain words: why a feature reads as unsupported, what the terminal
//! actually answered, what to try next. For a person reading `--details` and for a coding agent
//! handed the saved JSON.

use super::model::{Decision, Outcome, ProbeResults, Section};

/// A list of names for a sentence: `a, b and c`.
fn names(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

fn value_of<'a>(r: &'a ProbeResults, name: &str) -> &'a str {
    match r.feature(name).map(|f| &f.outcome) {
        Some(Outcome::Value(v)) => v,
        _ => "",
    }
}

pub fn findings(r: &ProbeResults) -> Vec<String> {
    let mut out = Vec::new();
    let da1 = r.feature("device attributes (DA1)");
    let silent = da1.is_some_and(|f| {
        matches!(f.trace.decided, Decision::Timeout | Decision::Unparsed | Decision::Skipped)
    }) && da1.is_some_and(|f| f.outcome == Outcome::Unknown);
    if silent {
        out.push(
            "No reply at all within 1 s: probably not a real terminal (output is piped or redirected, or this is \
             a recording or CI shell), or a multiplexer that swallows queries. Try again in the terminal window \
             itself, outside tmux or screen."
                .to_string(),
        );
        return out;
    }

    let mux = value_of(r, "multiplexer");
    if !mux.is_empty() && mux != "none" {
        out.push(format!(
            "Running inside {mux}: it answers some queries itself and swallows others, so these results describe \
             {mux} together with the terminal. Run outside {mux} to test the terminal alone."
        ));
    }

    let mode_rows = || r.features.iter().filter(|f| f.section == Section::Modes);
    let not_reported: Vec<String> =
        mode_rows().filter(|f| f.trace.decided == Decision::Sentinel).map(|f| f.name.clone()).collect();
    if !not_reported.is_empty() {
        out.push(format!(
            "{} not reported (DA1 came back with no answer): the terminal may still support {} without \
             answering the query, so ✗ means \"not announced\", not \"proven absent\".",
            names(&not_reported),
            if not_reported.len() == 1 { "it" } else { "them" }
        ));
    }
    let not_recognized: Vec<String> = mode_rows()
        .filter(|f| f.outcome == Outcome::No && f.note == "not recognized")
        .map(|f| f.name.clone())
        .collect();
    if !not_recognized.is_empty() {
        out.push(format!(
            "{} answered DECRPM 0: the terminal says it does not know the mode.",
            names(&not_recognized)
        ));
    }

    let xt = r.feature("terminal (XTVERSION)");
    if xt.is_some_and(|f| f.trace.decided == Decision::Sentinel) {
        let env = value_of(r, "TERM_PROGRAM");
        out.push(format!(
            "XTVERSION was not answered, so the terminal can't be told from its reply; the name comes from $TERM_PROGRAM ({}).",
            if env.is_empty() { "unset" } else { env }
        ));
    }

    let colorterm = value_of(r, "COLORTERM");
    let tc = r.feature("truecolor (DECRQSS)");
    if tc.is_some_and(|f| f.outcome != Outcome::Yes) && matches!(colorterm, "truecolor" | "24bit") {
        out.push(
            "$COLORTERM says truecolor but the DECRQSS check could not confirm it: many terminals don't implement \
             DECRQSS for colors, so this may be a false alarm."
                .to_string(),
        );
    }
    if let Some(g) = r.feature("Kitty graphics")
        && g.note.starts_with("answered:")
    {
        out.push(format!(
            "Kitty graphics {}: the protocol is there but rejected the one-pixel test image.",
            g.note
        ));
    }

    width_findings(r, &mut out);

    let by = |d: Decision| -> Vec<String> {
        r.features.iter().filter(|f| f.trace.decided == d).map(|f| f.name.clone()).collect()
    };
    let timed_out = by(Decision::Timeout);
    if !timed_out.is_empty() {
        out.push(format!(
            "{} got neither an answer nor DA1 in time: the terminal stopped answering, or took longer than the limit.",
            names(&timed_out)
        ));
    }
    let odd = by(Decision::Unparsed);
    if !odd.is_empty() {
        out.push(format!(
            "{} got a reply that doesn't have the expected shape: compare `received` with `sent` in the transcript.",
            names(&odd)
        ));
    }
    let skipped = by(Decision::Skipped);
    if !skipped.is_empty() {
        out.push(format!(
            "{} {} not asked because earlier queries got no DA1 reply.",
            names(&skipped),
            if skipped.len() == 1 { "was" } else { "were" }
        ));
    }

    let slow: Vec<String> = r
        .features
        .iter()
        .filter(|f| matches!(f.trace.decided, Decision::Reply | Decision::Sentinel))
        .filter(|f| f.trace.micros.is_some_and(|m| m > 100_000))
        .map(|f| format!("{} ({} ms)", f.name, f.trace.micros.unwrap_or(0) / 1000))
        .collect();
    if !slow.is_empty() {
        out.push(format!("Slow answers (over 100 ms): {}.", names(&slow)));
    }

    let with_notes = r
        .features
        .iter()
        .map(|f| &f.trace)
        .chain(
            r.widths
                .iter()
                .flat_map(|w| std::iter::once(&w.as_is.trace).chain(w.with_2027.as_ref().map(|p| &p.trace))),
        )
        .filter(|t| t.notes.iter().any(|n| !n.starts_with("DA1 was never answered")))
        .count();
    if with_notes > 0 {
        out.push(format!(
            "{with_notes} exchange(s) had something unexpected (out-of-order or unrequested replies, leftover bytes): \
             see their notes in --details or the JSON."
        ));
    }
    for e in &r.events {
        out.push(e.clone());
    }
    if out.is_empty() {
        out.push(
            "Nothing unexpected: every query was answered or cleanly reported as unsupported.".to_string(),
        );
    }
    out
}

fn width_findings(r: &ProbeResults, out: &mut Vec<String>) {
    let measured = r.widths.iter().any(|w| w.as_is.width.is_some());
    if !measured {
        if !r.widths.is_empty() {
            out.push(
                "The cursor position report (DSR 6) did not come back, so no text width could be measured."
                    .to_string(),
            );
        }
        return;
    }
    let mode = r.feature("grapheme clustering (2027)");
    let has_2027 = r.has_2027_pass();
    let wrong: Vec<String> = r
        .widths
        .iter()
        .filter(|w| !w.disputed && w.as_is.width.is_some() && w.as_is.width != Some(w.expected))
        .map(|w| format!("{} drawn {} wide, expected {}", w.name, w.as_is.width.unwrap_or(0), w.expected))
        .collect();
    if !wrong.is_empty() {
        out.push(format!("Widths that differ from Unicode's grapheme clusters: {}.", wrong.join("; ")));
    }
    if let Some(f) = r.widths.iter().find(|w| w.name.starts_with("family")) {
        match f.as_is.width {
            Some(w) if w > f.expected && has_2027 => out.push(format!(
                "ZWJ sequences drawn {w} wide: grapheme clustering is off; enable mode 2027 (CSI ? 2027 h) to get {}.",
                f.expected
            )),
            Some(w) if w > f.expected => out.push(format!(
                "ZWJ sequences drawn {w} wide and the terminal has no mode 2027: it draws each part of an emoji sequence \
                 separately, so programs that count by grapheme will drift from what is on screen."
            )),
            _ => {}
        }
    }
    if has_2027 && let (Some((a, n)), Some((b, _))) = (r.score(false), r.score(true)) {
        if b > a {
            out.push(format!(
                "Mode 2027 is available and fixes {} of {n} cases ({a} match as-is, {b} with it set): programs have to turn it on.",
                b - a
            ));
        } else if b < a {
            out.push(format!("Setting mode 2027 made widths worse ({a} match as-is, {b} with it set)."));
        } else if b < n {
            out.push(format!("Setting mode 2027 changes nothing: {b} of {n} match either way."));
        }
    }
    if !has_2027 && mode.is_some_and(|m| m.outcome == Outcome::No) && wrong.is_empty() {
        out.push("The terminal has no mode 2027 but every width already matches.".to_string());
    }
    for w in r.widths.iter().filter(|w| w.disputed) {
        if let Some(n) = w.as_is.width {
            out.push(format!("{} is disputed and not scored: drawn {n} wide.", w.name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::{Row, Trace, sample};
    use super::*;

    fn set_decided(r: &mut ProbeResults, name: &str, d: Decision) {
        if let Some(f) = r.features.iter_mut().find(|f| f.name == name) {
            f.trace.decided = d;
            f.trace.notes.clear();
        }
    }

    #[test]
    fn a_silent_terminal_gets_one_finding() {
        let mut r = sample("a", &[(1, None)]);
        r.features.push(Row {
            section: Section::Identity,
            name: "device attributes (DA1)".to_string(),
            outcome: Outcome::Unknown,
            note: "no reply".to_string(),
            how: String::new(),
            trace: Trace::without_query(Decision::Timeout),
        });
        let f = findings(&r);
        assert_eq!(f.len(), 1);
        assert!(f[0].contains("No reply at all within 1 s") && f[0].contains("outside tmux"), "{f:?}");
    }

    #[test]
    fn unreported_modes_are_not_called_absent() {
        let mut r = sample("a", &[]);
        r.features[1].outcome = Outcome::No;
        set_decided(&mut r, "synchronized output (2026)", Decision::Sentinel);
        let f = findings(&r).join("\n");
        assert!(f.contains("synchronized output (2026) not reported"), "{f}");
        assert!(f.contains("may still support it without answering the query"), "{f}");
    }

    #[test]
    fn zwj_drawn_wide_points_at_mode_2027() {
        let mut r = sample("a", &[(2, Some(6))]);
        r.widths[0].name = "family (ZWJ sequence)".to_string();
        let f = findings(&r).join("\n");
        assert!(
            f.contains("ZWJ sequences drawn 6 wide: grapheme clustering is off; enable mode 2027"),
            "{f}"
        );
        assert!(f.contains("Mode 2027 is available and fixes 1 of 1"), "{f}");
        r.widths[0].with_2027 = None;
        let f = findings(&r).join("\n");
        assert!(f.contains("no mode 2027"), "{f}");
    }

    #[test]
    fn unmeasured_widths_say_why() {
        let r = sample("a", &[(1, None), (2, None)]);
        let f = findings(&r).join("\n");
        assert!(f.contains("DSR 6) did not come back"), "{f}");
    }

    #[test]
    fn a_quiet_run_says_so() {
        let mut r = sample("a", &[(1, Some(1))]);
        for f in &mut r.features {
            f.trace.notes.clear();
        }
        for w in &mut r.widths {
            w.as_is.trace.notes.clear();
            if let Some(p) = &mut w.with_2027 {
                p.trace.notes.clear();
            }
        }
        r.features.retain(|f| f.name != "grapheme clustering (2027)");
        r.widths[0].with_2027 = None;
        let f = findings(&r);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].starts_with("Nothing unexpected"));
    }
}
