//! Running cases against an engine and saying what differs.

use std::fmt::Write as _;

use super::escape::glob_match;
use super::{Case, CellCheck, ReplyExpect, StepKind};
use crate::engine::{Cell, Engine};

/// How the input reaches the engine. A correct engine does not care; feeding one byte at a time
/// splits every escape sequence and every UTF-8 character at every possible place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chunking {
    Whole,
    /// Pieces of this many bytes.
    Bytes(usize),
}

impl Chunking {
    fn label(self) -> String {
        match self {
            Chunking::Whole => String::new(),
            Chunking::Bytes(1) => " [input fed one byte at a time]".to_string(),
            Chunking::Bytes(n) => format!(" [input fed {n} bytes at a time]"),
        }
    }
}

/// How one case went on one engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseResult {
    Passed,
    /// The engine gets it wrong as the case says it does (`known-fail`).
    KnownFail {
        reason: String,
    },
    /// A `known-fail` case that passes: the mark can go.
    UnexpectedPass {
        reason: String,
    },
    /// A real failure, already formatted with its diff.
    Failed(String),
}

/// Run `case` once for each of the ways the input can arrive (all at once and byte by byte).
pub fn run_case<E: Engine>(case: &Case) -> CaseResult {
    let mut failure = None;
    for chunking in [Chunking::Whole, Chunking::Bytes(1)] {
        if let Err(message) = run_once::<E>(case, chunking) {
            failure = Some(message);
            break;
        }
    }
    let known = case.known_fail.iter().find(|k| k.engine == E::NAME);
    match (failure, known) {
        (None, None) => CaseResult::Passed,
        (None, Some(k)) => CaseResult::UnexpectedPass { reason: k.reason.clone() },
        (Some(_), Some(k)) => CaseResult::KnownFail { reason: k.reason.clone() },
        (Some(message), None) => CaseResult::Failed(message),
    }
}

/// What running a whole suite on one engine found.
#[derive(Debug, Default)]
pub struct Report {
    pub engine: &'static str,
    pub passed: usize,
    pub failed: Vec<String>,
    /// `file:line name – reason`
    pub known_failures: Vec<String>,
    pub unexpected_passes: Vec<String>,
}

impl Report {
    pub fn total(&self) -> usize {
        self.passed + self.failed.len() + self.known_failures.len() + self.unexpected_passes.len()
    }

    /// A one-paragraph summary plus every failure; `verbose` adds the known failures too.
    pub fn render(&self, verbose: bool) -> String {
        let mut out = String::new();
        for failure in &self.failed {
            let _ = writeln!(out, "{failure}");
        }
        let _ = writeln!(
            out,
            "conformance on {}: {} cases, {} passed, {} FAILED, {} known failures, {} unexpected passes",
            self.engine,
            self.total(),
            self.passed,
            self.failed.len(),
            self.known_failures.len(),
            self.unexpected_passes.len()
        );
        if !self.unexpected_passes.is_empty() {
            let _ = writeln!(out, "known-fail marks that can go (the engine passes them now):");
            for line in &self.unexpected_passes {
                let _ = writeln!(out, "  {line}");
            }
        }
        if verbose && !self.known_failures.is_empty() {
            let _ = writeln!(out, "known failures:");
            for line in &self.known_failures {
                let _ = writeln!(out, "  {line}");
            }
        }
        out
    }
}

/// Run every case on engine `E`. Cases whose name does not contain `filter` are skipped.
pub fn run_suite<E: Engine>(cases: &[Case], filter: Option<&str>) -> Report {
    let mut report = Report { engine: E::NAME, ..Report::default() };
    for case in cases.iter().filter(|c| filter.is_none_or(|f| c.name.contains(f))) {
        let origin = format!("{}:{} {}", case.file, case.line, case.name);
        match run_case::<E>(case) {
            CaseResult::Passed => report.passed += 1,
            CaseResult::KnownFail { reason } => report.known_failures.push(format!("{origin} – {reason}")),
            CaseResult::UnexpectedPass { reason } => {
                report.unexpected_passes.push(format!("{origin} – was: {reason}"));
            }
            CaseResult::Failed(message) => report.failed.push(message),
        }
    }
    report
}

fn run_once<E: Engine>(case: &Case, chunking: Chunking) -> Result<(), String> {
    let mut engine = E::new(case.cols, case.rows, case.scrollback_limit);
    if case.allow_screen_checksum {
        engine.allow_screen_checksum();
    }
    if case.allow_window_ops {
        engine.allow_window_ops();
    }
    if case.allow_title_report {
        engine.allow_title_report();
    }
    let mut replies: Vec<u8> = Vec::new();
    let mut fed = Vec::new();
    for step in &case.steps {
        let outcome = match &step.kind {
            StepKind::Feed(bytes) => {
                fed.extend_from_slice(bytes);
                match chunking {
                    Chunking::Whole => engine.feed(bytes),
                    Chunking::Bytes(n) => bytes.chunks(n).for_each(|chunk| engine.feed(chunk)),
                }
                Ok(())
            }
            StepKind::Resize { cols, rows } => {
                engine.resize(*cols, *rows);
                Ok(())
            }
            other => {
                replies.extend(engine.take_replies());
                check(&mut engine, &mut replies, other)
            }
        };
        if let Err(detail) = outcome {
            let mut message = format!("FAIL {}:{} {}{}\n", case.file, case.line, case.name, chunking.label());
            if let Some(basis) = &case.basis {
                let _ = writeln!(message, "  {}", basis.label());
            }
            let _ = writeln!(message, "  input: {}", show_bytes(&fed));
            let _ = writeln!(message, "  line {} ({}):", step.line, step_name(&step.kind));
            for line in detail.lines() {
                let _ = writeln!(message, "    {line}");
            }
            return Err(message);
        }
    }
    Ok(())
}

fn step_name(kind: &StepKind) -> &'static str {
    match kind {
        StepKind::Feed(_) => "feed",
        StepKind::Resize { .. } => "resize",
        StepKind::Screen(_) => "screen",
        StepKind::Row { .. } => "row",
        StepKind::Cursor { .. } => "cursor",
        StepKind::Cell { .. } => "cell",
        StepKind::Reply(_) => "reply",
        StepKind::Title(_) => "title",
        StepKind::AltScreen(_) => "alt-screen",
        StepKind::Scrollback(_) => "scrollback",
        StepKind::ScrollbackLen(_) => "scrollback-len",
        StepKind::Lines(_) => "lines",
        StepKind::Wrapped { .. } => "wrapped",
    }
}

/// Check one step; the error says what was expected and what the engine has.
fn check<E: Engine>(engine: &mut E, replies: &mut Vec<u8>, kind: &StepKind) -> Result<(), String> {
    let (cols, rows) = engine.size();
    match kind {
        StepKind::Feed(_) | StepKind::Resize { .. } => Ok(()),
        StepKind::Screen(expected) => {
            let actual: Vec<String> = (0..rows).map(|r| engine.row_text(r)).collect();
            let mut want: Vec<String> = expected.clone();
            if want.len() > rows {
                return Err(format!("the case lists {} rows, the screen has {rows}", want.len()));
            }
            want.resize(rows, String::new());
            lines_equal("screen", &want, &actual)
        }
        StepKind::Row { row, text } => {
            if *row > rows {
                return Err(format!("row {row} is below the screen ({rows} rows)"));
            }
            let actual = engine.row_text(row - 1);
            if actual == *text {
                Ok(())
            } else {
                Err(format!("row {row}\nexpected |{text}|\n  actual |{actual}|"))
            }
        }
        StepKind::Cursor { row, col, visible } => {
            let cursor = engine.cursor();
            let (arow, acol) = (cursor.row + 1, cursor.col + 1);
            let mut problems = Vec::new();
            if row.is_some_and(|r| r != arow) || *col != acol {
                let want_row = row.map_or("any".to_string(), |r| r.to_string());
                problems.push(format!(
                    "cursor expected at row {want_row}, col {col}; it is at row {arow}, col {acol}"
                ));
            }
            if let Some(visible) = visible
                && *visible != cursor.visible
            {
                let word = |v: bool| if v { "visible" } else { "hidden" };
                problems.push(format!("cursor expected {}; it is {}", word(*visible), word(cursor.visible)));
            }
            if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
        }
        StepKind::Cell { row, col, checks } => {
            if *row > rows || *col > cols {
                return Err(format!("cell {row},{col} is outside the {cols}x{rows} screen"));
            }
            let cell = engine.cell(row - 1, col - 1);
            let problems: Vec<String> = checks.iter().filter_map(|c| cell_problem(&cell, c)).collect();
            if problems.is_empty() {
                Ok(())
            } else {
                Err(format!("cell {row},{col}\n{}", problems.join("\n")))
            }
        }
        StepKind::Reply(expect) => {
            let actual = std::mem::take(replies);
            let ok = match expect {
                ReplyExpect::Exact(want) => *want == actual,
                ReplyExpect::Glob(pattern) => glob_match(pattern, &actual),
                ReplyExpect::Nothing => actual.is_empty(),
            };
            if ok {
                return Ok(());
            }
            let (label, want) = match expect {
                ReplyExpect::Exact(want) => ("expected", show_bytes(want)),
                ReplyExpect::Glob(pattern) => ("expected like", show_bytes(pattern)),
                ReplyExpect::Nothing => ("expected", "no reply".to_string()),
            };
            let got = if actual.is_empty() { "no reply".to_string() } else { show_bytes(&actual) };
            Err(format!(
                "reply {label} {want}\nreply actual{:w$} {got}",
                "",
                w = label.len() - "actual".len()
            ))
        }
        StepKind::Title(expected) => {
            let actual = engine.title();
            if actual == *expected {
                Ok(())
            } else {
                Err(format!("title expected {expected:?}\n      actual {actual:?}"))
            }
        }
        StepKind::AltScreen(expected) => {
            let actual = engine.alt_screen();
            if actual == *expected {
                Ok(())
            } else {
                Err(format!("alternate screen expected {}; it is {}", on_off(*expected), on_off(actual)))
            }
        }
        StepKind::Scrollback(expected) => {
            let actual: Vec<String> =
                (0..engine.scrollback_len()).map(|i| engine.scrollback_text(i)).collect();
            lines_equal("scrollback (oldest first)", expected, &actual)
        }
        StepKind::Lines(expected) => {
            let actual = logical_lines(engine);
            let mut want = expected.clone();
            while want.last().is_some_and(|l| l.is_empty()) {
                want.pop();
            }
            lines_equal("logical lines (history and screen, wrapped rows joined)", &want, &actual)
        }
        StepKind::ScrollbackLen(expected) => {
            let actual = engine.scrollback_len();
            if actual == *expected {
                Ok(())
            } else {
                Err(format!("scrollback expected {expected} lines; it has {actual}"))
            }
        }
        StepKind::Wrapped { row, wrapped } => {
            if *row > rows {
                return Err(format!("row {row} is below the screen ({rows} rows)"));
            }
            let actual = engine.row_wrapped(row - 1);
            if actual == *wrapped {
                Ok(())
            } else {
                let word = |w: bool| if w { "wrapped into the next row" } else { "not wrapped" };
                Err(format!("row {row} expected {}; it is {}", word(*wrapped), word(actual)))
            }
        }
    }
}

/// The text the program wrote, as lines: history first, rows that continue by wrapping joined,
/// trailing empty lines dropped.
fn logical_lines<E: Engine>(engine: &E) -> Vec<String> {
    let (_, rows) = engine.size();
    let mut lines = Vec::new();
    let mut current = String::new();
    let history = engine.scrollback_len();
    for i in 0..history + rows {
        let (text, wrapped) = if i < history {
            (engine.scrollback_text_raw(i), engine.scrollback_wrapped(i))
        } else {
            (engine.row_text_raw(i - history), engine.row_wrapped(i - history))
        };
        // A wrapped row keeps its trailing blanks (they are text the program wrote); the last row
        // of a line does not, as `row_text` says.
        if wrapped {
            current.push_str(&text);
        } else {
            current.push_str(text.trim_end_matches(' '));
            lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        lines.push(current.trim_end_matches(' ').to_string());
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

fn on_off(v: bool) -> &'static str {
    if v { "on" } else { "off" }
}

fn cell_problem(cell: &Cell, check: &CellCheck) -> Option<String> {
    match check {
        CellCheck::Text(want) if *want != cell.text => {
            Some(format!("text expected {want:?}; it is {:?}", cell.text))
        }
        CellCheck::Width(want) if *want != cell.width => {
            Some(format!("width expected {want}; it is {}", cell.width))
        }
        CellCheck::Fg(want) if *want != cell.fg => {
            Some(format!("fg expected {}; it is {}", want.describe(), cell.fg.describe()))
        }
        CellCheck::Bg(want) if *want != cell.bg => {
            Some(format!("bg expected {}; it is {}", want.describe(), cell.bg.describe()))
        }
        CellCheck::Underline(want) if *want != cell.underline_color => Some(format!(
            "underline color expected {}; it is {}",
            want.describe(),
            cell.underline_color.describe()
        )),
        CellCheck::Attrs(want) if *want != cell.attrs => {
            Some(format!("attrs expected {}; it is {}", want.describe(), cell.attrs.describe()))
        }
        CellCheck::Link(want) if *want != cell.link => {
            Some(format!("link expected {want:?}; it is {:?}", cell.link))
        }
        _ => None,
    }
}

/// Compare two lists of screen lines; the error shows every row that differs between `|`s, with
/// the rows around it so the position is clear.
fn lines_equal(what: &str, expected: &[String], actual: &[String]) -> Result<(), String> {
    if expected == actual {
        return Ok(());
    }
    let mut out = format!("{what}: expected (-) vs actual (+)\n");
    let count = expected.len().max(actual.len());
    for i in 0..count {
        let (e, a) = (expected.get(i), actual.get(i));
        if e == a {
            if let Some(line) = e {
                let _ = writeln!(out, "  {:>2} |{line}|", i + 1);
            }
        } else {
            if let Some(line) = e {
                let _ = writeln!(out, "- {:>2} |{line}|", i + 1);
            }
            if let Some(line) = a {
                let _ = writeln!(out, "+ {:>2} |{line}|", i + 1);
            }
        }
    }
    Err(out)
}

/// Bytes the way a `.vtt` file writes them: escapes for the unprintable, text as it is.
pub fn show_bytes(bytes: &[u8]) -> String {
    let mut out = String::from("\"");
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\x1b' => out.push_str("\\e"),
                '\r' => out.push_str("\\r"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                '\x07' => out.push_str("\\a"),
                '\x08' => out.push_str("\\b"),
                '\\' => out.push_str("\\\\"),
                c if c.is_control() => {
                    let _ = write!(out, "\\u{{{:x}}}", c as u32);
                }
                c => out.push(c),
            }
        }
        for byte in chunk.invalid() {
            let _ = write!(out, "\\x{byte:02x}");
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_chunking_label_says_the_size() {
        assert_eq!(super::Chunking::Whole.label(), "");
        assert_eq!(super::Chunking::Bytes(1).label(), " [input fed one byte at a time]");
        assert_eq!(super::Chunking::Bytes(3).label(), " [input fed 3 bytes at a time]");
    }

    use super::{CaseResult, run_case, run_suite, show_bytes};
    use crate::engine::{Cell, Cursor, Engine};
    use crate::engines::WormholeEngine;
    use crate::vtt::{Case, parse_file};

    fn cases(text: &str) -> Vec<Case> {
        parse_file("t.vtt", text).unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn bytes_are_shown_as_a_vtt_file_writes_them() {
        assert_eq!(show_bytes(b"\x1b[1mhi\r\n"), r#""\e[1mhi\r\n""#);
        assert_eq!(show_bytes(&[b'a', 0xff, 0x9b]), r#""a\xff\x9b""#);
        assert_eq!(show_bytes("日\u{85}".as_bytes()), r#""日\u{85}""#);
    }

    #[test]
    fn a_passing_case_passes() {
        let case = &cases("test ok\nconvention c\nsize 10 2\nfeed ab\nscreen\n|ab|\ncursor 1,3\n")[0];
        assert_eq!(run_case::<WormholeEngine>(case), CaseResult::Passed);
    }

    #[test]
    fn a_failure_shows_the_input_and_a_diff() {
        let case = &cases("test bad\nspec a manual\nsize 10 2\nfeed \\e[1mab\ncursor 1,3\nscreen\n|ax|\n")[0];
        let CaseResult::Failed(message) = run_case::<WormholeEngine>(case) else {
            panic!("should fail");
        };
        assert!(message.starts_with("FAIL t.vtt:1 bad\n"), "{message}");
        assert!(message.contains("spec: a manual"), "{message}");
        assert!(message.contains(r#"input: "\e[1mab""#), "{message}");
        assert!(message.contains("line 6 (screen)"), "{message}");
        assert!(message.contains("-  1 |ax|"), "{message}");
        assert!(message.contains("+  1 |ab|"), "{message}");
    }

    #[test]
    fn a_cursor_in_the_wrong_place_says_where_it_is() {
        let case = &cases("test cur\nspec s\nsize 10 2\nfeed ab\ncursor 2,2 hidden\n")[0];
        let CaseResult::Failed(message) = run_case::<WormholeEngine>(case) else {
            panic!("should fail");
        };
        assert!(message.contains("expected at row 2, col 2; it is at row 1, col 3"), "{message}");
        assert!(message.contains("expected hidden; it is visible"), "{message}");
    }

    #[test]
    fn a_known_failure_is_not_a_failure_and_a_known_pass_is_reported() {
        let text = "test wrong\nspec s\nknown-fail: wormhole – on purpose\nsize 10 2\nfeed a\ncursor 1,1\n\n\
                    test right\nspec s\nknown-fail: wormhole – stale mark\nsize 10 2\nfeed a\ncursor 1,2\n\n\
                    test elsewhere\nspec s\nknown-fail: another – other engine\nsize 10 2\nfeed a\ncursor 1,1\n";
        let cases = cases(text);
        assert_eq!(
            run_case::<WormholeEngine>(&cases[0]),
            CaseResult::KnownFail { reason: "on purpose".into() }
        );
        assert_eq!(
            run_case::<WormholeEngine>(&cases[1]),
            CaseResult::UnexpectedPass { reason: "stale mark".into() }
        );
        let report = run_suite::<WormholeEngine>(&cases, None);
        assert_eq!((report.passed, report.failed.len(), report.known_failures.len()), (0, 1, 1));
        assert_eq!(report.unexpected_passes.len(), 1);
        assert!(report.render(true).contains("t.vtt:1 wrong – on purpose"));
        let only = run_suite::<WormholeEngine>(&cases, Some("right"));
        assert_eq!(only.total(), 1);
    }

    #[test]
    fn replies_are_checked_exactly_by_pattern_and_for_nothing() {
        let case = &cases(
            "test r\nspec s\nsize 10 2\nfeed \\e[5n\nreply \\e[0n\nreply none\nfeed \\e[6n\nreply~ \\e[*R\nreply none\n",
        )[0];
        assert_eq!(run_case::<WormholeEngine>(case), CaseResult::Passed);
        let wrong = &cases("test r\nspec s\nsize 10 2\nfeed \\e[5n\nreply \\e[1n\n")[0];
        let CaseResult::Failed(message) = run_case::<WormholeEngine>(wrong) else {
            panic!("should fail");
        };
        assert!(message.contains(r#"reply expected "\e[1n""#) && message.contains(r#""\e[0n""#), "{message}");
    }

    /// An engine whose screen depends on how many `feed` calls it saw, to show that the runner
    /// also feeds the input one byte at a time.
    struct CallCounter(usize);

    impl Engine for CallCounter {
        const NAME: &'static str = "counter";
        fn new(_: usize, _: usize, _: usize) -> Self {
            CallCounter(0)
        }
        fn feed(&mut self, _: &[u8]) {
            self.0 += 1;
        }
        fn resize(&mut self, _: usize, _: usize) {}
        fn size(&self) -> (usize, usize) {
            (10, 2)
        }
        fn row_text(&self, row: usize) -> String {
            if row == 0 { self.0.to_string() } else { String::new() }
        }
        fn row_wrapped(&self, _: usize) -> bool {
            false
        }
        fn cell(&self, _: usize, _: usize) -> Cell {
            Cell::blank()
        }
        fn cursor(&self) -> Cursor {
            Cursor { row: 0, col: 0, visible: true }
        }
        fn scrollback_len(&self) -> usize {
            0
        }
        fn scrollback_text(&self, _: usize) -> String {
            String::new()
        }
        fn scrollback_wrapped(&self, _: usize) -> bool {
            false
        }
        fn take_replies(&mut self) -> Vec<u8> {
            Vec::new()
        }
        fn title(&self) -> Option<String> {
            None
        }
        fn alt_screen(&self) -> bool {
            false
        }
    }

    #[test]
    fn input_is_also_fed_one_byte_at_a_time() {
        let case = &cases("test chunks\nspec s\nsize 10 2\nfeed abc\nscreen\n|1|\n")[0];
        let CaseResult::Failed(message) = run_case::<CallCounter>(case) else {
            panic!("a byte at a time is 3 calls");
        };
        assert!(message.contains("[input fed one byte at a time]"), "{message}");
    }
}
