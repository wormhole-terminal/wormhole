//! Checking a value as it is typed into the Theme Studio: whether it parses, and whether
//! every name in it is a color that exists, with where it goes wrong.

use std::fmt;
use std::ops::Range;

use super::catalog::{UI_TOKENS, syntax_keys};
use super::expr::{Tok, parse_located, tokenize};
use crate::storage::jsonc::suggest::closest;

/// The terminal colors an expression can name, without their `terminal.` prefix.
const TERMINAL_KEYS: [&str; 6] =
    ["foreground", "background", "cursor", "cursor_text", "selection_background", "selection_foreground"];

/// An expression the Studio can't use, and where in it the problem is.
#[derive(Clone, Debug, PartialEq)]
pub struct ExprError {
    /// What is wrong, without the position: "Unknown token “primery”".
    pub message: String,
    /// 1-based line of the problem.
    pub line: usize,
    /// 1-based column of the problem, in characters.
    pub column: usize,
    /// The bytes of the source the problem is (the unknown name, the token that doesn't
    /// belong); empty, at the end, for an expression that stops too early.
    pub span: Range<usize>,
    /// A name that exists and is close to an unknown one.
    pub suggestion: Option<String>,
}

impl ExprError {
    fn new(src: &str, message: String, span: Range<usize>, suggestion: Option<String>) -> Self {
        let before = src.get(..span.start).unwrap_or(src);
        let line = 1 + before.matches('\n').count();
        let column = 1 + before.rsplit('\n').next().unwrap_or(before).chars().count();
        Self { message, line, column, span, suggestion }
    }

    /// `src` with the unknown name replaced by the suggestion.
    pub fn with_suggestion(&self, src: &str) -> Option<String> {
        let suggestion = self.suggestion.as_deref()?;
        Some(format!("{}{suggestion}{}", src.get(..self.span.start)?, src.get(self.span.end..)?))
    }
}

/// "Unknown token “primery” at 1:1."
impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}.", self.message, self.line, self.column)
    }
}

impl std::error::Error for ExprError {}

/// Every color an expression may name: the UI tokens, the terminal's colors and the
/// built-in syntax colors.
fn known_names() -> impl Iterator<Item = String> {
    UI_TOKENS
        .iter()
        .map(|t| t.name.to_string())
        .chain(terminal_names())
        .chain(syntax_keys().map(|k| format!("syntax.{k}")))
}

/// Whether `name` is a color this version has (the reader of a newer file drops a value that
/// refers to one it lacks).
pub(super) fn is_known_node(name: &str) -> bool {
    known_names().any(|k| k == name)
}

fn terminal_names() -> impl Iterator<Item = String> {
    TERMINAL_KEYS.iter().map(|k| format!("terminal.{k}")).chain((0..16).map(|i| format!("terminal.ansi.{i}")))
}

/// Checks a value typed for `target` (a UI token, `terminal.*` or `syntax.*`): that it
/// parses and that every name in it is a color that exists. A terminal color may only
/// refer to other terminal colors (the interface is derived from them), and nothing may
/// refer to itself. Longer cycles are found when the theme is resolved.
pub fn check_expression(src: &str, target: &str) -> Result<(), ExprError> {
    parse_located(src).map_err(|e| ExprError::new(src, capitalized(&e.message), e.span, None))?;
    let tokens = tokenize(src).map_err(|e| ExprError::new(src, capitalized(&e.message), e.span, None))?;
    let terminal_only = target.starts_with("terminal.");
    let known: Vec<String> = if terminal_only { terminal_names().collect() } else { known_names().collect() };
    for (i, (tok, span)) in tokens.iter().enumerate() {
        let Tok::Ident(name) = tok else { continue };
        // A name followed by `(` is a function, which the parser has checked.
        if matches!(tokens.get(i + 1), Some((Tok::LParen, _))) {
            continue;
        }
        if name == target {
            return Err(ExprError::new(src, format!("“{name}” refers to itself"), span.clone(), None));
        }
        if known.iter().any(|k| k == name) {
            continue;
        }
        if terminal_only && known_names().any(|k| k == *name) {
            let message = format!("A terminal color may only refer to terminal.* colors, not “{name}”");
            return Err(ExprError::new(src, message, span.clone(), None));
        }
        let suggestion = closest(name, known.iter().map(String::as_str)).map(str::to_string);
        return Err(ExprError::new(src, format!("Unknown token “{name}”"), span.clone(), suggestion));
    }
    Ok(())
}

/// The parser's lowercase message as a sentence's start.
fn capitalized(message: &str) -> String {
    let mut chars = message.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// The functions an expression can call, each as the completion list shows it: what it takes.
pub const FUNCTIONS: [(&str, &str); 8] = [
    ("mix", "mix(a, b, 30%)"),
    ("readable", "readable(a, bg, 4.5)"),
    ("over", "over(a, bg)"),
    ("lighten", "lighten(a, 10%)"),
    ("darken", "darken(a, 10%)"),
    ("alpha", "alpha(a, 50%)"),
    ("shade", "shade(a, 8%)"),
    ("text_on", "text_on(bg)"),
];

/// How many names the completion list offers at once.
pub const COMPLETIONS_MAX: usize = 8;

/// One name the expression field offers while a name is typed: a color, or a function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    /// What the list shows: the name, or the function with what it takes (`mix(a, b, 30%)`).
    pub label: String,
    /// What takes the place of the name being typed: the name, or the function and its `(`
    /// (the arguments are the person's to write).
    pub insert: String,
    pub function: bool,
}

/// What the field offers at the caret: the names that fit what is typed before it, best first,
/// and the bytes of the source the one taken replaces (the whole name the caret is in).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completions {
    pub range: Range<usize>,
    pub items: Vec<Completion>,
}

/// The names to offer for a value typed for `target` with the caret at byte `caret`: the colors
/// it may name (as [`check_expression`] allows them, never itself) and the functions, matched
/// against the name before the caret: a name that starts with it first, then one with a part
/// (after a `.` or `_`) that does, then one that contains it. Nothing while the caret isn't
/// after a name, or once that name is a color that exists, so Return applies a finished value
/// instead of taking a longer name from the list (`primary.` still offers `primary.hover`).
pub fn completions(src: &str, caret: usize, target: &str) -> Option<Completions> {
    let caret = caret.min(src.len());
    let is_name_char = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    let before = src.get(..caret)?;
    let start = before.char_indices().rev().take_while(|(_, c)| is_name_char(*c)).last().map(|(i, _)| i)?;
    let typed = &src[start..caret];
    // A name starts with a letter; after `#` it is a hex color, and a digit starts a number.
    if !typed.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') || before[..start].ends_with('#') {
        return None;
    }
    let end = caret + src[caret..].find(|c: char| !is_name_char(c)).unwrap_or(src.len() - caret);
    let names: Vec<String> =
        if target.starts_with("terminal.") { terminal_names().collect() } else { known_names().collect() };
    // A color that is complete: Return should apply it, not take a longer name from a list.
    if names.iter().any(|n| n == typed) {
        return None;
    }
    let query = typed.to_ascii_lowercase();
    let rank = |name: &str| -> Option<u8> {
        let name = name.to_ascii_lowercase();
        if name.starts_with(&query) {
            Some(0)
        } else if name.split(['.', '_']).skip(1).any(|part| part.starts_with(&query)) {
            Some(1)
        } else if query.len() > 1 && name.contains(&query) {
            Some(2)
        } else {
            None
        }
    };
    let functions = FUNCTIONS.iter().filter_map(|(name, label)| {
        let item = Completion { label: label.to_string(), insert: format!("{name}("), function: true };
        rank(name).map(|r| (r, item))
    });
    let colors = names.iter().filter(|name| name.as_str() != target).filter_map(|name| {
        rank(name).map(|r| (r, Completion { label: name.clone(), insert: name.clone(), function: false }))
    });
    let mut ranked: Vec<(u8, Completion)> = functions.chain(colors).collect();
    if ranked.is_empty() {
        return None;
    }
    // Stable: within a rank, the functions and then the catalogue's order.
    ranked.sort_by_key(|(r, _)| *r);
    let items = ranked.into_iter().map(|(_, item)| item).take(COMPLETIONS_MAX).collect();
    Some(Completions { range: start..end, items })
}

#[cfg(test)]
mod tests {
    use super::{
        COMPLETIONS_MAX, Completions, ExprError, FUNCTIONS, check_expression, completions, parse_located,
    };

    fn error(src: &str, target: &str) -> ExprError {
        check_expression(src, target).unwrap_err()
    }

    #[test]
    fn an_unknown_name_is_pointed_at_with_a_suggestion() {
        let e = error("primery / 20%", "state.highlight");
        assert_eq!(e.to_string(), "Unknown token “primery” at 1:1.");
        assert_eq!((e.line, e.column, e.span.clone()), (1, 1, 0..7));
        assert_eq!(e.suggestion.as_deref(), Some("primary"));
        assert_eq!(e.with_suggestion("primery / 20%").as_deref(), Some("primary / 20%"));

        let e = error("mix(sidebar.background, primry, 14%)", "sidebar.row.selected");
        assert_eq!((e.column, e.suggestion.as_deref()), (25, Some("primary")));
        // Nothing close enough: no guess.
        assert_eq!(error("zzzzzzzzzz", "border").suggestion, None);
        assert_eq!(error("zzzzzzzzzz", "border").with_suggestion("zzzzzzzzzz"), None);
    }

    #[test]
    fn parse_errors_say_where() {
        let e = error("mix(a, b)", "border");
        assert_eq!(e.to_string(), "Expected `,` in mix(), found `)` at 1:9.");
        let e = error("primary $ 2", "border");
        assert_eq!((e.column, e.message.as_str()), (9, "Unexpected character '$'"));
        // An expression that stops early points just past its end.
        let e = error("mix(primary, border, 10%", "border.strong");
        assert_eq!((e.column, e.span.clone()), (25, 24..24));
        assert!(e.message.contains("closing mix()"), "{}", e.message);
        let e = error("", "border");
        assert_eq!((e.line, e.column), (1, 1));
    }

    #[test]
    fn lines_and_columns_count_characters() {
        let e = error("mix(primary,\n  bordr, 10%)", "border.strong");
        assert_eq!((e.line, e.column), (2, 3));
        assert_eq!(e.suggestion.as_deref(), Some("border"));
        // A wide character before the problem counts as one column.
        let e = error("mix(é", "border");
        assert_eq!((e.line, e.column), (1, 5));
    }

    #[test]
    fn names_must_exist_in_the_targets_scope() {
        assert_eq!(check_expression("primary / 20%", "state.highlight"), Ok(()));
        assert_eq!(check_expression("mix(terminal.ansi.4, syntax.keyword, 50%)", "border"), Ok(()));
        assert_eq!(check_expression("readable(#7aa2f7, surface.window, 3)", "primary"), Ok(()));
        assert_eq!(check_expression("lighten(terminal.ansi.0, 10%)", "terminal.background"), Ok(()));

        let e = error("mix(terminal.background, primary, 10%)", "terminal.selection_background");
        assert!(e.message.contains("terminal.* colors, not “primary”"), "{}", e.message);
        assert_eq!(e.column, 26);
        let e = error("terminal.backgrund", "terminal.cursor");
        assert_eq!(e.suggestion.as_deref(), Some("terminal.background"));

        let e = error("lighten(border, 5%)", "border");
        assert_eq!(e.to_string(), "“border” refers to itself at 1:9.");
        assert!(check_expression("syntax.nope", "border").is_err());
    }

    /// The labels offered with the caret at the end of `src`.
    fn offered(src: &str, target: &str) -> Vec<String> {
        completions(src, src.len(), target)
            .map(|c| c.items.into_iter().map(|i| i.label).collect())
            .unwrap_or_default()
    }

    #[test]
    fn a_name_being_typed_is_completed() {
        let c = completions("prim", 4, "state.highlight").expect("offers names");
        assert_eq!(c.range, 0..4);
        let labels: Vec<&str> = c.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels[..2], ["primary", "primary.hover"]);
        assert_eq!(c.items[0].insert, "primary");

        // Inside an expression, only the name at the caret is replaced: the whole of it.
        let src = "mix(sidebar.background, prim, 14%)";
        let c = completions(src, 28, "sidebar.row.selected").expect("offers names");
        assert_eq!(&src[c.range.clone()], "prim");
        let c = completions(src, 26, "sidebar.row.selected").expect("offers names");
        assert_eq!(c.range, 24..28, "the caret inside the name");

        // A part of a name after a dot counts, after what starts with it.
        let labels = offered("sel", "border");
        assert_eq!(labels.first().map(String::as_str), Some("selection.ui"));
        assert!(labels.iter().any(|l| l == "sidebar.row.selected"), "{labels:?}");
        assert!(labels.len() <= COMPLETIONS_MAX);
    }

    #[test]
    fn functions_are_offered_with_what_they_take() {
        let c = completions("mi", 2, "border").expect("offers mix");
        assert_eq!(c.items[0].label, "mix(a, b, 30%)");
        assert_eq!(c.items[0].insert, "mix(");
        assert!(c.items[0].function);
        assert_eq!(offered("rea", "border").first().map(String::as_str), Some("readable(a, bg, 4.5)"));
        assert_eq!(offered("ov", "border").first().map(String::as_str), Some("over(a, bg)"));
        // Every one is a function the parser knows, written as it takes its arguments.
        for (name, form) in FUNCTIONS {
            let args = form.strip_prefix(&format!("{name}(")).and_then(|r| r.strip_suffix(')')).expect(form);
            let args: Vec<&str> =
                args.split(", ").map(|a| if matches!(a, "a" | "b" | "bg") { "#808080" } else { a }).collect();
            let sample = format!("{name}({})", args.join(", "));
            assert!(parse_located(&sample).is_ok(), "{sample}");
        }
        assert!(parse_located("blend(#000, #fff)").is_err(), "a function it doesn't know");
    }

    #[test]
    fn nothing_is_offered_where_no_name_is_typed() {
        assert_eq!(completions("", 0, "border"), None);
        assert_eq!(completions("#7aa2f7", 7, "border"), None, "a hex color");
        assert_eq!(completions("#abc", 4, "border"), None, "a hex color of letters");
        assert_eq!(completions("primary / 2", 11, "border"), None, "a number");
        assert_eq!(completions("mix(", 4, "border"), None, "after a parenthesis");
        assert_eq!(completions("zzzz", 4, "border"), None, "nothing like it");
        // A color that exists is finished: Return applies it.
        assert_eq!(completions("primary", 7, "border"), None);
        // Going on from it offers what is longer.
        assert_eq!(offered("primary.", "border"), ["primary.hover"]);
    }

    #[test]
    fn completions_keep_to_the_targets_scope() {
        // A terminal color names terminal colors only, and nothing offers the token itself.
        let labels = offered("terminal.back", "terminal.cursor");
        assert_eq!(labels, ["terminal.background"]);
        assert!(offered("prim", "terminal.cursor").is_empty(), "primary isn't a terminal color");
        assert!(!offered("bord", "border").contains(&"border".to_string()));
        assert!(matches!(completions("bord", 4, "border"), Some(Completions { .. })));
    }
}
