//! The result screen: a painted panel in the style of the benchmarks' (so a screenshot looks the
//! same in any theme), and the same results as plain text for `--plain`.

use super::model::{Outcome, ProbeResults, Row, Section, WidthPass, WidthResult};
use crate::bench::ui::{BAD, DIM, FAINT, GOOD, Line, Panel, TEXT, fit};
use crate::canvas::Rgb;

/// The width the panel aims for.
pub fn panel_width(cols: u16) -> usize {
    (cols as usize).clamp(64, 100)
}

const NAME_W: usize = 28;
/// The "how it was asked" column shows only when the panel is wide enough for it.
const HOW_MIN_WIDTH: usize = 90;
const HOW_W: usize = 24;

/// `#rrggbb` as a color, for the swatch beside a color value.
pub fn swatch(v: &str) -> Option<Rgb> {
    let h = v.strip_prefix('#').filter(|h| h.len() == 6)?;
    let n = u32::from_str_radix(h, 16).ok()?;
    Some(Rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}

fn machine_line(r: &ProbeResults, max: usize) -> String {
    let m = &r.machine;
    let mut parts = vec![format!("{}×{} cells", r.cols, r.rows), m.cpu.clone(), m.os.clone()];
    if !m.model.is_empty() {
        parts.push(m.model.clone());
    }
    parts.retain(|p| !p.is_empty());
    while parts.len() > 1 && parts.join("  ·  ").chars().count() > max {
        parts.pop();
    }
    fit(&parts.join("  ·  "), max)
}

fn feature_line(f: &Row, width: usize) -> Line {
    let mut l = Line::new();
    l.pad(2).text(&format!("{:<NAME_W$}", fit(&f.name, NAME_W)), TEXT).pad(1);
    let show_how = width >= HOW_MIN_WIDTH;
    let room = width.saturating_sub(2 + NAME_W + 1 + 2 + if show_how { HOW_W + 2 } else { 0 });
    let start = l.width();
    match &f.outcome {
        Outcome::Yes => {
            l.bold("✓", GOOD);
        }
        Outcome::No => {
            l.bold("✗", BAD);
        }
        Outcome::Unknown => {
            l.text("?", DIM);
        }
        Outcome::Value(v) => {
            if let Some(c) = swatch(v) {
                l.text("██", c).pad(1);
            }
            let left = room.saturating_sub(l.width() - start);
            l.bold(&fit(v, left), TEXT);
        }
    }
    if !f.note.is_empty() {
        let left = room.saturating_sub(l.width() - start + 2);
        if left > 0 {
            l.text(&format!("  {}", fit(&f.note, left)), DIM);
        }
    }
    if show_how {
        l.pad_to(2 + NAME_W + 1 + room + 2).text(&fit(&f.how, HOW_W), FAINT);
    }
    l
}

fn cell(l: &mut Line, p: &WidthPass, expected: u8, disputed: bool, end: usize) {
    match p.width {
        None => {
            l.right("?", DIM, end);
        }
        Some(w) if disputed => {
            l.right(&format!("{w} ~"), DIM, end);
        }
        Some(w) if w == expected => {
            // The mark is one cell, so the number is placed first and the mark after it.
            l.pad_to(end - 3).text(&format!("{w} "), TEXT).bold("✓", GOOD);
        }
        Some(w) => {
            l.pad_to(end - 3).text(&format!("{w} "), BAD).bold("✗", BAD);
        }
    }
}

fn width_line(w: &WidthResult, x: &WidthCols) -> Line {
    let mut l = Line::new();
    l.pad(2).text(&format!("{:<NAME_W$}", fit(&w.name, NAME_W)), TEXT);
    let room = x.expected.saturating_sub(8 + l.width() + 1);
    if room >= 8 {
        l.pad(1).text(&fit(&w.codepoints, room), FAINT);
    }
    l.right(&w.expected.to_string(), DIM, x.expected);
    cell(&mut l, &w.as_is, w.expected, w.disputed, x.as_is);
    if let (Some(p), Some(end)) = (&w.with_2027, x.second) {
        cell(&mut l, p, w.expected, w.disputed, end);
    }
    l
}

struct WidthCols {
    expected: usize,
    as_is: usize,
    second: Option<usize>,
}

fn width_cols(width: usize, with_2027: bool) -> WidthCols {
    let last = width - 2;
    if with_2027 {
        WidthCols { expected: last - 16, as_is: last - 8, second: Some(last) }
    } else {
        WidthCols { expected: last - 8, as_is: last, second: None }
    }
}

fn score_text(s: Option<(usize, usize)>) -> String {
    s.map_or("not measured".to_string(), |(m, n)| format!("{m} of {n} match"))
}

fn score_color(s: Option<(usize, usize)>) -> Rgb {
    match s {
        Some((m, n)) if m == n => GOOD,
        Some(_) => BAD,
        None => DIM,
    }
}

pub fn results_panel(r: &ProbeResults, width: usize, saved: Option<&str>) -> Panel {
    let mut p = Panel::new(width);
    let inner = width.saturating_sub(4);
    p.blank();
    let mut l = Line::new();
    l.pad(2).title("ttyprobe").text("  terminal feature check", DIM);
    l.right(&r.date, DIM, width - 2);
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
    p.blank();
    p.rule(2);

    for section in Section::ALL {
        let rows: Vec<&Row> = r.features.iter().filter(|f| f.section == section).collect();
        if rows.is_empty() {
            continue;
        }
        p.blank();
        let mut l = Line::new();
        l.pad(2).bold(&section.label().to_uppercase(), FAINT);
        p.push(l);
        for f in rows {
            p.push(feature_line(f, width));
        }
    }

    if !r.widths.is_empty() {
        let second = r.has_2027_pass();
        let x = width_cols(width, second);
        p.blank();
        let mut l = Line::new();
        l.pad(2).bold("UNICODE WIDTHS", FAINT);
        l.right("expected", DIM, x.expected).right("as-is", DIM, x.as_is);
        if let Some(end) = x.second {
            l.right("2027", DIM, end);
        }
        p.push(l);
        for w in &r.widths {
            p.push(width_line(w, &x));
        }
        let mut l = Line::new();
        let a = r.score(false);
        l.pad(2).text("score  ", DIM).bold(&format!("as-is {}", score_text(a)), score_color(a));
        if second {
            let b = r.score(true);
            l.text("   ", DIM).bold(&format!("with mode 2027 {}", score_text(b)), score_color(b));
        }
        p.push(l);
    }
    p.blank();
    p.rule(2);
    let mut l = Line::new();
    l.pad(2)
        .text(&fit("✗ not supported or not announced   ? no answer   ~ disputed, not scored", inner), FAINT);
    p.push(l);
    let mut l = Line::new();
    l.pad(2).text(&fit("What was sent and received: ttyprobe --details", inner), FAINT);
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

fn plain_result(f: &Row) -> String {
    let base = match &f.outcome {
        Outcome::Yes => "yes".to_string(),
        Outcome::No => "no".to_string(),
        Outcome::Unknown => "unknown".to_string(),
        Outcome::Value(v) => v.clone(),
    };
    if f.note.is_empty() { base } else { format!("{base} ({})", f.note) }
}

/// The results as plain text, without colors or escape sequences: `--plain`, for logs and scripts.
pub fn results_plain(r: &ProbeResults, saved: Option<&str>) -> String {
    let mut s = format!("ttyprobe {}  ·  {}  ·  {}\n", r.version, r.label, machine_line(r, usize::MAX));
    for section in Section::ALL {
        let rows: Vec<&Row> = r.features.iter().filter(|f| f.section == section).collect();
        if rows.is_empty() {
            continue;
        }
        s.push_str(&format!("\n{}\n", section.label().to_uppercase()));
        for f in rows {
            s.push_str(format!("  {:<NAME_W$} {}", f.name, plain_result(f)).trim_end());
            s.push('\n');
        }
    }
    if !r.widths.is_empty() {
        s.push_str("\nUNICODE WIDTHS\n");
        let show = |p: &WidthPass| p.width.map_or("?".to_string(), |w| w.to_string());
        for w in &r.widths {
            let second = w.with_2027.as_ref().map(|p| format!("  2027 {}", show(p))).unwrap_or_default();
            let tag = if w.disputed { "  (disputed)" } else { "" };
            s.push_str(&format!(
                "  {:<NAME_W$} expected {}  as-is {}{second}{tag}\n",
                w.name,
                w.expected,
                show(&w.as_is)
            ));
        }
        s.push_str(&format!("  score: as-is {}", score_text(r.score(false))));
        if r.has_2027_pass() {
            s.push_str(&format!("; with mode 2027 {}", score_text(r.score(true))));
        }
        s.push('\n');
    }
    if let Some(path) = saved {
        s.push_str(&format!("\nsaved to {path}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::super::model::sample;
    use super::*;

    #[test]
    fn plain_results_have_no_escapes_and_name_every_row() {
        let r = sample("a", &[(1, Some(1)), (2, Some(1)), (2, Some(1))]);
        let text = results_plain(&r, Some("p.json"));
        assert!(!text.contains('\x1b'), "{text}");
        for f in &r.features {
            assert!(text.contains(&f.name), "{}", f.name);
        }
        assert!(text.contains("score: as-is 1 of 2 match; with mode 2027 2 of 2 match"), "{text}");
        assert!(text.contains("(disputed)") && text.contains("saved to p.json"), "{text}");
    }

    #[test]
    fn every_line_fits_the_panel() {
        let mut r = sample("a", &[(1, Some(1)), (2, Some(1)), (2, None)]);
        for width in [64, 80, 90, 100] {
            for second in [true, false] {
                if !second {
                    for w in &mut r.widths {
                        w.with_2027 = None;
                    }
                }
                let p = results_panel(&r, width, Some("/tmp/a.json"));
                for (i, line) in p.lines().iter().enumerate() {
                    assert!(line.width() <= width, "width {width}, line {i}: {}", line.width());
                }
            }
        }
    }

    #[test]
    fn swatches_read_hex() {
        assert_eq!(swatch("#1e1e2e"), Some(Rgb(30, 30, 46)));
        assert_eq!(swatch("dark"), None);
        assert_eq!(swatch("#12"), None);
    }
}
