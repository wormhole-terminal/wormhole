//! `ttystorm compare`: the saved results of several terminals side by side. Per test one bar for
//! each terminal on a shared scale (the best fills it), the best marked, and how many times better
//! than the first file each one is. `--markdown` prints the same as a table to paste into an issue
//! or a README instead.

use super::report::value_parts;
use super::results::{Results, TestResult};
use super::ui::{ACCENT, BAD, DIM, FAINT, GOOD, Line, Panel, TEXT, fit, wrap};
use super::workloads;

const VALUE_W: usize = 12;
const RATIO_W: usize = 7;

/// The tests present in any file: the known ones in running order, then any others.
fn test_names(all: &[Results]) -> Vec<String> {
    let mut names: Vec<String> = workloads::NAMES
        .iter()
        .filter(|n| all.iter().any(|r| r.tests.iter().any(|t| t.name == **n)))
        .map(|n| n.to_string())
        .collect();
    for t in all.iter().flat_map(|r| &r.tests) {
        if !names.contains(&t.name) {
            names.push(t.name.clone());
        }
    }
    names
}

fn find<'a>(r: &'a Results, name: &str) -> Option<&'a TestResult> {
    r.tests.iter().find(|t| t.name == name)
}

/// A result with a number to put in a row.
fn usable(t: &TestResult) -> bool {
    t.unusable().is_none() && t.median > 0.0
}

/// The index of the best result among `all` for `name`; the first wins a tie. Verified numbers compete
/// with each other; an unverified one is never better than a verified one. Only in a row without a
/// verified number do the unverified ones compete, among themselves.
fn best_of(all: &[Results], name: &str) -> Option<usize> {
    let found: Vec<(usize, &TestResult)> = all
        .iter()
        .enumerate()
        .filter_map(|(i, r)| find(r, name).filter(|t| usable(t)).map(|t| (i, t)))
        .collect();
    let kind = found.iter().any(|(_, t)| t.verified());
    let mut best: Option<(usize, &TestResult)> = None;
    for (i, t) in found.into_iter().filter(|(_, t)| t.verified() == kind) {
        let better = match best {
            None => true,
            Some((_, b)) => t.better_than(b).is_some_and(|x| x > 1.0),
        };
        if better {
            best = Some((i, t));
        }
    }
    best.map(|(i, _)| i)
}

fn ratio_text(x: f64) -> String {
    if x >= 100.0 { format!("×{x:.0}") } else { format!("×{x:.2}") }
}

/// An overall ratio, and what it was worked out from.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Score {
    value: f64,
    /// Worked out from unverified numbers.
    unverified: bool,
    /// The number of tests in the mean.
    tests: usize,
}

impl Score {
    fn text(self) -> String {
        format!("{}{}", ratio_text(self.value), if self.unverified { " †" } else { "" })
    }

    /// "3 of 4 tests" when the mean leaves out tests of the table, `None` when it covers them all. A
    /// terminal that timed out on the hard tests would otherwise lead on the easy ones without a word.
    fn coverage(self, of: usize) -> Option<String> {
        (self.tests < of).then(|| format!("{} of {of} tests", self.tests))
    }
}

/// The geometric mean of how many times better `r` is than `base` over the tests both completed, among
/// the verified numbers; when there are none, among the unverified ones, marked as such. The two kinds
/// are never mixed in one mean. `None` when no test has a number of the same kind in both files: latency
/// sends no payload and has no kind, but it alone is no ground for an overall ratio.
fn overall(r: &Results, base: &Results) -> Option<Score> {
    let (mut verified, mut unverified, mut neutral) = (Vec::new(), Vec::new(), Vec::new());
    for t in &r.tests {
        if let Some(x) = find(base, &t.name).and_then(|b| t.better_than(b)) {
            if !t.has_payload() {
                neutral.push(x)
            } else if t.verified() {
                verified.push(x)
            } else {
                unverified.push(x)
            }
        }
    }
    let (mut ratios, unverified) = match (verified.is_empty(), unverified.is_empty()) {
        (false, _) => (verified, false),
        (true, false) => (unverified, true),
        (true, true) => return None,
    };
    ratios.extend(neutral);
    let value = (ratios.iter().map(|x| x.ln()).sum::<f64>() / ratios.len() as f64).exp();
    Some(Score { value, unverified, tests: ratios.len() })
}

/// Which scores compete: the verified ones, or when there are none, the unverified ones.
fn leading_kind_unverified(scores: &[Option<Score>]) -> bool {
    !scores.iter().flatten().any(|s| !s.unverified)
}

/// The best score of the competing kind, `0.0` when there is no race: with a single score (the first
/// file's own, always 1) nobody leads.
fn top_score(scores: &[Option<Score>], unverified: bool) -> f64 {
    let racing: Vec<f64> =
        scores.iter().flatten().filter(|s| s.unverified == unverified).map(|s| s.value).collect();
    if racing.len() < 2 { 0.0 } else { racing.into_iter().fold(0.0f64, f64::max) }
}

fn ratio_color(x: f64) -> super::super::canvas::Rgb {
    if x >= 1.05 {
        GOOD
    } else if x <= 0.95 {
        BAD
    } else {
        DIM
    }
}

struct Cols {
    label: usize,
    bar: usize,
}

fn cols(all: &[Results], width: usize) -> Cols {
    let label = all.iter().map(|r| r.label.chars().count()).max().unwrap_or(8).clamp(8, 16);
    // margin, marker, label, gap | bar | gap, value, gap, ratio, margin
    let fixed = 2 + 2 + label + 1 + 2 + VALUE_W + 2 + RATIO_W + 2;
    Cols { label, bar: width.saturating_sub(fixed).max(6) }
}

fn terminal_row(
    c: &Cols,
    label: &str,
    best: bool,
    bar: Option<(f64, f64)>,
    t: Option<&TestResult>,
    ratio: Option<f64>,
    note: &str,
) -> Line {
    let mut l = Line::new();
    l.pad(2);
    if best {
        l.bold("▸ ", GOOD)
    } else {
        l.pad(2)
    };
    let name = format!("{:<w$}", fit(label, c.label), w = c.label);
    if best {
        l.bold(&name, TEXT)
    } else {
        l.text(&name, DIM)
    };
    l.pad(1);
    match (bar, t) {
        (Some((frac, dim)), Some(t)) => {
            l.bar(frac, c.bar, dim).pad(2);
            let (num, unit) = value_parts(t);
            let dagger = if t.unverified() { " †" } else { "" };
            let used = num.chars().count() + 1 + unit.chars().count() + dagger.chars().count();
            l.pad(VALUE_W.saturating_sub(used));
            if best {
                l.bold(&num, TEXT).text(&format!(" {unit}{dagger}"), DIM);
            } else {
                l.text(&format!("{num} {unit}{dagger}"), DIM);
            }
        }
        (None, Some(t)) => {
            l.text(&fit(note, c.bar), ACCENT).pad_to(2 + 2 + c.label + 1 + c.bar).pad(2);
            let (num, unit) = value_parts(t);
            let shown = format!("{num} {unit} †");
            l.pad(VALUE_W.saturating_sub(shown.chars().count())).text(&shown, DIM);
        }
        _ => {
            l.text(&fit(note, c.bar + 2 + VALUE_W), BAD);
            l.pad_to(2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W);
        }
    }
    l.pad(2);
    match ratio {
        Some(x) => {
            l.right(&ratio_text(x), ratio_color(x), 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W + 2 + RATIO_W)
        }
        None => l.right("—", FAINT, 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W + 2 + RATIO_W),
    };
    l
}

/// What makes the files hard to compare: different machines or window sizes.
fn warnings(all: &[Results]) -> Vec<String> {
    let base = &all[0];
    let same = |f: fn(&Results) -> String| all.iter().all(|r| f(r) == f(base));
    let mut warnings = Vec::new();
    if !same(|r| r.machine.cpu.clone()) {
        warnings.push("These results come from different machines.".to_string());
    }
    if !same(|r| format!("{}×{}", r.cols, r.rows)) {
        warnings
            .push("Terminal sizes differ, which changes the scrolling, cursor and frame tests.".to_string());
    }
    if all.iter().skip(1).any(|r| overall(r, base).is_none()) {
        warnings.push(
            "No overall ratio (—) for a file with no test whose number is of the same kind as the first file's: a verified number is never set against an unverified one."
                .to_string(),
        );
    }
    if all.iter().flat_map(|r| &r.tests).any(TestResult::unverified) {
        warnings.push(
            "Numbers marked † are not verified (the input check was off, or could not decide, or the file was saved before the check existed). They rank only among themselves: never above a verified number, with no ratio against one, and the overall ratio is worked out within each kind."
                .to_string(),
        );
    }
    warnings
}

/// A cell's text in a markdown table: the unit with the number, `|` kept from ending the cell.
fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

fn markdown_cell(t: Option<&TestResult>, best: bool) -> String {
    match t {
        None => "—".to_string(),
        Some(t) if t.unusable().is_some() => t.unusable().unwrap_or_default().to_string(),
        Some(t) => {
            let (num, unit) = value_parts(t);
            // A number the input check could not vouch for carries a dagger; the note says what it means.
            let mark = if t.unverified() { " †" } else { "" };
            if best { format!("**{num} {unit}**{mark}") } else { format!("{num} {unit}{mark}") }
        }
    }
}

/// The comparison as a markdown table: a row per test, a column per terminal, the best of each row
/// in bold and the overall ratio to the first file last. A note above says when the files differ in
/// machine or window size.
pub fn compare_markdown(all: &[Results]) -> String {
    let mut s = String::new();
    let warnings = warnings(all);
    if !warnings.is_empty() {
        s.push_str(&format!("> Note: {}\n\n", warnings.join(" ")));
    }
    let labels: Vec<String> = all.iter().map(|r| escape_cell(&r.label)).collect();
    s.push_str(&format!("| Test | {} |\n", labels.join(" | ")));
    s.push_str(&format!("|---|{}\n", "--:|".repeat(all.len())));
    for name in test_names(all) {
        let best = best_of(all, &name);
        let cells: Vec<String> =
            all.iter().enumerate().map(|(i, r)| markdown_cell(find(r, &name), best == Some(i))).collect();
        s.push_str(&format!("| {} | {} |\n", escape_cell(&name), cells.join(" | ")));
    }
    let scores: Vec<Option<Score>> = all.iter().map(|r| overall(r, &all[0])).collect();
    let of = test_names(all).len();
    let kind = leading_kind_unverified(&scores);
    let top = top_score(&scores, kind);
    let cells: Vec<String> = scores
        .iter()
        .map(|sc| {
            let Some(s) = sc else { return "—".to_string() };
            let mut cell = if s.unverified == kind && s.value >= top && top > 0.0 {
                format!("**{}**", ratio_text(s.value)) + if s.unverified { " †" } else { "" }
            } else {
                s.text()
            };
            if let Some(cover) = s.coverage(of) {
                cell.push_str(&format!(" ({cover})"));
            }
            cell
        })
        .collect();
    s.push_str(&format!("| Overall vs {} | {} |\n", escape_cell(&all[0].label), cells.join(" | ")));
    s
}

pub fn compare_panel(all: &[Results], width: usize) -> Panel {
    let mut p = Panel::new(width);
    let inner = width.saturating_sub(4);
    let c = cols(all, width);
    let base = &all[0];
    p.blank();
    let mut l = Line::new();
    l.pad(2).title("ttystorm compare");
    l.text(&format!("  {} terminals", all.len()), DIM);
    if width >= 70 {
        l.right(&base.date, DIM, width - 2);
    }
    p.push(l);
    p.blank();
    for (i, r) in all.iter().enumerate() {
        let mut l = Line::new();
        l.pad(2)
            .text(&format!("{} ", i + 1), FAINT)
            .bold(&format!("{:<w$}", fit(&r.label, c.label), w = c.label), TEXT);
        let m = &r.machine;
        let detail =
            [m.cpu.clone(), format!("{}×{}", r.cols, r.rows), format!("{} MB × {}", r.size_mb, r.runs)];
        let detail = detail.iter().filter(|d| !d.is_empty()).cloned().collect::<Vec<_>>().join("  ·  ");
        let room = inner.saturating_sub(l.width() - 2 + 2);
        l.pad(2).text(&fit(&detail, room), DIM);
        p.push(l);
    }
    let warnings = warnings(all);
    for w in warnings {
        for text in wrap(&w, inner) {
            let mut l = Line::new();
            l.pad(2).text(&text, BAD);
            p.push(l);
        }
    }
    p.blank();
    p.rule(2);

    for name in test_names(all) {
        p.blank();
        let mut l = Line::new();
        l.pad(2).bold(&name, TEXT);
        // The note on the right takes 16 cells; the description gets what is left.
        let room = width.saturating_sub(2 + name.chars().count() + 2 + 16 + 4);
        if room >= 10 {
            l.text(&format!("  {}", fit(workloads::about(&name), room)), FAINT);
        }
        let unit = all.iter().find_map(|r| find(r, &name)).map(|t| t.unit);
        if let Some(u) = unit {
            let note = if u.higher_is_better() { "higher is better" } else { "lower is better" };
            l.right(note, FAINT, width - 2);
        }
        p.push(l);
        let best = best_of(all, &name);
        let best_t = best.and_then(|i| find(&all[i], &name));
        let base_t = find(base, &name);
        for (i, r) in all.iter().enumerate() {
            let t = find(r, &name);
            let ratio = t.zip(base_t).and_then(|(t, b)| t.better_than(b));
            let row = match (t, best_t) {
                (Some(t), Some(b)) if usable(t) && t.verified() == b.verified() => {
                    // The best fills the bar; the others are as long as they are good compared with it.
                    let frac = b.better_than(t).map(|x| 1.0 / x).unwrap_or(0.0).clamp(0.0, 1.0);
                    let is_best = best == Some(i);
                    terminal_row(
                        &c,
                        &r.label,
                        is_best,
                        Some((if is_best { 1.0 } else { frac }, if is_best { 1.0 } else { 0.55 })),
                        Some(t),
                        ratio,
                        "",
                    )
                }
                (Some(t), _) if t.unusable().is_some() => {
                    terminal_row(&c, &r.label, false, None, None, None, t.unusable().unwrap_or_default())
                }
                // A number without the input check's word: shown, not ranked, no bar.
                (Some(t), _) if usable(t) => {
                    terminal_row(&c, &r.label, false, None, Some(t), None, "unverified, not ranked")
                }
                _ => terminal_row(&c, &r.label, false, None, None, None, "not measured"),
            };
            p.push(row);
        }
    }

    p.blank();
    p.rule(2);
    p.blank();
    let mut l = Line::new();
    l.pad(2).bold("OVERALL", FAINT);
    l.text(
        &fit(&format!("  geometric mean over the tests, vs {}", base.label), inner.saturating_sub(9)),
        FAINT,
    );
    p.push(l);
    let scores: Vec<Option<Score>> = all.iter().map(|r| overall(r, base)).collect();
    let of = test_names(all).len();
    // Only scores of the leading kind compete; a score of the other kind is shown, with its dagger.
    let kind = leading_kind_unverified(&scores);
    let counts = |s: &Score| s.unverified == kind;
    let top = top_score(&scores, kind);
    let wins: Vec<usize> = (0..all.len())
        .map(|i| test_names(all).iter().filter(|n| best_of(all, n) == Some(i)).count())
        .collect();
    for (i, r) in all.iter().enumerate() {
        let mut l = Line::new();
        l.pad(2);
        let leader = scores[i].is_some_and(|s| counts(&s) && s.value >= top && top > 0.0);
        if leader {
            l.bold("▸ ", GOOD)
        } else {
            l.pad(2)
        };
        let name = format!("{:<w$}", fit(&r.label, c.label), w = c.label);
        if leader {
            l.bold(&name, TEXT)
        } else {
            l.text(&name, DIM)
        };
        l.pad(1);
        let frac = scores[i].filter(counts).map(|s| (s.value / top.max(1e-9)).min(1.0)).unwrap_or(0.0);
        let frac = if top > 0.0 { frac } else { 0.0 };
        l.bar(frac, c.bar, if leader { 1.0 } else { 0.55 }).pad(2);
        let w = format!("{} {}", wins[i], if wins[i] == 1 { "win" } else { "wins" });
        l.right(&w, if leader { TEXT } else { DIM }, 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W);
        l.pad(2);
        let end = 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W + 2 + RATIO_W;
        match scores[i] {
            Some(s) => l.right(&s.text(), ratio_color(s.value), end),
            None => l.right("—", FAINT, end),
        };
        p.push(l);
    }
    p.blank();
    let mut l = Line::new();
    l.pad(2).text(&fit("× = times better than the first file (above 1 is better)", inner), FAINT);
    p.push(l);
    // A mean over fewer tests than the table has is said, with the files it applies to.
    let partial: Vec<String> = all
        .iter()
        .zip(&scores)
        .filter_map(|(r, s)| s.and_then(|s| s.coverage(of)).map(|c| format!("{}: {c}", r.label)))
        .collect();
    if !partial.is_empty() {
        for text in wrap(&format!("Overall over fewer tests than listed ({}).", partial.join("; ")), inner) {
            let mut l = Line::new();
            l.pad(2).text(&text, FAINT);
            p.push(l);
        }
    }
    p.blank();
    p
}

#[cfg(test)]
mod tests {
    use super::super::results::sample;
    use super::*;

    fn three() -> Vec<Results> {
        let mut c = sample("Slow", 0.5);
        c.tests[1].timeout = true;
        c.tests[1].median = 0.0;
        vec![sample("Wormhole", 1.0), sample("Fast one", 2.0), c]
    }

    #[test]
    fn every_line_fits_the_panel() {
        for width in [56, 60, 80, 100] {
            let p = compare_panel(&three(), width);
            for (i, line) in p.lines().iter().enumerate() {
                assert!(line.width() <= width, "width {width}, line {i}: {}", line.width());
            }
        }
    }

    #[test]
    fn the_best_is_found_whichever_way_the_unit_goes() {
        let all = three();
        assert_eq!(best_of(&all, "logflood"), Some(1));
        // Latency: the second file has twice the microseconds, so the third (half) is best.
        assert_eq!(best_of(&all, "roundtrip"), Some(2));
        assert_eq!(best_of(&all, "nope"), None);
        // Timed out results never win.
        assert_eq!(best_of(&all, "colorflood"), Some(1));
    }

    #[test]
    fn overall_is_a_geometric_mean() {
        let all = three();
        let s = overall(&all[1], &all[0]).map_or(0.0, |s| s.value);
        // Three throughput/fps tests at x2 and a latency test at x0.5: (2*2*2*0.5)^(1/4).
        assert!((s - 2f64.sqrt()).abs() < 1e-9, "{s}");
        assert!((overall(&all[0], &all[0]).map_or(0.0, |s| s.value) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_screen_lists_every_terminal_and_flags_differences() {
        let mut all = three();
        all[1].machine.cpu = "Intel".to_string();
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        let text = String::from_utf8_lossy(&out);
        for r in &all {
            assert!(text.contains(&r.label));
        }
        assert!(text.contains("different machines"));
        assert!(text.contains("timed out"));
        assert!(text.contains("OVERALL"));
    }

    #[test]
    fn test_names_keep_the_running_order() {
        let mut a = sample("a", 1.0);
        a.tests.reverse();
        let names = test_names(&[a]);
        assert_eq!(names, vec!["logflood", "colorflood", "framerate", "roundtrip"]);
    }

    #[test]
    fn the_markdown_table_is_a_table_with_the_best_in_bold() {
        let md = compare_markdown(&three());
        let rows: Vec<&str> = md.lines().filter(|l| l.starts_with('|')).collect();
        // header, rule, one row per test, overall
        assert_eq!(rows.len(), 2 + 4 + 1, "{md}");
        let bars = |l: &str| l.matches('|').count();
        assert!(rows.iter().all(|r| bars(r) == bars(rows[0])), "{md}");
        assert_eq!(bars(rows[0]), 5);
        assert!(rows[0].contains("Wormhole") && rows[0].contains("Fast one") && rows[0].contains("Slow"));
        let logflood = rows.iter().find(|r| r.starts_with("| logflood")).copied().unwrap_or_default();
        assert!(logflood.contains("**1600 MB/s**") && logflood.contains("| 800 MB/s |"), "{logflood}");
        // Latency: lower is better, so the third file's 75 µs wins.
        let rt = rows.iter().find(|r| r.starts_with("| roundtrip")).copied().unwrap_or_default();
        assert!(rt.contains("**75 µs**"), "{rt}");
        let colorflood = rows.iter().find(|r| r.starts_with("| colorflood")).copied().unwrap_or_default();
        assert!(colorflood.contains("timed out"), "{colorflood}");
        let overall_row = rows.last().copied().unwrap_or_default();
        assert!(
            overall_row.starts_with("| Overall vs Wormhole | ×1.00 |") && overall_row.contains("**×1.41**"),
            "{overall_row}"
        );
        assert!(!md.contains("> Note"));
    }

    #[test]
    fn the_markdown_notes_differences_in_one_line() {
        let mut all = three();
        all[1].machine.cpu = "Intel".to_string();
        all[2].cols = 80;
        let md = compare_markdown(&all);
        let note: Vec<&str> = md.lines().filter(|l| l.starts_with('>')).collect();
        assert_eq!(note.len(), 1);
        assert!(note[0].contains("different machines") && note[0].contains("sizes differ"));
        let mut odd = three();
        odd[0].label = "a|b".to_string();
        assert!(compare_markdown(&odd).contains("a\\|b"));
    }

    #[test]
    fn dropped_input_never_wins_and_unverified_numbers_carry_a_dagger() {
        use super::super::integrity::{Check, Why};
        let mut all = three();
        // The fast file dropped input: its (absurd) number is gone and the slower ones compare alone.
        all[1].tests[0].check = Some(Check::Dropped { answered: 1, checkpoints: 48 });
        all[1].tests[0].median = 0.0;
        all[0].tests[1].check = Some(Check::Unchecked(Why::Silent));
        assert_eq!(best_of(&all, "logflood"), Some(0));
        let md = compare_markdown(&all);
        let logflood = md.lines().find(|l| l.starts_with("| logflood")).unwrap_or_default();
        assert!(logflood.contains("dropped input") && logflood.contains("**800 MB/s**"), "{logflood}");
        let colorflood = md.lines().find(|l| l.starts_with("| colorflood")).unwrap_or_default();
        assert!(colorflood.contains("†"), "{colorflood}");
        assert!(md.contains("> Note: Numbers marked †"), "{md}");
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        assert!(String::from_utf8_lossy(&out).contains("dropped input"));
    }

    /// Marks every throughput test of a file as unverified (the check was off).
    fn unverify(r: &mut Results) {
        for t in &mut r.tests {
            if t.unit != super::super::results::Unit::Micros {
                t.check = Some(super::super::integrity::Check::Unchecked(super::super::integrity::Why::Off));
            }
        }
    }

    #[test]
    fn a_row_without_a_verified_number_ranks_its_unverified_ones_among_themselves() {
        // Two files run with --no-verify (the README's way to see what the check costs).
        let mut all = vec![sample("a", 1.0), sample("b", 2.0)];
        all.iter_mut().for_each(unverify);
        assert_eq!(best_of(&all, "logflood"), Some(1));
        assert_eq!(all[1].tests[0].better_than(&all[0].tests[0]), Some(2.0));
        let score = overall(&all[1], &all[0]).expect("overall");
        assert!(score.unverified && score.value > 1.0);
        let md = compare_markdown(&all);
        let row = md.lines().find(|l| l.starts_with("| logflood")).unwrap_or_default();
        assert!(row.contains("**1600 MB/s** †") && row.contains("| 800 MB/s † |"), "{row}");
        let overall_row = md.lines().last().unwrap_or_default();
        assert!(overall_row.contains("†"), "{overall_row}");
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        let screen = String::from_utf8_lossy(&out);
        assert!(screen.contains("†") && !screen.contains("unverified, not ranked"), "{screen}");
    }

    #[test]
    fn an_unverified_number_never_wins_a_row_that_has_a_verified_one() {
        // The fast file switched the check off; the slower ones were verified.
        let mut all = three();
        unverify(&mut all[1]);
        assert_eq!(best_of(&all, "logflood"), Some(0));
        assert_eq!(all[1].tests[0].better_than(&all[0].tests[0]), None);
        assert_eq!(all[0].tests[0].better_than(&all[1].tests[0]), None);
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        let screen = String::from_utf8_lossy(&out);
        assert!(screen.contains("unverified, not ranked") && screen.contains(" †"), "{screen}");
        // The dagger is explained, in full, on lines of their own.
        assert!(screen.contains("Numbers marked †") && screen.contains("within each kind."), "{screen}");
        let md = compare_markdown(&all);
        let row = md.lines().find(|l| l.starts_with("| logflood")).unwrap_or_default();
        assert!(row.contains("1600 MB/s †") && !row.contains("**1600"), "{row}");
    }

    #[test]
    fn the_overall_ratio_is_worked_out_within_each_kind() {
        let mut all = three();
        unverify(&mut all[1]);
        // The fast file has no number of the base's kind to set against it: only the latency, which has
        // no kind, is left, and that alone is no ground for an overall ratio.
        assert_eq!(overall(&all[1], &all[0]), None);
        // Half verified, half not: the verified half decides, and the other is not mixed in.
        let mut mixed = [sample("base", 1.0), sample("m", 2.0)];
        mixed[1].tests[0].check =
            Some(super::super::integrity::Check::Unchecked(super::super::integrity::Why::Off));
        mixed[1].tests[0].median = 1.0; // would pull a mixed mean down
        let s = overall(&mixed[1], &mixed[0]).expect("overall");
        assert!(!s.unverified);
        assert_eq!(s.tests, 3);
        let verified_only: f64 = (2.0f64 * 2.0 * 0.5).powf(1.0 / 3.0);
        assert!((s.value - verified_only).abs() < 1e-9, "{}", s.value);
    }

    /// What the reviewer found: with every payload test of a different kind, the latency ratio alone
    /// came out as a verified overall, and a file saved with `--no-verify` could lead.
    #[test]
    fn a_file_with_nothing_of_the_same_kind_has_no_overall_and_the_note_says_so() {
        let mut all = three();
        unverify(&mut all[1]);
        let md = compare_markdown(&all);
        let row = md.lines().last().unwrap_or_default();
        // The file of another kind leads nothing: the base keeps the lead it has among the verified.
        assert!(row.starts_with("| Overall vs Wormhole | **×1.00** | — |"), "{row}");
        assert!(md.contains("No overall ratio (—) for a file with no test"), "{md}");
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        let screen = String::from_utf8_lossy(&out);
        assert!(screen.contains("No overall ratio (—)"), "{screen}");
        // Two files of one kind need no note.
        assert!(!compare_markdown(&three()).contains("No overall ratio"));
    }

    #[test]
    fn the_first_file_alone_with_an_overall_does_not_lead() {
        let mut all = three();
        unverify(&mut all[1]);
        unverify(&mut all[2]);
        let md = compare_markdown(&all);
        let row = md.lines().last().unwrap_or_default();
        assert!(row.contains("| ×1.00 |") && !row.contains("**"), "{row}");
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        let screen = String::from_utf8_lossy(&out);
        let overall_part = screen.split("OVERALL").nth(1).unwrap_or_default();
        assert!(overall_part.contains("Wormhole") && !overall_part.contains('▸'), "{overall_part}");
    }

    #[test]
    fn an_overall_over_fewer_tests_than_the_table_says_so() {
        // `Slow` timed out on one test: its mean covers three of the four.
        let all = three();
        assert_eq!(overall(&all[2], &all[0]).map(|s| s.tests), Some(3));
        let md = compare_markdown(&all);
        let row = md.lines().last().unwrap_or_default();
        assert!(row.contains("(3 of 4 tests)") && row.matches("of 4 tests").count() == 1, "{row}");
        let mut out = Vec::new();
        compare_panel(&all, 100).write(&mut out);
        assert!(String::from_utf8_lossy(&out).contains("Slow: 3 of 4 tests"));
    }

    #[test]
    fn files_from_before_the_check_are_unverified_and_the_note_says_why() {
        let mut all = three();
        for t in &mut all[0].tests {
            t.check = None;
        }
        let md = compare_markdown(&all);
        assert!(md.contains("saved before the check existed"), "{md}");
        let row = md.lines().find(|l| l.starts_with("| logflood")).unwrap_or_default();
        // The old file's number is daggered and the verified ones win the row.
        assert!(row.contains("800 MB/s †") && row.contains("**1600 MB/s**"), "{row}");
        // roundtrip has no payload and no check: never daggered.
        let rt = md.lines().find(|l| l.starts_with("| roundtrip")).unwrap_or_default();
        assert!(!rt.contains('†'), "{rt}");
        assert!(!compare_markdown(&three()).contains("†"));
    }
}
