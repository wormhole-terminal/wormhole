//! The `.vtt` conformance format: one test per terminal behaviour, written as the escape sequences
//! that go in and the screen, cursor, cell attributes and replies that must come out.
//!
//! The format is documented in `crates/wormhole-vt/tests/conformance/README.md`; this module
//! parses it ([`parse`]) and runs it against any [`Engine`](crate::Engine) ([`run`]).

mod escape;
mod parse;
mod run;

pub use escape::{glob_match, unescape_bytes, unescape_text};
pub use parse::{load_dir, parse_file};
pub use run::{CaseResult, Chunking, Report, run_case, run_suite, show_bytes};

use crate::engine::{Attrs, CellColor};

/// A `known-fail:` line: this engine gets the case wrong, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFail {
    pub engine: String,
    pub reason: String,
}

/// One test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub name: String,
    /// The file the case is in (as given to the parser) and the line of its `test` line.
    pub file: String,
    pub line: usize,
    /// What the expected values come from: a `spec` line names the published text, a `convention`
    /// line says the behaviour is universal but no published text spells it out. Documentation
    /// only, but every case has one.
    pub basis: Option<Basis>,
    pub cols: usize,
    pub rows: usize,
    /// Lines of history the engine keeps (`scrollback-limit`).
    pub scrollback_limit: usize,
    /// The engine is told to answer DECRQCRA (`screen-checksum allow`); it is off by default.
    pub allow_screen_checksum: bool,
    /// The engine is told to act on the window operations and answer the title reports
    /// (`window-ops allow`); they are off by default.
    pub allow_window_ops: bool,
    pub known_fail: Vec<KnownFail>,
    pub steps: Vec<Step>,
}

/// Where a case's expected values come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Basis {
    /// A published text (xterm's ctlseqs, a DEC manual, ECMA-48, a terminfo capability).
    Spec(String),
    /// Behaviour every terminal shares that the published texts leave unsaid; the line says what
    /// it is. Reported separately so the choice can be reviewed.
    Convention(String),
}

impl Basis {
    pub fn label(&self) -> String {
        match self {
            Basis::Spec(text) => format!("spec: {text}"),
            Basis::Convention(text) => format!("convention: {text}"),
        }
    }
}

/// A step runs in the order it is written: input and resizes change the terminal, everything else
/// checks what it looks like at that point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub line: usize,
    pub kind: StepKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Bytes for the terminal.
    Feed(Vec<u8>),
    Resize {
        cols: usize,
        rows: usize,
    },
    /// The whole visible screen: the listed rows from the top, every other row is blank.
    Screen(Vec<String>),
    /// One row, counted from 1.
    Row {
        row: usize,
        text: String,
    },
    /// Cursor position counted from 1 (`*` for the row when only the column matters), and
    /// optionally whether it is shown.
    Cursor {
        row: Option<usize>,
        col: usize,
        visible: Option<bool>,
    },
    Cell {
        row: usize,
        col: usize,
        checks: Vec<CellCheck>,
    },
    /// What the terminal sent back since the last `reply` (or the start).
    Reply(ReplyExpect),
    Title(Option<String>),
    AltScreen(bool),
    /// The history, oldest line first; the engine must have exactly these lines.
    Scrollback(Vec<String>),
    ScrollbackLen(usize),
    /// The text as logical lines: history and screen together, rows that continue by wrapping
    /// joined, trailing empty lines left out. What a resize must not change, wherever the
    /// screen begins.
    Lines(Vec<String>),
    /// Whether the row (from 1) continues in the next one because the text wrapped.
    Wrapped {
        row: usize,
        wrapped: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellCheck {
    Text(String),
    Width(u8),
    Fg(CellColor),
    Bg(CellColor),
    Underline(CellColor),
    /// The exact set of attributes.
    Attrs(Attrs),
    Link(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyExpect {
    Exact(Vec<u8>),
    /// `*` matches any run of bytes.
    Glob(Vec<u8>),
    Nothing,
}
