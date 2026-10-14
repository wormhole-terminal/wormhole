//! The result screen printed after a run: a header with the terminal and machine, then one bar per
//! test. It is built to be screenshotted, so it paints its own dark panel and looks the
//! same in any theme.

use super::results::{Results, TestResult, Unit};
use super::stats;
use super::ui::{BAD, DIM, FAINT, Line, Panel, TEXT, fit};

/// The width the panel aims for: wide enough for the bars to breathe, narrow enough to screenshot.
pub fn panel_width(cols: u16) -> usize {
    (cols as usize).clamp(56, 100)
}

/// A rate that fills a bar for each test: roughly what a fast, GPU-drawn terminal manages. Bars are
/// linear and stop at full, so a faster terminal shows the same full bar; the number says the rest.
fn reference(name: &str, unit: Unit) -> f64 {
    match (unit, name) {
        (Unit::Fps, _) => 600.0,
        (_, "logflood") => 400.0,
        (_, "glyphstorm") => 300.0,
        (_, "colorflood") => 300.0,
        (_, "scrollstorm") => 400.0,
        (_, "cursorstorm") => 300.0,
        (_, "longlines") => 600.0,
        // Mostly text with a few colors: close to plain log lines.
        (_, "buildlog") => 300.0,
        // Tiny writes: the cost is per write, so even a fast terminal takes in only a few MB/s.
        (_, "agentstream") => 8.0,
        (_, "diffscroll") => 250.0,
        (_, "editstorm") => 150.0,
        (_, "stylestorm") => 200.0,
        (_, "linkstorm") => 250.0,
        // A full-screen paint per swap, about 20 KB at 120×40.
        (Unit::PerSec, _) => 1000.0,
        _ => 1000.0,
    }
}

/// How full a test's bar is, 0..=1. Latency is logarithmic: 50 µs fills it, 10 ms leaves it empty.
pub fn bar_fraction(t: &TestResult) -> f64 {
    if t.timeout || t.median <= 0.0 {
        return 0.0;
    }
    match t.unit {
        Unit::Micros => 1.0 - ((t.median / 50.0).max(1.0).ln() / (10_000.0f64 / 50.0).ln()).clamp(0.0, 1.0),
        unit => (t.median / reference(&t.name, unit)).clamp(0.0, 1.0),
    }
}

/// A number and its unit, split so the number can be bold.
pub fn value_parts(t: &TestResult) -> (String, &'static str) {
    let number = match t.unit {
        Unit::Micros => format!("{:.0}", t.median),
        _ if t.median >= 100.0 => format!("{:.0}", t.median),
        _ => format!("{:.1}", t.median),
    };
    (number, t.unit.label())
}

/// Appends a right-aligned number and unit that end at the current width plus `width`.
pub fn push_value(l: &mut Line, t: &TestResult, width: usize) {
    let (num, unit) = value_parts(t);
    let used = num.chars().count() + 1 + unit.chars().count();
    l.pad(width.saturating_sub(used)).bold(&num, TEXT).text(&format!(" {unit}"), DIM);
}

/// The section a result sits under: real-world output apart from the stress tests that share its unit
/// (the agent redraws are measured in frames per second but are what a program prints all the same).
fn section(t: &TestResult) -> &'static str {
    match t.unit {
        Unit::MBps | Unit::Fps if super::workloads::REALISTIC.contains(&t.name.as_str()) => {
            "REAL-WORLD OUTPUT"
        }
        Unit::MBps => "THROUGHPUT",
        Unit::Fps => "FULL-SCREEN REDRAWS",
        Unit::PerSec => "SCREEN SWAPS",
        Unit::Micros => "ROUND-TRIP LATENCY",
    }
}

struct Layout {
    bar: usize,
    spread: bool,
}

const NAME_W: usize = 13;
const VALUE_W: usize = 12;
const SPREAD_W: usize = 10;

fn layout(width: usize) -> Layout {
    // margin, name, gap | bar | gap, value, gap, spread, margin
    let with_spread = 2 + NAME_W + 1 + 2 + VALUE_W + 2 + SPREAD_W + 2;
    let without = 2 + NAME_W + 1 + 2 + VALUE_W;
    if width >= with_spread + 12 {
        Layout { bar: width - with_spread, spread: true }
    } else if width >= without + 6 {
        Layout { bar: width - without, spread: false }
    } else {
        Layout { bar: 0, spread: false }
    }
}

fn spread_text(t: &TestResult) -> String {
    match t.unit {
        Unit::Micros => format!("p99 {:.0}", t.extra("p99").unwrap_or(t.max)),
        _ => format!("±{:.0}%", stats::spread_pct(t.median, t.min, t.max)),
    }
}

fn row(t: &TestResult, lay: &Layout) -> Line {
    let mut l = Line::new();
    l.pad(2).text(&format!("{:<NAME_W$}", fit(&t.name, NAME_W)), TEXT).pad(1);
    if t.timeout {
        l.text("timed out", BAD);
        return l;
    }
    if lay.bar > 0 {
        l.bar(bar_fraction(t), lay.bar, 1.0).pad(2);
    } else {
        l.pad(1);
    }
    push_value(&mut l, t, VALUE_W);
    if lay.spread {
        l.pad(2);
        let s = spread_text(t);
        l.pad(SPREAD_W.saturating_sub(s.chars().count())).text(&s, DIM);
    }
    l
}

/// The machine line, dropping the least useful parts from the end until it fits.
fn machine_line(r: &Results, max: usize) -> String {
    let m = &r.machine;
    let mut parts = vec![format!("{}×{} cells", r.cols, r.rows), m.cpu.clone()];
    if !m.memory.is_empty() {
        parts.push(m.memory.clone());
    }
    parts.push(m.os.clone());
    if !m.model.is_empty() {
        parts.push(m.model.clone());
    }
    parts.retain(|p| !p.is_empty());
    while parts.len() > 1 && parts.join("  ·  ").chars().count() > max {
        parts.pop();
    }
    fit(&parts.join("  ·  "), max)
}

pub fn results_panel(r: &Results, width: usize, saved: Option<&str>) -> Panel {
    let mut p = Panel::new(width);
    let inner = width.saturating_sub(4);
    p.blank();
    let mut l = Line::new();
    l.pad(2).title("ttystorm").text("  terminal throughput & latency", DIM);
    if width >= 70 {
        l.right(&r.date, DIM, width - 2);
    }
    p.push(l);
    let mut l = Line::new();
    l.pad(2).bold(&fit(&r.label, inner), TEXT);
    if !r.terminal.is_empty() && !r.label.contains(&r.terminal) {
        let room = inner.saturating_sub(l.width() - 2 + 2);
        l.text(&format!("  {}", fit(&r.terminal, room)), DIM);
    }
    p.push(l);
    let mut l = Line::new();
    l.pad(2).text(&machine_line(r, inner), DIM);
    p.push(l);
    let mut l = Line::new();
    let runs = format!("{} MB per test  ·  median of {} runs  ·  v{}", r.size_mb, r.runs, r.version);
    l.pad(2).text(&fit(&runs, inner), DIM);
    p.push(l);
    p.blank();
    p.rule(2);

    let lay = layout(width);
    let mut last: Option<&str> = None;
    for t in &r.tests {
        if last != Some(section(t)) {
            p.blank();
            let mut l = Line::new();
            l.pad(2).bold(section(t), FAINT);
            if lay.spread && last.is_none() {
                // Column titles above the first section only.
                let end = 2 + NAME_W + 1 + lay.bar + 2 + VALUE_W;
                l.right("median", DIM, end).right("spread", DIM, end + 2 + SPREAD_W);
            }
            p.push(l);
            last = Some(section(t));
        }
        p.push(row(t, &lay));
    }
    p.blank();
    p.rule(2);
    let mut l = Line::new();
    l.pad(2).text(&fit("Full bar = a fast terminal's rate; latency bars are logarithmic.", inner), FAINT);
    p.push(l);
    if let Some(path) = saved {
        let mut l = Line::new();
        let name = std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path);
        l.pad(2).text(&fit(&format!("Saved to {name}  ·  ttystorm compare a.json b.json"), inner), FAINT);
        p.push(l);
    }
    p.blank();
    p
}

/// The results as plain text, without colors or escape sequences: `--plain`, for logs and scripts.
pub fn results_plain(r: &Results, saved: Option<&str>) -> String {
    let mut s = format!("{}  ·  {}\n", r.label, machine_line(r, usize::MAX));
    s.push_str(&format!(
        "{} MB per test  ·  median of {} runs  ·  ttystorm {}\n\n",
        r.size_mb, r.runs, r.version
    ));
    for t in &r.tests {
        let (num, unit) = value_parts(t);
        let detail = match t.unit {
            _ if t.timeout => String::new(),
            Unit::Micros => format!(
                "p90 {:.0}  p99 {:.0}",
                t.extra("p90").unwrap_or(t.max),
                t.extra("p99").unwrap_or(t.max)
            ),
            _ => format!("min {:.1}  max {:.1}", t.min, t.max),
        };
        let value = if t.timeout { "timed out".to_string() } else { format!("{num:>8} {unit:<5}") };
        s.push_str(format!("{:<NAME_W$} {value}  {detail}", t.name).trim_end());
        s.push('\n');
    }
    if let Some(path) = saved {
        s.push_str(&format!("\nsaved to {path}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::super::results::sample;
    use super::*;

    #[test]
    fn plain_results_name_each_test_without_escapes() {
        let mut r = sample("Wormhole 0.3.0", 1.0);
        r.tests[1].timeout = true;
        let text = results_plain(&r, Some("w.json"));
        assert!(!text.contains('\x1b'), "{text}");
        for t in &r.tests {
            assert!(text.lines().any(|l| l.starts_with(t.name.as_str())), "{}: {text}", t.name);
        }
        assert!(text.contains("timed out") && text.contains("p99") && text.contains("w.json"), "{text}");
    }

    #[test]
    fn every_line_fits_the_panel() {
        for width in [56, 60, 80, 100] {
            let r = sample("Wormhole 0.1.3", 1.0);
            let p = results_panel(&r, width, Some("/tmp/a.json"));
            for (i, line) in p.lines().iter().enumerate() {
                assert!(line.width() <= width, "width {width}, line {i}: {}", line.width());
            }
        }
    }

    #[test]
    fn the_screen_names_each_test() {
        let r = sample("Wormhole", 1.0);
        let mut out = Vec::new();
        results_panel(&r, 100, None).write(&mut out);
        let text = String::from_utf8_lossy(&out);
        for t in &r.tests {
            assert!(text.contains(&t.name), "{}", t.name);
        }
        assert!(text.contains("2026-10-10") && text.contains("Apple M3 Pro") && text.contains("120×40"));
    }

    #[test]
    fn bars_follow_the_numbers() {
        let r = sample("x", 1.0);
        let ascii = &r.tests[0];
        assert!((bar_fraction(ascii) - 1.0).abs() < 1e-9);
        let mut mid = ascii.clone();
        mid.median = 100.0;
        assert!((bar_fraction(&mid) - 0.25).abs() < 1e-9);
        let mut fast = ascii.clone();
        fast.median = 5000.0;
        assert_eq!(bar_fraction(&fast), 1.0);
        let mut quick = r.tests[3].clone();
        quick.median = 40.0;
        assert_eq!(bar_fraction(&quick), 1.0);
        quick.median = 20_000.0;
        assert_eq!(bar_fraction(&quick), 0.0);
        let mut dead = ascii.clone();
        dead.timeout = true;
        assert_eq!(bar_fraction(&dead), 0.0);
    }

    #[test]
    fn numbers_keep_their_precision_sensibly() {
        let r = sample("x", 1.0);
        assert_eq!(value_parts(&r.tests[0]), ("800".to_string(), "MB/s"));
        let mut slow = r.tests[1].clone();
        slow.median = 12.34;
        assert_eq!(value_parts(&slow), ("12.3".to_string(), "MB/s"));
    }
}
