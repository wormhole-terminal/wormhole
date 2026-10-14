//! `ttystorm compare` for probe files: one matrix, the features and width cases down the side and
//! a terminal in each column, with the rows where terminals differ marked.

use std::io::{self, Write};
use std::path::PathBuf;

use super::model::{Outcome, ProbeResults, Section};
use super::report;
use crate::bench::json;
use crate::bench::ui::{ACCENT, BAD, DIM, FAINT, GOOD, Line, Panel, TEXT, fit};
use crate::canvas::Rgb;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Good,
    Bad,
    Plain,
    Dim,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cell {
    text: String,
    tone: Tone,
}

impl Cell {
    fn new(text: &str, tone: Tone) -> Cell {
        Cell { text: text.to_string(), tone }
    }

    fn missing() -> Cell {
        Cell::new("–", Tone::Dim)
    }
}

struct MRow {
    name: String,
    cells: Vec<Cell>,
    /// The terminals disagree on this row.
    differs: bool,
}

struct Group {
    title: String,
    rows: Vec<MRow>,
}

/// Whether a file is a probe file, without reading it as one: how `compare` tells the two kinds of
/// result apart.
pub fn is_probe_file(text: &str) -> bool {
    json::parse(text)
        .ok()
        .and_then(|v| v.get("format").and_then(json::Value::as_str).map(|f| f == super::model::FORMAT))
        == Some(true)
}

fn feature_cell(o: Option<&Outcome>) -> Cell {
    match o {
        Some(Outcome::Yes) => Cell::new("✓", Tone::Good),
        Some(Outcome::No) => Cell::new("✗", Tone::Bad),
        Some(Outcome::Unknown) => Cell::new("?", Tone::Dim),
        Some(Outcome::Value(v)) => Cell::new(v, Tone::Plain),
        None => Cell::missing(),
    }
}

/// What kind of answer a cell is, for deciding whether terminals differ on it.
fn kind(o: Option<&Outcome>) -> &'static str {
    match o {
        Some(Outcome::Yes) => "yes",
        Some(Outcome::No) => "no",
        Some(Outcome::Unknown) => "unknown",
        Some(Outcome::Value(_)) => "value",
        None => "missing",
    }
}

fn differ<T: PartialEq>(items: impl Iterator<Item = T>) -> bool {
    let all: Vec<T> = items.collect();
    all.iter().any(|x| *x != all[0])
}

fn matrix(all: &[ProbeResults]) -> Vec<Group> {
    let mut groups = Vec::new();
    for section in Section::ALL {
        let mut names: Vec<&str> = Vec::new();
        for f in all.iter().flat_map(|r| &r.features).filter(|f| f.section == section) {
            if !names.contains(&f.name.as_str()) {
                names.push(&f.name);
            }
        }
        if names.is_empty() {
            continue;
        }
        let rows = names
            .into_iter()
            .map(|name| {
                let found: Vec<Option<&Outcome>> =
                    all.iter().map(|r| r.feature(name).map(|f| &f.outcome)).collect();
                // What a terminal says about itself or its window is expected to differ.
                let comparable = !matches!(section, Section::Identity | Section::Sizes);
                MRow {
                    name: name.to_string(),
                    cells: found.iter().map(|o| feature_cell(*o)).collect(),
                    differs: comparable && differ(found.iter().map(|o| kind(*o))),
                }
            })
            .collect();
        groups.push(Group { title: section.label().to_string(), rows });
    }
    groups.extend(width_groups(all));
    groups
}

fn width_groups(all: &[ProbeResults]) -> Vec<Group> {
    let mut cases: Vec<(&str, u8, bool)> = Vec::new();
    for w in all.iter().flat_map(|r| &r.widths) {
        if !cases.iter().any(|c| c.0 == w.name) {
            cases.push((&w.name, w.expected, w.disputed));
        }
    }
    if cases.is_empty() {
        return Vec::new();
    }
    let mut groups = Vec::new();
    for second in [false, true] {
        if second && !all.iter().any(ProbeResults::has_2027_pass) {
            continue;
        }
        let mut rows: Vec<MRow> = cases
            .iter()
            .map(|(name, expected, disputed)| {
                let widths: Vec<Option<u8>> = all
                    .iter()
                    .map(|r| {
                        let w = r.widths.iter().find(|w| w.name == *name)?;
                        if second { w.with_2027.as_ref()?.width } else { w.as_is.width }
                    })
                    .collect();
                let present: Vec<&Option<u8>> = widths.iter().filter(|w| w.is_some()).collect();
                MRow {
                    name: format!("{name} ({expected})"),
                    cells: widths
                        .iter()
                        .map(|w| match w {
                            None => Cell::missing(),
                            Some(n) if *disputed => Cell::new(&n.to_string(), Tone::Dim),
                            Some(n) if n == expected => Cell::new(&n.to_string(), Tone::Good),
                            Some(n) => Cell::new(&n.to_string(), Tone::Bad),
                        })
                        .collect(),
                    differs: differ(present.into_iter()),
                }
            })
            .collect();
        rows.push(MRow {
            name: "score".to_string(),
            cells: all
                .iter()
                .map(|r| match r.score(second) {
                    Some((m, n)) => {
                        Cell::new(&format!("{m} of {n}"), if m == n { Tone::Good } else { Tone::Bad })
                    }
                    None => Cell::missing(),
                })
                .collect(),
            differs: false,
        });
        let title =
            if second { "Unicode widths with mode 2027 set" } else { "Unicode widths, as the terminal is" };
        groups.push(Group { title: title.to_string(), rows });
    }
    groups
}

fn tone_color(t: Tone) -> Rgb {
    match t {
        Tone::Good => GOOD,
        Tone::Bad => BAD,
        Tone::Plain => TEXT,
        Tone::Dim => DIM,
    }
}

fn panel(all: &[ProbeResults], width: usize) -> Panel {
    let groups = matrix(all);
    let label_w =
        groups.iter().flat_map(|g| &g.rows).map(|r| r.name.chars().count()).max().unwrap_or(18).clamp(18, 34);
    let cell_w = groups
        .iter()
        .flat_map(|g| g.rows.iter().flat_map(|r| &r.cells))
        .map(|c| c.text.chars().count())
        .chain(all.iter().map(|r| r.label.chars().count()))
        .max()
        .unwrap_or(6)
        .clamp(6, 14);
    // margin, mark, name | each terminal: gap, cell | margin
    let needed = 2 + 2 + label_w + all.len() * (cell_w + 2) + 2;
    let width = width.max(needed);
    let accent = ACCENT;

    let mut p = Panel::new(width);
    p.blank();
    let mut l = Line::new();
    l.pad(2).title("ttyprobe").text("  terminals side by side", DIM);
    p.push(l);
    let mut l = Line::new();
    l.pad_to(4 + label_w);
    for r in all {
        l.pad(2).bold(&format!("{:<cell_w$}", fit(&r.label, cell_w)), TEXT);
    }
    p.push(l);
    let mut l = Line::new();
    l.pad_to(4 + label_w);
    for r in all {
        l.pad(2).text(&format!("{:<cell_w$}", fit(&format!("{}×{}", r.cols, r.rows), cell_w)), FAINT);
    }
    p.push(l);
    p.rule(2);
    for g in &groups {
        p.blank();
        let mut l = Line::new();
        l.pad(2).bold(&g.title.to_uppercase(), FAINT);
        p.push(l);
        for row in &g.rows {
            let mut l = Line::new();
            l.pad(2);
            if row.differs {
                l.bold("≠ ", accent);
            } else {
                l.pad(2);
            }
            let name = format!("{:<label_w$}", fit(&row.name, label_w));
            if row.differs {
                l.bold(&name, TEXT)
            } else {
                l.text(&name, DIM)
            };
            for c in &row.cells {
                l.pad(2).text(&format!("{:<cell_w$}", fit(&c.text, cell_w)), tone_color(c.tone));
            }
            p.push(l);
        }
    }
    p.blank();
    p.rule(2);
    let mut l = Line::new();
    l.pad(2).text(
        &fit("≠ the terminals differ here.  In the widths the expected cells are in parentheses;", width - 4),
        FAINT,
    );
    p.push(l);
    let mut l = Line::new();
    l.pad(2).text(
        &fit("green matches Unicode's grapheme clusters, red does not.  – = not in that file.", width - 4),
        FAINT,
    );
    p.push(l);
    p.blank();
    p
}

fn md_cell(c: &Cell) -> String {
    let t = c.text.replace('|', "\\|");
    if c.tone == Tone::Bad && c.text.parse::<u8>().is_ok() { format!("{t} ✗") } else { t }
}

fn markdown(all: &[ProbeResults]) -> String {
    let mut s = String::new();
    let labels: Vec<String> = all.iter().map(|r| r.label.replace('|', "\\|")).collect();
    s.push_str(&format!(
        "{}\n\n",
        all.iter().map(|r| format!("{} ({})", r.label, r.date)).collect::<Vec<_>>().join(", ")
    ));
    for g in matrix(all) {
        s.push_str(&format!(
            "### {}\n\n| | {} |\n|---|{}\n",
            g.title,
            labels.join(" | "),
            "---|".repeat(all.len())
        ));
        for row in &g.rows {
            let name = row.name.replace('|', "\\|");
            let name = if row.differs { format!("**{name}**") } else { name };
            let cells: Vec<String> = row.cells.iter().map(md_cell).collect();
            s.push_str(&format!("| {name} | {} |\n", cells.join(" | ")));
        }
        s.push('\n');
    }
    s.push_str("Bold rows are where the terminals differ. ✗ after a width: it does not match Unicode's grapheme clusters.\n");
    s
}

pub fn load(files: &[PathBuf]) -> io::Result<Vec<ProbeResults>> {
    let mut all = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(f)
            .map_err(|e| io::Error::other(format!("can't read {}: {e}", f.display())))?;
        all.push(
            ProbeResults::from_json(&text).map_err(|e| io::Error::other(format!("{}: {e}", f.display())))?,
        );
    }
    Ok(all)
}

pub fn compare(files: &[PathBuf], as_markdown: bool) -> io::Result<()> {
    if files.len() < 2 {
        return Err(io::Error::other("compare needs at least two result files (from `ttyprobe --save`)"));
    }
    let all = load(files)?;
    let mut out = Vec::new();
    if as_markdown {
        out.extend_from_slice(markdown(&all).as_bytes());
    } else {
        panel(&all, report::panel_width(crate::term::size().cols)).write(&mut out);
    }
    io::stdout().write_all(&out)?;
    io::stdout().flush()
}

#[cfg(test)]
mod tests {
    use super::super::model::sample;
    use super::*;

    fn two() -> Vec<ProbeResults> {
        vec![
            sample("a", &[(1, Some(1)), (2, Some(2)), (2, Some(1))]),
            sample("b", &[(1, Some(1)), (2, Some(2)), (2, Some(2))]),
        ]
    }

    #[test]
    fn probe_files_are_told_from_benchmark_files() {
        let r = sample("a", &[]);
        assert!(is_probe_file(&r.to_json()));
        assert!(!is_probe_file("{\"format\": \"ttystorm\", \"tests\": []}"));
        assert!(!is_probe_file("not json"));
    }

    #[test]
    fn rows_where_terminals_differ_are_marked() {
        let groups = matrix(&two());
        let find = |name: &str| {
            groups.iter().flat_map(|g| &g.rows).find(|r| r.name.starts_with(name)).map(|r| r.differs)
        };
        assert_eq!(find("grapheme clustering"), Some(true));
        assert_eq!(find("synchronized output"), Some(false));
        // Identity and sizes are expected to differ and are never marked.
        assert_eq!(find("terminal (XTVERSION)"), Some(false));
        assert_eq!(find("case 2"), Some(true));
        assert_eq!(find("case 0"), Some(false));
    }

    #[test]
    fn the_width_groups_score_each_terminal() {
        let groups = matrix(&two());
        let w = groups.iter().find(|g| g.title.starts_with("Unicode widths, as")).expect("widths");
        let score = w.rows.last().expect("score");
        assert_eq!(score.cells[0].text, "1 of 2");
        assert_eq!(score.cells[1].text, "2 of 2");
        assert!(groups.iter().any(|g| g.title.contains("mode 2027")));
    }

    #[test]
    fn the_matrix_panel_fits_its_width() {
        let p = panel(&two(), 80);
        for line in p.lines() {
            assert!(line.width() <= 80, "{}", line.width());
        }
    }

    #[test]
    fn markdown_is_a_table_per_section() {
        let md = markdown(&two());
        assert!(md.contains("### Modes\n\n| | a | b |\n|---|---|---|\n"), "{md}");
        assert!(md.contains("| **grapheme clustering (2027)** | ✓ | ✗ |"), "{md}");
        assert!(md.contains("| **case 2 (2)** | 1 ✗ | 2 |"), "{md}");
    }
}
