//! Reading `.vtt` files.

use std::collections::HashMap;
use std::path::Path;

use super::escape::{unescape_bytes, unescape_text};
use super::{Basis, Case, CellCheck, KnownFail, ReplyExpect, Step, StepKind};
use crate::engine::{Attrs, CellColor};

/// Every `*.vtt` file in `dir`, in name order, as cases. All problems of all files are returned
/// together (`file:line: message`), so a broken suite is fixed in one go.
pub fn load_dir(dir: &Path) -> Result<Vec<Case>, Vec<String>> {
    let mut paths: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "vtt"))
            .collect(),
        Err(e) => return Err(vec![format!("{}: {e}", dir.display())]),
    };
    paths.sort();
    let mut cases = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_file(&name, &text) {
                Ok(mut parsed) => cases.append(&mut parsed),
                Err(mut e) => errors.append(&mut e),
            },
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    let mut first_seen: HashMap<&str, (&str, usize)> = HashMap::new();
    for case in &cases {
        if let Some((file, line)) = first_seen.insert(&case.name, (&case.file, case.line)) {
            errors.push(format!(
                "{}:{}: the name {:?} is already used at {file}:{line}",
                case.file, case.line, case.name
            ));
        }
    }
    if errors.is_empty() { Ok(cases) } else { Err(errors) }
}

/// The cases in one file's text; `file` is only used in messages.
pub fn parse_file(file: &str, text: &str) -> Result<Vec<Case>, Vec<String>> {
    let mut parser = Parser { file, cases: Vec::new(), errors: Vec::new(), current: None, block: None };
    for (index, raw) in text.lines().enumerate() {
        parser.line(index + 1, raw);
    }
    parser.finish_case();
    if parser.errors.is_empty() { Ok(parser.cases) } else { Err(parser.errors) }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Screen,
    Scrollback,
    Lines,
}

struct Block {
    line: usize,
    kind: BlockKind,
    rows: Vec<String>,
}

struct Builder {
    case: Case,
    has_size: bool,
}

struct Parser<'a> {
    file: &'a str,
    cases: Vec<Case>,
    errors: Vec<String>,
    current: Option<Builder>,
    block: Option<Block>,
}

impl Parser<'_> {
    fn error(&mut self, line: usize, message: impl std::fmt::Display) {
        self.errors.push(format!("{}:{line}: {message}", self.file));
    }

    fn line(&mut self, number: usize, raw: &str) {
        let line = raw.trim_end();
        if self.block.is_some() {
            if let Some(row) = line.trim_start().strip_prefix('|') {
                self.block_row(number, row);
                return;
            }
            self.close_block();
        }
        let line = line.trim_start();
        if line.is_empty() || line.starts_with('#') {
            return;
        }
        let (key, rest) = match line.split_once(char::is_whitespace) {
            Some((key, rest)) => (key, rest.trim_start()),
            None => (line, ""),
        };
        if key == "test" {
            self.finish_case();
            if rest.is_empty() {
                self.error(number, "test needs a name");
                return;
            }
            self.current = Some(Builder {
                case: Case {
                    name: rest.to_string(),
                    file: self.file.to_string(),
                    line: number,
                    basis: None,
                    cols: 0,
                    rows: 0,
                    scrollback_limit: 100,
                    allow_screen_checksum: false,
                    allow_window_ops: false,
                    known_fail: Vec::new(),
                    steps: Vec::new(),
                },
                has_size: false,
            });
            return;
        }
        let Some(mut builder) = self.current.take() else {
            self.error(number, format!("`{key}` outside of a test (a test starts with `test <name>`)"));
            return;
        };
        if let Err(message) = self.directive(&mut builder, number, key, rest) {
            self.error(number, format!("{key}: {message}"));
        }
        self.current = Some(builder);
    }

    fn block_row(&mut self, number: usize, row: &str) {
        let Some(inner) = row.strip_suffix('|') else {
            self.error(number, "a screen row must end with `|` (it shows the trailing blanks)");
            return;
        };
        match unescape_text(inner) {
            Ok(text) => {
                if let Some(block) = &mut self.block {
                    block.rows.push(text.trim_end_matches(' ').to_string());
                }
            }
            Err(e) => self.error(number, e),
        }
    }

    fn close_block(&mut self) {
        let Some(block) = self.block.take() else { return };
        let kind = match block.kind {
            BlockKind::Screen => StepKind::Screen(block.rows),
            BlockKind::Scrollback => StepKind::Scrollback(block.rows),
            BlockKind::Lines => StepKind::Lines(block.rows),
        };
        if let Some(builder) = &mut self.current {
            builder.case.steps.push(Step { line: block.line, kind });
        }
    }

    fn finish_case(&mut self) {
        self.close_block();
        let Some(builder) = self.current.take() else { return };
        let line = builder.case.line;
        if !builder.has_size {
            self.error(line, format!("test {:?} has no `size <cols> <rows>`", builder.case.name));
        } else if !builder
            .case
            .steps
            .iter()
            .any(|s| !matches!(s.kind, StepKind::Feed(_) | StepKind::Resize { .. }))
        {
            self.error(line, format!("test {:?} checks nothing", builder.case.name));
        } else {
            self.cases.push(builder.case);
        }
    }

    fn directive(&mut self, b: &mut Builder, number: usize, key: &str, rest: &str) -> Result<(), String> {
        let kind = match key {
            "spec" | "convention" => {
                if rest.is_empty() {
                    return Err("say what the expected values come from".to_string());
                }
                if b.case.basis.is_some() {
                    return Err("a case has one `spec` or `convention` line".to_string());
                }
                let text = rest.to_string();
                b.case.basis = Some(if key == "spec" { Basis::Spec(text) } else { Basis::Convention(text) });
                return Ok(());
            }
            "size" => {
                let (cols, rows) = two_numbers(rest, ' ')?;
                if cols == 0 || rows == 0 {
                    return Err("a screen needs at least 1 column and 1 row".to_string());
                }
                b.case.cols = cols;
                b.case.rows = rows;
                b.has_size = true;
                return Ok(());
            }
            "scrollback-limit" => {
                b.case.scrollback_limit = rest.parse().map_err(|_| format!("{rest:?} is not a number"))?;
                return Ok(());
            }
            "screen-checksum" => {
                if rest != "allow" {
                    return Err("screen-checksum allow".to_string());
                }
                b.case.allow_screen_checksum = true;
                return Ok(());
            }
            "window-ops" => {
                if rest != "allow" {
                    return Err("window-ops allow".to_string());
                }
                b.case.allow_window_ops = true;
                return Ok(());
            }
            "known-fail:" => {
                b.case.known_fail.push(parse_known_fail(rest)?);
                return Ok(());
            }
            "feed" => StepKind::Feed(unescape_bytes(rest)?),
            "repeat" => {
                let (count, text) = rest.split_once(' ').ok_or("repeat <count> <text>")?;
                let count: usize = count.parse().map_err(|_| format!("{count:?} is not a number"))?;
                let unit = unescape_bytes(text.trim_start())?;
                StepKind::Feed(unit.repeat(count))
            }
            "resize" => {
                let (cols, rows) = two_numbers(rest, ' ')?;
                if cols == 0 || rows == 0 {
                    return Err("a screen needs at least 1 column and 1 row".to_string());
                }
                StepKind::Resize { cols, rows }
            }
            "screen" | "scrollback" | "lines" => {
                if !rest.is_empty() {
                    return Err("rows go on the following lines, each between `|` and `|`".to_string());
                }
                let kind = match key {
                    "screen" => BlockKind::Screen,
                    "scrollback" => BlockKind::Scrollback,
                    _ => BlockKind::Lines,
                };
                self.block = Some(Block { line: number, kind, rows: Vec::new() });
                return Ok(());
            }
            "scrollback-len" => {
                StepKind::ScrollbackLen(rest.parse().map_err(|_| format!("{rest:?} is not a number"))?)
            }
            "row" => {
                let (row, text) = rest.split_once(' ').ok_or("row <n> |text|")?;
                let row = one_based(row)?;
                let inner = text
                    .trim_start()
                    .strip_prefix('|')
                    .and_then(|t| t.strip_suffix('|'))
                    .ok_or("the text goes between `|` and `|`")?;
                StepKind::Row { row, text: unescape_text(inner)?.trim_end_matches(' ').to_string() }
            }
            "cursor" => {
                let mut words = rest.split_whitespace();
                let spot = words.next().ok_or("cursor <row>,<col>")?;
                let (row, col) = match spot.strip_prefix("*,") {
                    Some(col) => (None, one_based(col)?),
                    None => position(spot).map(|(row, col)| (Some(row), col))?,
                };
                let visible = match words.next() {
                    None => None,
                    Some("visible") => Some(true),
                    Some("hidden") => Some(false),
                    Some(other) => return Err(format!("{other:?} is neither visible nor hidden")),
                };
                StepKind::Cursor { row, col, visible }
            }
            "cell" => {
                let mut words = rest.split_whitespace();
                let (row, col) = position(words.next().ok_or("cell <row>,<col> key=value …")?)?;
                let checks = words.map(cell_check).collect::<Result<Vec<_>, _>>()?;
                if checks.is_empty() {
                    return Err("a cell check needs at least one key=value".to_string());
                }
                StepKind::Cell { row, col, checks }
            }
            "reply" => {
                if rest == "none" {
                    StepKind::Reply(ReplyExpect::Nothing)
                } else {
                    StepKind::Reply(ReplyExpect::Exact(unescape_bytes(rest)?))
                }
            }
            "reply~" => StepKind::Reply(ReplyExpect::Glob(unescape_bytes(rest)?)),
            "title" => StepKind::Title(if rest == "none" { None } else { Some(unescape_text(rest)?) }),
            "alt-screen" => StepKind::AltScreen(on_off(rest)?),
            "wrapped" => {
                let (row, answer) = rest.split_once(' ').ok_or("wrapped <row> yes|no")?;
                let wrapped = match answer.trim() {
                    "yes" => true,
                    "no" => false,
                    other => return Err(format!("{other:?} is neither yes nor no")),
                };
                StepKind::Wrapped { row: one_based(row)?, wrapped }
            }
            other => return Err(format!("unknown directive `{other}`")),
        };
        b.case.steps.push(Step { line: number, kind });
        Ok(())
    }
}

fn one_based(s: &str) -> Result<usize, String> {
    match s.trim().parse::<usize>() {
        Ok(n) if n >= 1 => Ok(n),
        _ => Err(format!("{s:?} is not a position counted from 1")),
    }
}

fn two_numbers(s: &str, sep: char) -> Result<(usize, usize), String> {
    let (a, b) = s.split_once(sep).ok_or_else(|| format!("expected two numbers, got {s:?}"))?;
    let num = |t: &str| t.trim().parse::<usize>().map_err(|_| format!("{t:?} is not a number"));
    Ok((num(a)?, num(b)?))
}

fn position(s: &str) -> Result<(usize, usize), String> {
    let (row, col) = s.split_once(',').ok_or_else(|| format!("expected <row>,<col>, got {s:?}"))?;
    Ok((one_based(row)?, one_based(col)?))
}

fn on_off(s: &str) -> Result<bool, String> {
    match s.trim() {
        "on" => Ok(true),
        "off" => Ok(false),
        other => Err(format!("{other:?} is neither on nor off")),
    }
}

fn parse_color(s: &str) -> Result<CellColor, String> {
    if s == "default" {
        return Ok(CellColor::Default);
    }
    if let Some(i) = s.strip_prefix("idx:") {
        return i.parse().map(CellColor::Indexed).map_err(|_| format!("{s:?}: a palette slot is 0-255"));
    }
    if let Some(hex) = s.strip_prefix("rgb:")
        && hex.len() == 6
        && let Ok(v) = u32::from_str_radix(hex, 16)
    {
        return Ok(CellColor::Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8));
    }
    Err(format!("{s:?} is not a color (default, idx:N or rgb:RRGGBB)"))
}

fn cell_check(word: &str) -> Result<CellCheck, String> {
    let (key, value) = word.split_once('=').ok_or_else(|| format!("{word:?} is not key=value"))?;
    Ok(match key {
        "text" => CellCheck::Text(unescape_text(value)?),
        "width" => CellCheck::Width(value.parse().map_err(|_| format!("{value:?} is not a width"))?),
        "fg" => CellCheck::Fg(parse_color(value)?),
        "bg" => CellCheck::Bg(parse_color(value)?),
        "ul" => CellCheck::Underline(parse_color(value)?),
        "attrs" => {
            let mut attrs = Attrs::empty();
            if value != "none" {
                for name in value.split(',') {
                    let (_, attr) = Attrs::NAMES
                        .iter()
                        .find(|(n, _)| *n == name)
                        .ok_or_else(|| format!("{name:?} is not an attribute"))?;
                    attrs |= *attr;
                }
            }
            CellCheck::Attrs(attrs)
        }
        "link" => CellCheck::Link(if value == "none" { None } else { Some(unescape_text(value)?) }),
        other => {
            return Err(format!("{other:?} is not a cell property (text, width, fg, bg, ul, attrs, link)"));
        }
    })
}

fn parse_known_fail(rest: &str) -> Result<KnownFail, String> {
    // The first separator in the text splits it, whichever kind it is: a reason may contain a
    // later one ("alacritty – wraps early – see the manual").
    let (at, sep) = ["–", "—", " -- ", " - "]
        .iter()
        .filter_map(|sep| rest.find(sep).map(|at| (at, *sep)))
        .min_by_key(|(at, _)| *at)
        .ok_or("known-fail: <engine> – <reason>")?;
    let (engine, reason) = (&rest[..at], &rest[at + sep.len()..]);
    let (engine, reason) = (engine.trim(), reason.trim());
    if engine.is_empty() || reason.is_empty() {
        return Err("known-fail: <engine> – <reason> needs both".to_string());
    }
    Ok(KnownFail { engine: engine.to_string(), reason: reason.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<Case> {
        parse_file("t.vtt", text).unwrap_or_else(|e| panic!("{e:?}"))
    }

    fn errors(text: &str) -> Vec<String> {
        parse_file("t.vtt", text).expect_err("should not parse")
    }

    #[test]
    fn a_known_fail_splits_at_the_first_separator_of_any_kind() {
        let split = |text: &str| {
            let fail = parse_known_fail(text).unwrap_or_else(|e| panic!("{e}"));
            (fail.engine, fail.reason)
        };
        let want = |engine: &str, reason: &str| (engine.to_string(), reason.to_string());
        assert_eq!(split("alacritty – wrong"), want("alacritty", "wrong"));
        assert_eq!(split("alacritty -- wrong"), want("alacritty", "wrong"));
        assert_eq!(split("alacritty - wrong"), want("alacritty", "wrong"));
        // The reason has a separator of an earlier kind in the list than the one that splits.
        assert_eq!(split("alacritty - it wraps – early"), want("alacritty", "it wraps – early"));
        assert_eq!(split("alacritty – a - b -- c"), want("alacritty", "a - b -- c"));
        assert!(parse_known_fail("alacritty").is_err());
        assert!(parse_known_fail("alacritty – ").is_err());
    }

    #[test]
    fn a_case_with_every_directive() {
        let cases = parse(
            "# comment\n\
             test one\n\
             spec ctlseqs CUP\n\
             size 10 4\n\
             scrollback-limit 5\n\
             known-fail: alacritty – it is wrong\n\
             feed \\e[2;3Hab\n\
             repeat 3 c\n\
             screen\n\
             |  abccc   |\n\
             |          |\n\
             row 2 |  |\n\
             cursor 1,4 hidden\n\
             cursor *,2\n\
             cell 1,3 text=a fg=idx:1 bg=rgb:0a141e attrs=bold,italic link=none width=1\n\
             reply~ \\e[?6*c\n\
             title none\n\
             alt-screen off\n\
             wrapped 1 no\n\
             scrollback\n\
             |old|\n\
             scrollback-len 1\n\
             resize 5 2\n",
        );
        assert_eq!(cases.len(), 1);
        let case = &cases[0];
        assert_eq!((case.cols, case.rows, case.scrollback_limit), (10, 4, 5));
        assert_eq!(case.known_fail, [KnownFail { engine: "alacritty".into(), reason: "it is wrong".into() }]);
        assert_eq!(case.basis, Some(Basis::Spec("ctlseqs CUP".into())));
        assert_eq!(case.steps[0].kind, StepKind::Feed(b"\x1b[2;3Hab".to_vec()));
        assert_eq!(case.steps[1].kind, StepKind::Feed(b"ccc".to_vec()));
        assert_eq!(case.steps[2].kind, StepKind::Screen(vec!["  abccc".into(), "".into()]));
        assert_eq!(case.steps[2].line, 9);
        assert_eq!(case.steps[3].kind, StepKind::Row { row: 2, text: "".into() });
        assert_eq!(case.steps[4].kind, StepKind::Cursor { row: Some(1), col: 4, visible: Some(false) });
        assert_eq!(case.steps.len(), 14);
    }

    #[test]
    fn trailing_blanks_inside_pipes_are_not_significant_but_must_be_closed() {
        let cases = parse("test a\nsize 4 1\nfeed x\nscreen\n|x   |\n");
        assert_eq!(cases[0].steps[1].kind, StepKind::Screen(vec!["x".into()]));
        let e = errors("test a\nsize 4 1\nfeed x\nscreen\n|x\n");
        assert!(e[0].contains("t.vtt:5") && e[0].contains("end with"), "{e:?}");
    }

    #[test]
    fn problems_are_all_reported_with_their_line() {
        let e = errors(
            "feed x\n\
             test a\n\
             size 0 4\n\
             test b\n\
             size 4 4\n\
             bogus 1\n\
             cursor 0,1\n\
             test c\n\
             size 4 4\n\
             feed x\n",
        );
        assert!(e.iter().any(|m| m.starts_with("t.vtt:1:") && m.contains("outside of a test")), "{e:?}");
        assert!(e.iter().any(|m| m.starts_with("t.vtt:3:")), "{e:?}");
        assert!(e.iter().any(|m| m.starts_with("t.vtt:6:") && m.contains("unknown directive")), "{e:?}");
        assert!(e.iter().any(|m| m.starts_with("t.vtt:7:")), "{e:?}");
        assert!(e.iter().any(|m| m.starts_with("t.vtt:8:") && m.contains("checks nothing")), "{e:?}");
    }

    #[test]
    fn a_case_has_one_basis() {
        let cases = parse("test a\nconvention everyone does this\nsize 2 2\ncursor 1,1\n");
        assert_eq!(cases[0].basis, Some(Basis::Convention("everyone does this".into())));
        let e = errors("test a\nspec x\nconvention y\nsize 2 2\ncursor 1,1\n");
        assert!(e[0].contains("one `spec` or `convention`"), "{e:?}");
    }

    #[test]
    fn a_known_fail_needs_a_reason() {
        assert!(errors("test a\nsize 2 2\nknown-fail: alacritty\ncursor 1,1\n")[0].contains("known-fail"));
        let cases = parse("test a\nsize 2 2\nknown-fail: alacritty - no margins\ncursor 1,1\n");
        assert_eq!(cases[0].known_fail[0].reason, "no margins");
    }
}
