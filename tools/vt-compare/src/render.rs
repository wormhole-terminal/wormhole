//! The matrix as text, markdown and JSON.

use std::fmt::Write as _;

use serde_json::{Value, json};

use crate::matrix::{Matrix, Outcome, Row, Totals};

/// How a report is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Aligned text.
    Text,
    /// GitHub tables and headings.
    Markdown,
}

/// How much of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detail {
    /// The tables.
    Tables,
    /// The tables, then per engine every case it fails (hundreds of lines).
    WithFailureLists,
}

/// How a report is laid out.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Text or markdown.
    pub format: Format,
    /// Tables only, or the per-engine lists too.
    pub detail: Detail,
}

/// A table that prints as markdown or as aligned text.
struct Table {
    head: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    fn render(&self, format: Format) -> String {
        let markdown = format == Format::Markdown;
        let columns = self.head.len();
        let width = |i: usize| {
            std::iter::once(&self.head)
                .chain(self.rows.iter())
                .map(|r| r.get(i).map_or(0, |c| c.chars().count()))
                .max()
                .unwrap_or(0)
        };
        let widths: Vec<usize> = (0..columns).map(width).collect();
        let line = |cells: &[String]| {
            let mut out = String::new();
            for (i, w) in widths.iter().enumerate() {
                let cell = cells.get(i).map_or("", String::as_str);
                let pad = w - cell.chars().count();
                if markdown {
                    let _ = write!(out, "| {cell}{} ", " ".repeat(pad));
                } else {
                    let _ = write!(out, "{cell}{}  ", " ".repeat(pad));
                }
            }
            if markdown {
                out.push('|');
            } else {
                out.truncate(out.trim_end().len());
            }
            out.push('\n');
            out
        };
        let mut out = line(&self.head);
        if markdown {
            out.push_str(&widths.iter().fold(String::new(), |mut s, w| {
                let _ = write!(s, "|{}", "-".repeat(w + 2));
                s
            }));
            out.push_str("|\n");
        } else {
            out.push_str(&widths.iter().map(|w| "-".repeat(*w)).collect::<Vec<_>>().join("  "));
            out.push('\n');
        }
        for row in &self.rows {
            out.push_str(&line(row));
        }
        out
    }
}

fn totals_cell(t: &Totals) -> String {
    if t.na > 0 {
        format!("{}/{} (+{} n/a)", t.pass, t.applicable(), t.na)
    } else {
        format!("{}/{}", t.pass, t.applicable())
    }
}

fn mark(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Pass => "✓".to_string(),
        Outcome::Fail(f) if f.known => "✗ (known)".to_string(),
        Outcome::Fail(_) => "✗".to_string(),
        Outcome::Na(_) => "–".to_string(),
    }
}

fn heading(out: &mut String, style: Style, text: &str) {
    if style.format == Format::Markdown {
        let _ = writeln!(out, "\n## {text}\n");
    } else {
        let _ = writeln!(out, "\n{text}\n{}\n", "=".repeat(text.chars().count()));
    }
}

fn place(row: &Row) -> String {
    format!("{}:{}", row.area, row.line)
}

/// The report. `info` names each engine for the header (`engines::engine_info`), `unavailable`
/// the ones that could not be built.
pub fn report(matrix: &Matrix, info: &[String], unavailable: &[(&str, &str)], style: Style) -> String {
    let mut out = String::new();
    let spec = matrix.rows.iter().filter(|r| !r.convention).count();
    let convention = matrix.rows.len() - spec;
    let names = &matrix.engines;
    if style.format == Format::Markdown {
        out.push_str("# Conformance matrix\n\n");
    } else {
        out.push_str("Conformance matrix\n==================\n\n");
    }
    let _ = writeln!(
        out,
        "{} cases ({spec} from published text, {convention} conventions) run on {} engines. \
         A cell is passes/cases asked; n/a is a case the adapter cannot ask (no replies, no hyperlinks, ...).",
        matrix.rows.len(),
        names.len()
    );
    let list = |out: &mut String, text: &str| {
        let _ = writeln!(out, "{}{text}", if style.format == Format::Markdown { "- " } else { "  " });
    };
    out.push('\n');
    for (name, description) in names.iter().zip(info) {
        list(&mut out, &format!("{name}: {description}"));
    }
    for (name, why) in unavailable {
        list(&mut out, &format!("{name}: not run, {why}"));
    }

    // Passes per area.
    heading(&mut out, style, "Passes per area");
    let mut head = vec!["area".to_string(), "cases".to_string()];
    head.extend(names.iter().cloned());
    let mut table = Table { head, rows: Vec::new() };
    for (area, cases, totals) in matrix.areas() {
        let mut cells = vec![area, cases.to_string()];
        cells.extend(totals.iter().map(totals_cell));
        table.row(cells);
    }
    for (label, keep) in [
        ("all", Box::new(|_: &Row| true) as Box<dyn Fn(&Row) -> bool>),
        ("spec", Box::new(|r: &Row| !r.convention)),
        ("convention", Box::new(|r: &Row| r.convention)),
    ] {
        let totals = matrix.totals(keep.as_ref());
        let cases = matrix.rows.iter().filter(|r| keep(r)).count();
        let mut cells = vec![
            if style.format == Format::Markdown { format!("**{label}**") } else { label.to_string() },
            cases.to_string(),
        ];
        cells.extend(totals.iter().map(totals_cell));
        table.row(cells);
    }
    out.push_str(&table.render(style.format));

    // Conventions against the other engines: the ones some engine fails are the ones to review.
    heading(&mut out, style, "Our conventions against the other engines");
    let conventions: Vec<&Row> = matrix.rows.iter().filter(|r| r.convention).collect();
    let unanimous = conventions.iter().filter(|r| r.tally().1 == 0 && r.tally().0 > 0).count();
    let against = conventions.iter().filter(|r| r.failed_by_majority()).count();
    let split: Vec<&Row> = conventions.iter().copied().filter(|r| r.tally().1 > 0).collect();
    let _ = writeln!(
        out,
        "Of {} convention cases (behaviour no published text spells out): {unanimous} pass on every engine that \
         can be asked, {against} fail on most engines (a convention the others do not follow: wrong or \
         unusual, check it), the {} below fail on at least one.\n",
        conventions.len(),
        split.len()
    );
    if !split.is_empty() {
        out.push_str(&case_table(matrix, split.iter().copied()).render(style.format));
    }

    // Spec cases the majority fails.
    let spec_against: Vec<&Row> =
        matrix.rows.iter().filter(|r| !r.convention && r.failed_by_majority()).collect();
    if !spec_against.is_empty() {
        heading(
            &mut out,
            style,
            &format!("Cases from published text that most engines fail ({})", spec_against.len()),
        );
        out.push_str(&case_table(matrix, spec_against.iter().copied()).render(style.format));
    }

    // Disagreements among the cases from published text.
    let disagreements: Vec<&Row> =
        matrix.rows.iter().filter(|r| !r.convention && r.disputed() && !r.failed_by_majority()).collect();
    heading(
        &mut out,
        style,
        &format!("Cases from published text that some engines fail and most pass ({})", disagreements.len()),
    );
    out.push_str("`S` is a case from published text, `C` a convention; `–` is n/a.\n\n");
    out.push_str(&case_table(matrix, disagreements.iter().copied()).render(style.format));

    if style.detail == Detail::WithFailureLists {
        heading(&mut out, style, "What each engine fails");
        for (i, name) in names.iter().enumerate() {
            let failures: Vec<_> = matrix.failures_of(i).collect();
            let _ = writeln!(
                out,
                "{}{name} ({} cases)\n",
                if style.format == Format::Markdown { "### " } else { "" },
                failures.len()
            );
            for (row, failure) in failures {
                let split = if failure.split_only { ", only when fed a byte at a time" } else { "" };
                let known = if failure.known { ", known" } else { "" };
                let _ = writeln!(out, "- {} {} ({}{split}{known})", place(row), row.name, failure.step);
            }
            out.push('\n');
        }
    }
    out
}

/// One row per case, with a mark per engine.
fn case_table<'a>(matrix: &Matrix, rows: impl Iterator<Item = &'a Row>) -> Table {
    let mut head = vec!["case".to_string(), "".to_string(), "name".to_string()];
    head.extend(matrix.engines.iter().cloned());
    let mut table = Table { head, rows: Vec::new() };
    for row in rows {
        let mut cells =
            vec![place(row), if row.convention { "C" } else { "S" }.to_string(), row.name.clone()];
        cells.extend(row.outcomes.iter().map(mark));
        table.row(cells);
    }
    table
}

/// The matrix as JSON: engines, totals per area and every case with each engine's outcome.
pub fn json(matrix: &Matrix, info: &[String]) -> Value {
    let outcome = |o: &Outcome| match o {
        Outcome::Pass => json!({ "result": "pass" }),
        Outcome::Na(why) => json!({ "result": "n/a", "reason": why }),
        Outcome::Fail(f) => json!({
            "result": "fail",
            "step": f.step,
            "detail": f.detail,
            "only_when_split": f.split_only,
            "known_fail": f.known,
        }),
    };
    let totals = |t: &Totals| json!({ "pass": t.pass, "fail": t.fail, "na": t.na });
    json!({
        "format": "vt-matrix",
        "version": 1,
        "engines": matrix.engines.iter().zip(info).map(|(n, i)| json!({ "name": n, "what": i })).collect::<Vec<_>>(),
        "areas": matrix.areas().iter().map(|(area, cases, per_engine)| json!({
            "area": area,
            "cases": cases,
            "engines": matrix.engines.iter().zip(per_engine).map(|(n, t)| (n.clone(), totals(t))).collect::<serde_json::Map<_, _>>(),
        })).collect::<Vec<_>>(),
        "totals": matrix.engines.iter().zip(matrix.totals(|_| true)).map(|(n, t)| (n.clone(), totals(&t))).collect::<serde_json::Map<_, _>>(),
        "cases": matrix.rows.iter().map(|row| json!({
            "area": row.area,
            "line": row.line,
            "name": row.name,
            "basis": if row.convention { "convention" } else { "spec" },
            "basis_text": row.basis,
            "disputed": row.disputed(),
            "failed_by_majority": row.failed_by_majority(),
            "engines": matrix.engines.iter().zip(&row.outcomes).map(|(n, o)| (n.clone(), outcome(o))).collect::<serde_json::Map<_, _>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Detail, Format, Style, json, report};
    use crate::matrix::{Failure, Matrix, Outcome, Row};

    fn fail() -> Outcome {
        Outcome::Fail(Failure { step: "screen".into(), detail: "x".into(), split_only: false, known: false })
    }

    fn row(area: &str, name: &str, convention: bool, outcomes: Vec<Outcome>) -> Row {
        Row { area: area.into(), name: name.into(), line: 3, convention, basis: "b".into(), outcomes }
    }

    fn sample() -> Matrix {
        Matrix {
            engines: vec!["a".into(), "b".into(), "c".into()],
            rows: vec![
                row("wrap", "all agree", false, vec![Outcome::Pass, Outcome::Pass, Outcome::Pass]),
                row("wrap", "most fail", true, vec![Outcome::Pass, fail(), fail()]),
                row("cursor", "one fails", false, vec![Outcome::Pass, Outcome::Pass, fail()]),
                row(
                    "cursor",
                    "no replies",
                    false,
                    vec![Outcome::Pass, Outcome::Pass, Outcome::Na("no replies")],
                ),
            ],
        }
    }

    #[test]
    fn the_report_counts_passes_per_area_and_flags_what_most_engines_fail() {
        let info = vec!["A 1".to_string(), "B 2".to_string(), "C 3".to_string()];
        let text = report(
            &sample(),
            &info,
            &[("d", "not built")],
            Style { format: Format::Markdown, detail: Detail::WithFailureLists },
        );
        assert!(text.contains("| wrap "), "{text}");
        // wrap: 2 cases, engine a passes both, b one, c one.
        assert!(text.contains("| 2/2 ") && text.contains("| 1/2 "), "{text}");
        // cursor: c passes none of the two it can be asked (one n/a).
        assert!(text.contains("0/1 (+1 n/a)"), "{text}");
        assert!(text.contains("1 fail on most"), "{text}");
        assert!(text.contains("d: not run, not built"), "{text}");
        assert!(text.contains("- wrap:3 most fail (screen)"), "{text}");
    }

    #[test]
    fn the_plain_report_has_no_table_pipes() {
        let info = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let text = report(&sample(), &info, &[], Style { format: Format::Text, detail: Detail::Tables });
        assert!(!text.contains('|'), "{text}");
        assert!(text.starts_with("Conformance matrix\n=="), "{text}");
    }

    #[test]
    fn json_has_every_case_with_every_engine() {
        let info = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let value = json(&sample(), &info);
        assert_eq!(value["cases"].as_array().map(Vec::len), Some(4));
        assert_eq!(value["cases"][1]["engines"]["b"]["result"], "fail");
        assert_eq!(value["cases"][3]["engines"]["c"]["reason"], "no replies");
        assert_eq!(value["totals"]["a"]["pass"], 4);
        assert_eq!(value["cases"][1]["failed_by_majority"], true);
    }
}
