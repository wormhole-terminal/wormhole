//! What an action would do for a match: the final link, the text to copy, the command and what it is
//! given. Only a ⌘-click needs it, so it is worked out apart from [`matches`](super::matches).

use super::compile::CompiledRules;
use super::matching::{Match, scanned};
use crate::probe::manifest::ActionKind;
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
}

impl Target {
    /// The target as one line: the link, the text, or the command's id.
    pub fn text(&self) -> &str {
        match self {
            Self::OpenUrl(text) | Self::Copy(text) => text,
            Self::Command { id, .. } => id,
        }
    }
}

/// The target of `rule`'s action for `found`, a match of `matches(rules, line)` on the same `line`.
/// `None` when the rule has no action, or when the action must not run: a capture that starts with
/// `-` would be read as an option by the program it is handed to, so the command is refused (the
/// pattern `validate` accepts cannot produce one, but a Probe that skipped `validate` can); and a link
/// that came out with a `.` or `..` segment in its path, because of what was captured, is refused.
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
