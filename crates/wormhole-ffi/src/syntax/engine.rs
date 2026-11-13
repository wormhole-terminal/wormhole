//! The parse and the colours of one text. A highlight query's captures are laid over the text in the order the
//! query finds them, a later one covering an earlier one; what is left of each is a span. A compiled query is kept
//! per language (compiling one takes about 25 ms); a parsed text is not kept: Swift asks once per file it shows.

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use tree_sitter::{
    Language, ParseOptions, ParseState, Parser, Query, QueryCursor, QueryCursorOptions, QueryCursorState,
    StreamingIterator,
};

use super::SyntaxRun;
use super::language::{self, Grammar};

/// The keys a capture can be drawn as: the names GPUI Kit's syntax theme knows, which is what the GPUI app colours
/// by. A capture named otherwise takes the key of its first segment (`keyword.modifier` is `keyword`), or none.
pub(super) const KEYS: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "comment.doc",
    "constant",
    "constructor",
    "embedded",
    "emphasis",
    "emphasis.strong",
    "enum",
    "function",
    "hint",
    "keyword",
    "label",
    "link_text",
    "link_uri",
    "number",
    "operator",
    "predictive",
    "preproc",
    "primary",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.list_marker",
    "punctuation.special",
    "string",
    "string.escape",
    "string.regex",
    "string.special",
    "string.special.symbol",
    "tag",
    "tag.doctype",
    "text.code.span",
    "text.literal",
    "title",
    "type",
    "variable",
    "variable.special",
    "variant",
];

/// The largest text that is parsed: what core keeps of a file as a diff's highlighting source (`git::MAX_TEXT_BYTES`).
const MAX_TEXT_BYTES: usize = 4 << 20;
/// How much of a line is coloured: what the diff and the viewer draw of it (`MAX_ROW_CHARS`).
const MAX_ROW_CHARS: usize = 5_000;
/// How long a parse and a query may each take. A file a person reads parses in a fraction of this; one made to be
/// slow is left plain.
const BUDGET: Duration = Duration::from_secs(2);
/// Matches a query may have in progress at once (tree-sitter's bound on a pattern that fans out).
const MATCH_LIMIT: u32 = 4096;

/// A language's grammar with its query compiled, and the key of each of the query's captures (0: none, else the
/// key's place in [`KEYS`] plus one).
struct Compiled {
    language: Language,
    query: Query,
    keys: Vec<u8>,
}

/// One compiled query per language, for the life of the process; `None` for one that did not compile.
static COMPILED: LazyLock<Mutex<HashMap<&'static str, Option<Arc<Compiled>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn compiled(grammar: Grammar) -> Option<Arc<Compiled>> {
    // A panic while the lock was held leaves at worst a language uncompiled, which is compiled again.
    let mut all = COMPILED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    all.entry(grammar.name).or_insert_with(|| compile(grammar).map(Arc::new)).clone()
}

fn compile(grammar: Grammar) -> Option<Compiled> {
    let whole = grammar.highlights.join("\n");
    let last = grammar.highlights.last().copied().unwrap_or_default();
    let query = Query::new(&grammar.language, &whole)
        .or_else(|_| Query::new(&grammar.language, last))
        .inspect_err(|err| tracing::warn!("syntax query for {}: {err}", grammar.name))
        .ok()?;
    let keys = query.capture_names().iter().map(|name| key_of(name)).collect();
    Some(Compiled { language: grammar.language, query, keys })
}

/// The key a capture named `name` is drawn as, as [`Compiled::keys`] counts it.
fn key_of(name: &str) -> u8 {
    let place = |name: &str| KEYS.iter().position(|key| *key == name);
    let found = place(name).or_else(|| place(name.split('.').next().unwrap_or_default()));
    found.and_then(|at| u8::try_from(at + 1).ok()).unwrap_or(0)
}

/// A text with its colours: for every byte the key it is drawn as (as [`Compiled::keys`] counts it).
pub(crate) struct Colored<'a> {
    text: &'a str,
    /// Where each line starts.
    starts: Vec<usize>,
    paint: Vec<u8>,
}

impl<'a> Colored<'a> {
    /// Parses `text` as the language `path` names. `None` for a file without a grammar (nothing is parsed), a text
    /// over the cap, or a parse or query that ran out of time.
    pub(crate) fn of(text: &'a str, path: &str) -> Option<Self> {
        let grammar = language::of(path)?;
        if text.len() > MAX_TEXT_BYTES {
            return None;
        }
        let compiled = compiled(grammar)?;
        let paint = paint(text, &compiled)?;
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        Some(Self { text, starts, paint })
    }

    /// How many lines the text has, cut at every line feed (a text that ends in one has an empty last line).
    pub(crate) fn lines(&self) -> usize {
        self.starts.len()
    }

    /// The spans of line `index` (0-based), without its line feed.
    pub(crate) fn line(&self, index: usize) -> Vec<SyntaxRun> {
        let Some(&start) = self.starts.get(index) else { return Vec::new() };
        let end = self.starts.get(index + 1).map_or(self.text.len(), |next| next - 1);
        self.text.get(start..end).map_or_else(Vec::new, |line| self.spans(start, line))
    }

    /// The spans of line `index` if the text has `expected` there; none if it has other text (a diff's line of a file
    /// that was edited since) or no such line.
    pub(crate) fn line_of(&self, index: usize, expected: &str) -> Vec<SyntaxRun> {
        let Some(&start) = self.starts.get(index) else { return Vec::new() };
        let Some(end) = start.checked_add(expected.len()) else { return Vec::new() };
        if self.text.get(start..end) != Some(expected) {
            return Vec::new();
        }
        self.spans(start, expected)
    }

    /// `line`, which starts at byte `start` of the text, as runs of one key, in scalars of the line.
    fn spans(&self, start: usize, line: &str) -> Vec<SyntaxRun> {
        let mut spans: Vec<SyntaxRun> = Vec::new();
        for (scalar, (byte, _)) in line.char_indices().take(MAX_ROW_CHARS).enumerate() {
            let key = self.paint.get(start + byte).copied().unwrap_or(0);
            let (Some(key), Ok(at)) = (key.checked_sub(1), u32::try_from(scalar)) else { continue };
            match spans.last_mut() {
                Some(last) if last.end == at && last.key == key => last.end = at + 1,
                _ => spans.push(SyntaxRun { start: at, end: at + 1, key }),
            }
        }
        spans
    }
}

/// The key of every byte of `text`; `None` when the parse or the query ran out of time.
fn paint(text: &str, compiled: &Compiled) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut parser = Parser::new();
    parser.set_language(&compiled.language).ok()?;

    let deadline = Instant::now() + BUDGET;
    let mut late = false;
    let mut progress = |_: &ParseState| -> ControlFlow<()> {
        if Instant::now() > deadline {
            late = true;
            return ControlFlow::Break(());
        }
        ControlFlow::Continue(())
    };
    let tree = parser.parse_with_options(
        &mut |at, _| bytes.get(at..).unwrap_or_default(),
        None,
        Some(ParseOptions::new().progress_callback(&mut progress)),
    )?;
    if late {
        return None;
    }

    let deadline = Instant::now() + BUDGET;
    let mut late = false;
    let mut paint = vec![0u8; bytes.len()];
    {
        let mut progress = |_: &QueryCursorState| -> ControlFlow<()> {
            if Instant::now() > deadline {
                late = true;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        };
        let mut cursor = QueryCursor::new();
        cursor.set_match_limit(MATCH_LIMIT);
        let mut matches = cursor.matches_with_options(
            &compiled.query,
            tree.root_node(),
            bytes,
            QueryCursorOptions::new().progress_callback(&mut progress),
        );
        // The node painted last: a capture of the very same node right after it keeps that one's key (GPUI Kit's
        // rule, so the first of two patterns that name one node decides).
        let mut last = (0, 0, 0u8);
        while let Some(found) = matches.next() {
            for capture in found.captures {
                let Some(&named) = compiled.keys.get(capture.index as usize) else { continue };
                let (start, end) = (capture.node.start_byte(), capture.node.end_byte().min(bytes.len()));
                let key = if (start, end) == (last.0, last.1) { last.2 } else { named };
                last = (start, end, key);
                // A capture without a key still covers what is under it: that stretch is plain.
                if let Some(covered) = paint.get_mut(start..end) {
                    covered.fill(key);
                }
            }
        }
    }
    (!late).then_some(paint)
}
