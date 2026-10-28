//! Syntax colors for the diff's lines, taken from the full old / new file
//! (a hunk alone is too little context for tree-sitter).
//!
//! Parsing is the expensive part, so it is shared. Building a highlighter
//! compiles the language's query (about 25 ms for Rust, TypeScript or C++), and
//! parsing costs 50 ms per 10k lines; both used to be paid on the UI thread for
//! every diff, split toggle, wrap toggle and theme change. Now:
//!
//! - a parsed text is cached by the hash of the text and its language, so
//!   showing the same text again (the unchanged old side of a diff that keeps
//!   refreshing, a split / unified toggle, a new theme) re-uses the tree;
//! - a new text of a language that already has its share of cached texts takes
//!   over the highlighter of the least recently used one, so the compiled query
//!   is re-used: a language costs its query compile once per side of a diff
//!   (two), the first time it is shown, and nothing after that.
//!
//! The colors are not cached: they depend on the theme and cost little per
//! line, and only the lines the diff shows are asked for.

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

use gpui_kit::component::highlighter::{HighlightTheme, Language, SyntaxHighlighter};
use gpui_kit::component::input::Rope;
use gpui_kit::*;
use wormhole_core::git::{DiffLine, FileDiff, LineKind};

/// Texts kept per language: a diff has two sides.
const PER_LANGUAGE: usize = 2;
/// How much parsed text is kept in all. A parse tree is several times the size
/// of its text, so this bounds memory, not just the count.
const MAX_ENTRIES: usize = 32;
const MAX_TEXT_BYTES: usize = 8 << 20;

type Styles = Vec<(Range<usize>, HighlightStyle)>;

/// One parsed text: its tree-sitter state and where each line starts.
pub(crate) struct Parsed {
    // `styles` takes `&self`, but a tree-sitter parser is not `Sync`; the lock is
    // taken per line and never contended (one build runs at a time per text).
    highlighter: Mutex<SyntaxHighlighter>,
    line_starts: Vec<usize>,
    text: String,
}

impl Parsed {
    /// Styles for line `index`, relative to the line. Empty if the text here no
    /// longer matches what the diff holds for it (the file was edited meanwhile).
    fn line(&self, index: usize, expected: &str, theme: &HighlightTheme) -> Styles {
        let Some(&start) = self.line_starts.get(index) else { return Vec::new() };
        let end = start + expected.len();
        if self.text.get(start..end) != Some(expected) {
            return Vec::new();
        }
        let Ok(highlighter) = self.highlighter.lock() else { return Vec::new() };
        highlighter
            .styles(&(start..end), theme)
            .into_iter()
            .filter(|(r, _)| r.start < r.end)
            .map(|(r, style)| (r.start - start..r.end - start, style))
            .collect()
    }
}

/// Identifies a text: the hash and length stand in for the text itself.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Key {
    hash: u64,
    len: usize,
    language: &'static str,
}

impl Key {
    fn of(text: &str, language: &'static str) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Self { hash: hasher.finish(), len: text.len(), language }
    }
}

#[derive(Clone, Copy)]
struct Limits {
    per_language: usize,
    entries: usize,
    text_bytes: usize,
}

/// What the cache did, for tests (and a glance in a debugger).
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Stats {
    /// Highlighters built from scratch: one query compile each.
    pub compiles: usize,
    /// Texts parsed.
    pub parses: usize,
}

/// Parsed texts, least recently used first.
struct Cache {
    entries: VecDeque<(Key, Arc<Parsed>)>,
    bytes: usize,
    limits: Limits,
    stats: Stats,
}

impl Cache {
    fn new(limits: Limits) -> Self {
        Self { entries: VecDeque::new(), bytes: 0, limits, stats: Stats::default() }
    }

    /// The cached parse of `key`, now the most recently used.
    fn get(&mut self, key: &Key) -> Option<Arc<Parsed>> {
        let at = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(at)?;
        let parsed = entry.1.clone();
        self.entries.push_back(entry);
        Some(parsed)
    }

    /// The highlighter of the least recently used cached text of `language`,
    /// whose query is already compiled, if the language has its share of cached
    /// texts. The text is dropped from the cache; `None` means build a new one.
    fn take_over(&mut self, language: &'static str) -> Option<SyntaxHighlighter> {
        let of_language = |(key, _): &&(Key, Arc<Parsed>)| key.language == language;
        if self.entries.iter().filter(of_language).count() < self.limits.per_language {
            return None;
        }
        let at = self.entries.iter().position(|e| of_language(&e))?;
        let (key, parsed) = self.entries.remove(at)?;
        self.bytes -= key.len;
        // Only if nothing else still reads it (a diff being built from it).
        Arc::try_unwrap(parsed).ok()?.highlighter.into_inner().ok()
    }

    /// Keep `parsed` for next time, evicting the least recently used to stay in
    /// bounds. A text larger than the whole budget is not kept.
    fn insert(&mut self, key: Key, parsed: Arc<Parsed>) -> Arc<Parsed> {
        if let Some(existing) = self.get(&key) {
            // Two builds parsed the same text at once; keep the first.
            return existing;
        }
        if key.len > self.limits.text_bytes {
            return parsed;
        }
        self.bytes += key.len;
        self.entries.push_back((key, parsed.clone()));
        while self.entries.len() > self.limits.entries || self.bytes > self.limits.text_bytes {
            let Some((old, _)) = self.entries.pop_front() else { break };
            self.bytes -= old.len;
        }
        parsed
    }
}

static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| {
    Mutex::new(Cache::new(Limits {
        per_language: PER_LANGUAGE,
        entries: MAX_ENTRIES,
        text_bytes: MAX_TEXT_BYTES,
    }))
});

fn lock(cache: &Mutex<Cache>) -> MutexGuard<'_, Cache> {
    // A panic while the lock was held leaves nothing half-updated that matters:
    // at worst an entry is missing and gets parsed again.
    cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `text` parsed as `language`, from the cache when it was parsed before.
fn parse_in(cache: &Mutex<Cache>, text: &str, language: &'static str) -> Arc<Parsed> {
    let key = Key::of(text, language);
    let reusable = {
        let mut cache = lock(cache);
        if let Some(hit) = cache.get(&key) {
            return hit;
        }
        cache.stats.parses += 1;
        cache.take_over(language)
    };
    // Compiling a query and parsing a big file take long; other builds must not wait for the lock meanwhile.
    let mut highlighter = reusable.unwrap_or_else(|| {
        lock(cache).stats.compiles += 1;
        SyntaxHighlighter::new(language)
    });
    highlighter.update(None, &Rope::from(text), None);
    let mut line_starts = vec![0];
    line_starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
    let parsed =
        Arc::new(Parsed { highlighter: Mutex::new(highlighter), line_starts, text: text.to_string() });
    lock(cache).insert(key, parsed)
}

/// The language a file's name says it is, if there is a grammar for it.
fn language_of(path: &str) -> Option<&'static str> {
    let extension = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let language = Language::from_str(extension).name();
    (language != Language::Plain.name()).then_some(language)
}

/// The syntax colors of one text shown whole, a file in the Files viewer.
pub(crate) struct TextHighlights {
    parsed: Arc<Parsed>,
}

impl TextHighlights {
    /// Parses `text` as the language `path` names; `None` for a file without a grammar. Takes long for a
    /// large text: not for the main thread.
    pub(crate) fn for_text(text: &str, path: &str) -> Option<Self> {
        Some(Self { parsed: parse_in(&CACHE, text, language_of(path)?) })
    }

    /// Colors for line `index` (0-based), whose text is `line` without its line break, relative to the
    /// line. Empty if the parsed text has other text there.
    pub(crate) fn line(&self, index: usize, line: &str, theme: &HighlightTheme) -> Styles {
        self.parsed.line(index, line, theme)
    }
}

/// The syntax colors of a diff's two files.
pub(crate) struct Highlights {
    old: Option<Arc<Parsed>>,
    new: Option<Arc<Parsed>>,
    theme: Arc<HighlightTheme>,
}

impl Highlights {
    /// `None` for a file without a grammar (plain text, unknown extensions):
    /// its lines get no colors and nothing is parsed.
    pub(crate) fn for_diff(diff: &FileDiff, path: &str, theme: &Arc<HighlightTheme>) -> Option<Self> {
        Self::for_diff_in(&CACHE, diff, path, theme)
    }

    fn for_diff_in(
        cache: &Mutex<Cache>,
        diff: &FileDiff,
        path: &str,
        theme: &Arc<HighlightTheme>,
    ) -> Option<Self> {
        let language = language_of(path)?;
        let parse = |text: &String| parse_in(cache, text, language);
        Some(Self {
            old: diff.old_text.as_ref().map(parse),
            new: diff.new_text.as_ref().map(parse),
            theme: theme.clone(),
        })
    }

    /// Colors for one diff line, relative to its text: a removed line from the
    /// old file, any other from the new one (the old one when the new is missing).
    pub(crate) fn line(&self, line: &DiffLine) -> Styles {
        let source = match line.kind {
            LineKind::Removed => self.old.as_ref().zip(line.old),
            _ => self.new.as_ref().zip(line.new).or(self.old.as_ref().zip(line.old)),
        };
        let Some((parsed, number)) = source else { return Vec::new() };
        let Some(index) = (number as usize).checked_sub(1) else { return Vec::new() };
        parsed.line(index, &line.text, &self.theme)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{Cache, Highlights, Limits, Stats, TextHighlights, lock, parse_in};
    use gpui_kit::component::highlighter::HighlightTheme;
    use wormhole_core::git::{DiffLine, FileDiff, LineKind};

    fn cache(per_language: usize, entries: usize, text_bytes: usize) -> Mutex<Cache> {
        Mutex::new(Cache::new(Limits { per_language, entries, text_bytes }))
    }

    /// Room for plenty of texts: only what a test is about limits it.
    fn roomy(per_language: usize) -> Mutex<Cache> {
        cache(per_language, 64, 1 << 20)
    }

    fn stats(cache: &Mutex<Cache>) -> Stats {
        lock(cache).stats
    }

    /// How many texts are kept, and their bytes (one lock: the mutex is not reentrant).
    fn shape(cache: &Mutex<Cache>) -> (usize, usize) {
        let cache = lock(cache);
        (cache.entries.len(), cache.bytes)
    }

    fn removed(old: u32, text: &str) -> DiffLine {
        DiffLine { kind: LineKind::Removed, old: Some(old), new: None, text: text.into(), no_newline: false }
    }

    const SOURCE: &str = "fn main() {\n    let x = 1;\n}\n";

    #[test]
    fn the_same_text_is_parsed_once() {
        let cache = roomy(2);
        let first = parse_in(&cache, SOURCE, "rust");
        let again = parse_in(&cache, SOURCE, "rust");
        assert!(Arc::ptr_eq(&first, &again));
        assert_eq!(stats(&cache), Stats { compiles: 1, parses: 1 });
        // Another language is another text, even with equal bytes.
        let other = parse_in(&cache, SOURCE, "javascript");
        assert!(!Arc::ptr_eq(&first, &other));
    }

    #[test]
    fn a_language_is_compiled_for_its_first_diff_and_never_again() {
        // A diff has two sides, so two texts of a language are cached; the next
        // diff's texts take over their highlighters instead of compiling new ones.
        let cache = roomy(2);
        parse_in(&cache, "fn old() {}\n", "rust");
        parse_in(&cache, "fn new() {}\n", "rust");
        assert_eq!(stats(&cache), Stats { compiles: 2, parses: 2 });
        for n in 0..5 {
            parse_in(&cache, &format!("fn old{n}() {{}}\n"), "rust");
            parse_in(&cache, &format!("fn new{n}() {{}}\n"), "rust");
        }
        assert_eq!(stats(&cache), Stats { compiles: 2, parses: 12 });
        // Another language has its own share.
        parse_in(&cache, "const a = 1;\n", "javascript");
        assert_eq!(stats(&cache).compiles, 3);
    }

    #[test]
    fn a_highlighter_that_took_over_colors_like_a_fresh_one() {
        let theme = HighlightTheme::default_dark();
        let (a, b) = ("fn a() { let aaaa = 1; }\nstruct S;\n", "const X: u8 = 2;\n");
        let fresh = parse_in(&roomy(1), b, "rust");
        // `a` is cached; with room for one text per language, `b` takes over its highlighter.
        let used = roomy(1);
        parse_in(&used, a, "rust");
        let reused = parse_in(&used, b, "rust");
        assert_eq!(stats(&used), Stats { compiles: 1, parses: 2 });
        assert_eq!(shape(&used), (1, b.len()));
        let colors = |parsed: &super::Parsed| parsed.line(0, "const X: u8 = 2;", &theme);
        assert!(colors(&fresh).iter().any(|(_, style)| style.color.is_some()));
        assert_eq!(colors(&fresh), colors(&reused));
    }

    #[test]
    fn a_text_still_being_read_is_not_taken_apart() {
        let cache = roomy(1);
        let in_use = parse_in(&cache, "fn a() {}\n", "rust");
        // `b` needs a highlighter; `a`'s is still held by a build, so it compiles its own.
        parse_in(&cache, "fn b() {}\n", "rust");
        assert_eq!(stats(&cache).compiles, 2);
        assert!(!in_use.line(0, "fn a() {}", &HighlightTheme::default_dark()).is_empty());
    }

    #[test]
    fn the_cache_is_bounded_by_entries_and_bytes() {
        let (a, b, c, big) = ("fn a() {}\n", "fn b() {}\n", "fn c() {}\n", "fn big() { 1 }\n");
        assert_eq!((a.len(), big.len()), (10, 15));
        let cache = cache(8, 2, 25);
        for text in [a, b, c] {
            parse_in(&cache, text, "rust");
        }
        // Two entries at most, whatever their size.
        assert_eq!(shape(&cache), (2, 20));
        // 20 + 15 bytes is over the budget of 25: the oldest goes.
        parse_in(&cache, big, "rust");
        assert_eq!(shape(&cache), (2, 25));
        // A text larger than the whole budget is parsed but not kept.
        parse_in(&cache, &"// 0123456789\n".repeat(2), "rust");
        assert_eq!(shape(&cache), (2, 25));
    }

    #[test]
    fn the_least_recently_used_text_goes_first() {
        let cache = cache(8, 2, 1 << 20);
        let a = parse_in(&cache, "fn a() {}\n", "rust");
        parse_in(&cache, "fn b() {}\n", "rust");
        // Using `a` again makes `b` the oldest.
        assert!(Arc::ptr_eq(&a, &parse_in(&cache, "fn a() {}\n", "rust")));
        parse_in(&cache, "fn c() {}\n", "rust");
        assert!(Arc::ptr_eq(&a, &parse_in(&cache, "fn a() {}\n", "rust")));
        assert_eq!(stats(&cache).parses, 3);
    }

    #[test]
    fn plain_text_is_not_parsed_at_all() {
        let cache = roomy(2);
        let theme = HighlightTheme::default_dark();
        let diff =
            FileDiff { old_text: Some("a\n".into()), new_text: Some("b\n".into()), ..FileDiff::default() };
        assert!(Highlights::for_diff_in(&cache, &diff, "notes.unknownext", &theme).is_none());
        assert!(Highlights::for_diff_in(&cache, &diff, "Makefile", &theme).is_none());
        assert_eq!(stats(&cache), Stats::default());
    }

    #[test]
    fn lines_get_colors_from_their_own_side_and_only_if_the_text_matches() {
        let cache = roomy(2);
        let theme = HighlightTheme::default_dark();
        let diff = FileDiff {
            old_text: Some("fn old() {}\n".into()),
            new_text: Some("fn new() {}\n".into()),
            ..FileDiff::default()
        };
        let highlights = Highlights::for_diff_in(&cache, &diff, "a.rs", &theme).expect("rust has a grammar");
        // A removed line is read from the old file, at its old line number.
        assert!(!highlights.line(&removed(1, "fn old() {}")).is_empty());
        // The line the diff holds is not what the file has there: no colors, no panic.
        assert!(highlights.line(&removed(1, "fn other() {}")).is_empty());
        assert!(highlights.line(&removed(9, "fn old() {}")).is_empty());
        // Line numbers count from 1; 0 must not underflow.
        assert!(highlights.line(&removed(0, "fn old() {}")).is_empty());
    }

    #[test]
    fn a_whole_text_is_colored_line_by_line_and_only_if_it_has_a_grammar() {
        let theme = HighlightTheme::default_dark();
        let text = "fn whole_text_test() {\n    let x = 1;\n}\n";
        let colors = TextHighlights::for_text(text, "src/a.rs").expect("rust has a grammar");
        assert!(colors.line(1, "    let x = 1;", &theme).iter().any(|(_, style)| style.color.is_some()));
        // Text that is not what was parsed gets nothing.
        assert!(colors.line(1, "    let y = 1;", &theme).is_empty());
        assert!(colors.line(9, "    let x = 1;", &theme).is_empty());
        assert!(TextHighlights::for_text(text, "notes.txt").is_none());
        assert!(TextHighlights::for_text(text, "Makefile").is_none());
    }
}
