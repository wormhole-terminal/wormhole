//! What a hook's actions come to for one event: the notification's words, the fold's range, the
//! argument vector, the status text. Pure, so `probe test` can say it and the runtime can do it.
//!
//! Everything that ends up on a screen is cleaned (control, bidi and invisible characters never survive
//! a template) and cut to what its place can show. The fixed rules of a `run` live here: a capture is a
//! whole argument, never spliced and never through a shell, one that could be read as an option is
//! refused, and `git` gets its end-of-options marker before the first capture.

use crate::probe::events::{
    EventData, MAX_BODY_CHARS, MAX_HIDDEN_ROWS, MAX_PATH_BYTES, MAX_STATUS_CHARS, MAX_TITLE_CHARS,
    MAX_TOOLTIP_CHARS, MIN_HIDDEN_ROWS, Piece, RUN_TIMEOUT_DEFAULT, Value,
};
use crate::probe::hidden::is_hidden;
use crate::probe::manifest::{FoldTarget, HookAction, HookActionKind};
use crate::probe::validate::{MAX_ARGUMENT_BYTES, capture_index};

/// One thing a hook would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Planned {
    Notify {
        title: String,
        body: String,
    },
    Highlight {
        tone: String,
    },
    /// `hidden_rows` is known for a block's output (its lines less what is kept) and left to the terminal
    /// for a run of matching lines.
    Fold {
        target: FoldTarget,
        hidden_rows: Option<u64>,
        label: String,
    },
    OpenPath {
        path: String,
        line: Option<u32>,
    },
    Run {
        command: String,
        argv: Vec<String>,
        timeout_s: u64,
        item: Option<String>,
    },
    Status {
        item: String,
        text: String,
        tone: Option<String>,
        icon: Option<String>,
        tooltip: Option<String>,
    },
    /// An action a fixed rule refused for this event, and why.
    Dropped {
        action: &'static str,
        why: String,
    },
}

/// `text` without the characters that hide or draw over, newlines and tabs as spaces.
pub(super) fn clean(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\n' | '\r' | '\t' => Some(' '),
            c if is_hidden(c) => None,
            c => Some(c),
        })
        .collect()
}

/// `text` cut to `max` characters, with a `…` in the last place when it was cut.
pub(super) fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The templates of one action, parsed once.
#[derive(Debug, Clone, Default)]
pub(super) struct Pieces {
    pub title: Option<Vec<Piece>>,
    pub body: Option<Vec<Piece>>,
    pub text: Option<Vec<Piece>>,
    pub tooltip: Option<Vec<Piece>>,
    pub path: Option<Vec<Piece>>,
    pub line: Option<Vec<Piece>>,
}

/// An action ready to plan.
#[derive(Debug, Clone)]
pub(super) struct Step {
    pub action: HookAction,
    pub pieces: Pieces,
    /// A `run`'s command and its argument vector as the manifest has it, `$n` and all.
    pub run: Option<(String, Vec<String>)>,
}

/// What an event gives a template.
pub(super) struct Source<'a> {
    pub data: &'a EventData,
    /// The pattern's groups, the whole match first.
    pub captures: &'a [String],
}

impl Source<'_> {
    fn expand(&self, pieces: &[Piece], max: usize) -> String {
        let mut out = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Capture(n) => out.push_str(&clean(self.captures.get(*n).map_or("", String::as_str))),
                Piece::Field(name) => match name.as_str() {
                    "took_s" => out.push_str(&self.data.number("took_ms").map(seconds).unwrap_or_default()),
                    _ => match self.data.get(name) {
                        Some(Value::Number(n)) => out.push_str(&n.to_string()),
                        Some(Value::Text(text)) => out.push_str(&clean(text)),
                        None => {}
                    },
                },
            }
        }
        cut(&out, max)
    }
}

/// Milliseconds as seconds with one decimal at most: `61000` is `61`, `3500` is `3.5`.
fn seconds(ms: i64) -> String {
    let tenths = (ms.max(0) + 50) / 100;
    if tenths % 10 == 0 { (tenths / 10).to_string() } else { format!("{}.{}", tenths / 10, tenths % 10) }
}

pub(super) fn plan(step: &Step, source: &Source<'_>) -> Planned {
    let a = &step.action;
    let expand = |pieces: &Option<Vec<Piece>>, max: usize| pieces.as_deref().map(|p| source.expand(p, max));
    let dropped = |why: String| Planned::Dropped { action: a.kind.name(), why };
    match a.kind {
        HookActionKind::Notify => Planned::Notify {
            title: expand(&step.pieces.title, MAX_TITLE_CHARS).unwrap_or_default(),
            body: expand(&step.pieces.body, MAX_BODY_CHARS).unwrap_or_default(),
        },
        HookActionKind::Highlight => Planned::Highlight { tone: a.tone.clone().unwrap_or_default() },
        HookActionKind::Fold => fold(step, source.data).unwrap_or_else(dropped),
        HookActionKind::OpenPath => open_path(step, source).unwrap_or_else(dropped),
        HookActionKind::Run => run(step, source).unwrap_or_else(dropped),
        HookActionKind::Status => Planned::Status {
            item: a.item.clone().unwrap_or_default(),
            text: expand(&step.pieces.text, MAX_STATUS_CHARS).unwrap_or_default(),
            tone: a.tone.clone(),
            icon: a.icon.clone(),
            tooltip: expand(&step.pieces.tooltip, MAX_TOOLTIP_CHARS),
        },
    }
}

fn fold(step: &Step, data: &EventData) -> Result<Planned, String> {
    let a = &step.action;
    let target = a.target.ok_or("no target")?;
    let label = a.label.clone().unwrap_or_default();
    if target == FoldTarget::MatchRun {
        return Ok(Planned::Fold { target, hidden_rows: None, label });
    }
    let lines = data.number("output_lines").ok_or("the event has no `output_lines`")?.max(0) as u64;
    let kept = u64::from(a.keep_head.unwrap_or(0)) + u64::from(a.keep_tail.unwrap_or(0));
    if lines < u64::from(a.min_lines.unwrap_or(0)) {
        return Err(format!("{lines} lines are fewer than `min_lines`"));
    }
    let hidden = lines.saturating_sub(kept);
    if hidden < u64::from(MIN_HIDDEN_ROWS) {
        return Err(format!("{lines} lines less the {kept} kept hide fewer than {MIN_HIDDEN_ROWS} rows"));
    }
    if hidden > u64::from(MAX_HIDDEN_ROWS) {
        return Err(format!("{hidden} rows are more than one fold may hide ({MAX_HIDDEN_ROWS})"));
    }
    Ok(Planned::Fold { target, hidden_rows: Some(hidden), label })
}

fn open_path(step: &Step, source: &Source<'_>) -> Result<Planned, String> {
    let pieces = step.pieces.path.as_deref().ok_or("no path")?;
    let path = source.expand(pieces, MAX_PATH_BYTES);
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains("://") {
        return Err("the path is empty, too long, or a link".into());
    }
    // A capture of `..` would walk a path up out of where the template put it.
    if path.split('/').any(|segment| segment == "..") {
        return Err("the path has a `..` segment".into());
    }
    let line = match step.pieces.line.as_deref() {
        None => None,
        Some(pieces) => {
            let text = source.expand(pieces, 16);
            Some(
                text.parse::<u32>()
                    .ok()
                    .filter(|n| *n >= 1)
                    .ok_or_else(|| format!("`{text}` is not a line number"))?,
            )
        }
    };
    Ok(Planned::OpenPath { path, line })
}

fn run(step: &Step, source: &Source<'_>) -> Result<Planned, String> {
    let (command, template) = step.run.as_ref().ok_or("the command has no argument vector")?;
    let program = template.first().ok_or("the command has no program")?;
    let mut argv = vec![program.clone()];
    let mut marker = program.eq_ignore_ascii_case("git");
    for arg in &template[1..] {
        let Some(n) = capture_index(arg) else {
            argv.push(arg.clone());
            continue;
        };
        let value = source.captures.get(n).ok_or_else(|| format!("capture {n} is not there"))?;
        if value.is_empty()
            || value.starts_with('-')
            || value.len() > MAX_ARGUMENT_BYTES
            || value.contains('\0')
        {
            return Err(format!(
                "capture {n} {:?} is empty, starts with `-`, is over {MAX_ARGUMENT_BYTES} bytes or holds NUL: a program would read it as an option or not at all",
                cut(value, 40)
            ));
        }
        // `git` reads what follows this as a revision or path, never as an option.
        if std::mem::take(&mut marker) {
            argv.push("--end-of-options".into());
        }
        argv.push(value.clone());
    }
    Ok(Planned::Run {
        command: command.clone(),
        argv,
        timeout_s: step.action.timeout.unwrap_or(RUN_TIMEOUT_DEFAULT),
        item: step.action.item.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_have_one_decimal_at_most() {
        for (ms, text) in
            [(61000, "61"), (3500, "3.5"), (0, "0"), (999, "1"), (1049, "1"), (1050, "1.1"), (-5, "0")]
        {
            assert_eq!(seconds(ms), text, "{ms}");
        }
    }

    #[test]
    fn text_is_cleaned_and_cut() {
        assert_eq!(clean("a\u{1b}[31mb\nc\u{202e}d\te"), "a[31mb cd e");
        assert_eq!(cut("abc", 3), "abc");
        assert_eq!(cut("abcd", 3), "ab…");
        assert_eq!(cut("é", 1), "é");
    }
}
