//! `ttystorm compare`: the saved results of several terminals side by side. Per test one bar for
//! each terminal on a shared scale (the best fills it), the best marked, and how many times better
//! than the first file each one is. `--markdown` prints the same as a table to paste into an issue
//! or a README instead.

use super::report::{push_value, value_parts};
use super::results::{Results, TestResult};
use super::ui::{BAD, DIM, FAINT, GOOD, Line, Panel, TEXT, fit};
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

/// The index of the best result among `all` for `name`; the first wins a tie.
fn best_of(all: &[Results], name: &str) -> Option<usize> {
    let mut best: Option<(usize, &TestResult)> = None;
    for (i, r) in all.iter().enumerate() {
        let Some(t) = find(r, name).filter(|t| !t.timeout && t.median > 0.0) else {
            continue;
        };
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

/// The geometric mean of how many times better `r` is than `base` over the tests both completed.
fn overall(r: &Results, base: &Results) -> Option<f64> {
    let ratios: Vec<f64> =
        r.tests.iter().filter_map(|t| find(base, &t.name).and_then(|b| t.better_than(b))).collect();
    if ratios.is_empty() {
        return None;
    }
    Some((ratios.iter().map(|x| x.ln()).sum::<f64>() / ratios.len() as f64).exp())
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
            if best {
                push_value(&mut l, t, VALUE_W);
            } else {
                let (num, unit) = value_parts(t);
                let used = num.chars().count() + 1 + unit.chars().count();
                l.pad(VALUE_W.saturating_sub(used)).text(&format!("{num} {unit}"), DIM);
            }
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
fn warnings(all: &[Results]) -> Vec<&'static str> {
    let base = &all[0];
    let same = |f: fn(&Results) -> String| all.iter().all(|r| f(r) == f(base));
    let mut warnings = Vec::new();
    if !same(|r| r.machine.cpu.clone()) {
        warnings.push("These results come from different machines.");
    }
    if !same(|r| format!("{}×{}", r.cols, r.rows)) {
        warnings.push("Terminal sizes differ, which changes the scrolling, cursor and frame tests.");
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
        Some(t) if t.timeout => "timed out".to_string(),
        Some(t) => {
            let (num, unit) = value_parts(t);
            if best { format!("**{num} {unit}**") } else { format!("{num} {unit}") }
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
    let scores: Vec<Option<f64>> = all.iter().map(|r| overall(r, &all[0])).collect();
    let top = scores.iter().flatten().cloned().fold(0.0f64, f64::max);
    let cells: Vec<String> = scores
        .iter()
        .map(|sc| match sc {
            Some(x) if *x >= top && top > 0.0 => format!("**{}**", ratio_text(*x)),
            Some(x) => ratio_text(*x),
            None => "—".to_string(),
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
        let mut l = Line::new();
        l.pad(2).text(&fit(w, inner), BAD);
        p.push(l);
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
                (Some(t), Some(b)) if !t.timeout && t.median > 0.0 => {
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
                (Some(t), _) if t.timeout => terminal_row(&c, &r.label, false, None, None, None, "timed out"),
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
    let scores: Vec<Option<f64>> = all.iter().map(|r| overall(r, base)).collect();
    let top = scores.iter().flatten().cloned().fold(0.0f64, f64::max);
    let wins: Vec<usize> = (0..all.len())
        .map(|i| test_names(all).iter().filter(|n| best_of(all, n) == Some(i)).count())
        .collect();
    for (i, r) in all.iter().enumerate() {
        let mut l = Line::new();
        l.pad(2);
        let leader = scores[i].is_some_and(|s| s >= top && top > 0.0);
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
        let frac = scores[i].map(|s| s / top.max(1e-9)).unwrap_or(0.0);
        l.bar(frac, c.bar, if leader { 1.0 } else { 0.55 }).pad(2);
        let w = format!("{} {}", wins[i], if wins[i] == 1 { "win" } else { "wins" });
        l.right(&w, if leader { TEXT } else { DIM }, 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W);
        l.pad(2);
        let end = 2 + 2 + c.label + 1 + c.bar + 2 + VALUE_W + 2 + RATIO_W;
        match scores[i] {
            Some(s) => l.right(&ratio_text(s), ratio_color(s), end),
            None => l.right("—", FAINT, end),
        };
        p.push(l);
    }
    p.blank();
    let mut l = Line::new();
    l.pad(2).text(&fit("× = times better than the first file (above 1 is better)", inner), FAINT);
    p.push(l);
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
        let s = overall(&all[1], &all[0]).unwrap_or(0.0);
        // Three throughput/fps tests at x2 and a latency test at x0.5: (2*2*2*0.5)^(1/4).
        assert!((s - 2f64.sqrt()).abs() < 1e-9, "{s}");
        assert!((overall(&all[0], &all[0]).unwrap_or(0.0) - 1.0).abs() < 1e-9);
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
}
