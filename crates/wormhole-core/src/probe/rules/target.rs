//! What an action would do for a match: the final link, the text to copy, the command and what it is
//! given. Only a ⌘-click needs it, so it is worked out apart from [`matches`](super::matches).

use std::borrow::Cow;

use super::compile::CompiledRules;
use super::matching::{Match, scanned};
use crate::probe::hidden::is_hidden;
use crate::probe::manifest::ActionKind;
use crate::probe::open_path::PathRef;
use crate::probe::validate::{capture_index, has_dot_segment};

/// What a rule's action comes to for one match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The link to open, with every capture percent-encoded where the template put it.
    OpenUrl(String),
    /// The text to copy.
    Copy(String),
    /// One of the Probe's commands, with the values of its `$n` arguments, in order.
    Command { id: String, args: Vec<String> },
    /// A file to open in the editor: the path as written, with its line and column. Not yet looked for on
    /// disk, which is [`open_path::resolve`](crate::probe::open_path::resolve)'s.
    OpenPath(PathRef),
}

impl Target {
    /// The target as one line: the link, the text, the command's id, or the path with its line and column.
    pub fn text(&self) -> Cow<'_, str> {
        match self {
            Self::OpenUrl(text) | Self::Copy(text) => Cow::Borrowed(text),
            Self::Command { id, .. } => Cow::Borrowed(id),
            Self::OpenPath(path) => Cow::Owned(path.text()),
        }
    }
}

/// The target of `rule`'s action for `found`, a match of `matches(rules, line)` on the same `line`.
/// `None` when the rule has no action, or when the action must not run: a capture that starts with
/// `-` would be read as an option by the program it is handed to, so the command is refused (the
/// pattern `validate` accepts cannot produce one, but a Probe that skipped `validate` can); and a link
/// that came out with a `.` or `..` segment in its path, because of what was captured, is refused; so is a
/// file path that is not one (`PathRef::parse`).
pub fn target(rules: &CompiledRules, found: &Match, line: &str) -> Option<Target> {
    let rule = rules.rules.get(found.rule)?;
    let action = rule.rule.action.as_ref()?;
    let text = scanned(line);
    // The captures of the match found at `start`: searching from there sees the same text around it, so
    // a word boundary means what it meant when the span was found.
    let caps = rule.regex.captures_at(text, found.start)?;
    let whole = caps.get(0)?;
    if whole.start() != found.start || whole.end() != found.end {
        return None;
    }
    let capture = |n: usize| caps.get(n).map_or("", |m| m.as_str());
    match action.kind {
        ActionKind::None => None,
        ActionKind::OpenUrl => {
            let url = action.url.as_deref()?;
            let url = expand(url, |n| percent_encode(capture(n)));
            // Encoding keeps a capture in its slot, but not a `.` or `..` that fills a whole segment:
            // browsers walk the path up there (`%2e` too), which would change where the link goes.
            (!has_dot_segment(&url)).then_some(Target::OpenUrl(url))
        }
        ActionKind::Copy => Some(Target::Copy(match &action.text {
            Some(template) => expand(template, |n| capture(n).to_string()),
            None => whole.as_str().to_string(),
        })),
        ActionKind::OpenPath => {
            let written = match &action.path {
                Some(template) => expand(template, |n| capture(n).to_string()),
                None => whole.as_str().to_string(),
            };
            PathRef::parse(&written).map(Target::OpenPath)
        }
        ActionKind::Command => {
            let id = action.command.clone()?;
            let args: Vec<String> = rule
                .command_run
                .iter()
                .flat_map(|run| run.iter().skip(1))
                .filter_map(|arg| capture_index(arg))
                .map(|n| capture(n).to_string())
                .collect();
            if args.iter().any(|arg| arg.starts_with('-')) {
                return None;
            }
            Some(Target::Command { id, args })
        }
    }
}

/// The longest hover text kept, in characters: the manifest's template is at most 120, and what its captures
/// add is cut here.
const HOVER_CHARS: usize = 160;

/// The text a rule says about `found` on ⌘-hover: its `hover` template with the match as `{0}` and the groups
/// as `{1}`…, written out (a capture is a program's text: control, bidi and invisible characters are dropped)
/// and cut to what a line can show. `None` for a rule without one.
pub fn hover(rules: &CompiledRules, found: &Match, line: &str) -> Option<String> {
    let rule = rules.rules.get(found.rule)?;
    let template = rule.rule.hover.as_deref()?;
    let caps = rule.regex.captures_at(scanned(line), found.start)?;
    if caps.get(0).is_none_or(|whole| whole.start() != found.start || whole.end() != found.end) {
        return None;
    }
    let text = expand(template, |n| {
        caps.get(n).map_or("", |m| m.as_str()).chars().filter(|c| !is_hidden(*c) && !c.is_control()).collect()
    });
    Some(text.chars().take(HOVER_CHARS).collect())
}

/// `template` with each `{n}` replaced by `value(n)`. A `{` that does not open a `{n}` stays as written
/// (`validate` refuses such a template; this only keeps a Probe that skipped it from panicking).
fn expand(template: &str, value: impl Fn(usize) -> String) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let digits = after.bytes().take_while(u8::is_ascii_digit).count();
        match after.as_bytes().get(digits) {
            Some(b'}') if digits > 0 && digits <= 2 => {
                out.push_str(&value(after[..digits].parse().unwrap_or(usize::MAX)));
                rest = &after[digits + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `text` for a place in a link: every byte but the unreserved characters of RFC 3986
/// (`A-Z a-z 0-9 - . _ ~`) as `%XX`, so a capture holding `?`, `#`, `/` or `../` stays text in the
/// slot it was written in and cannot change where the link goes.
pub(super) fn percent_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
