//! `[[contributes.hooks]]`: an event, what must hold, and what to do
//! (`repo-docs/docs/research/probes-hooks-and-fold.md`, sections 3 and 6).
//!
//! A hook may only name what its event carries, only ask for what the Probe has declared, and never
//! decide anything: its actions notify, tone, fold, offer a link, run one of the Probe's own background
//! commands, or set a status item. The rules, one `code` each:
//!
//! - the event's closed matcher fields, and each value's own shape (a number test, a list, a glob, a
//!   pattern with a rule's limits); a pattern is required where the event is a line of output;
//! - a template names only fields of its event; text a program or a repository chose (a [`Text`] field,
//!   a capture) leaves a hook only with `terminal = ["read"]`;
//! - `notify` needs `notify`, `fold` needs `fold` (Touch ID), `run` needs its command's program in
//!   `process` and the command in the background, and a capture that reaches a program is anchored
//!   like a rule's.
//!
//! [`Text`]: crate::probe::events::FieldKind::Text

use super::Findings;
use super::argv;
use super::pattern::{self, Pattern};
use super::rules::{TONES, check_command_target};
use crate::probe::events::{
    self, Comparison, EventSpec, FieldKind, Glob, MAX_ACTIONS, MAX_HIDDEN_ROWS, MAX_KEEP, MAX_LABEL_CHARS,
    MAX_PATH_BYTES, MIN_HIDDEN_ROWS, MatcherKind, Piece, RUN_TIMEOUT_MAX, STATES, list, parse_template,
};
use crate::probe::hidden::{has_hidden, is_hidden};
use crate::probe::manifest::{
    FoldTarget, Hook, HookAction, HookActionKind, HookEvent, RunIn, TerminalAccess,
};
use crate::probe::read::ProbeFolder;

/// The icon names a status action may give: a plain name, resolved when the status bar draws it.
fn is_icon_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// A name in a list of agents: lower-case, digits, `-`, `_` and `/` (a Probe's agent is `probe/agent`).
fn is_agent_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'/'))
}

/// What the hooks of a Probe use, which the Probe's permissions are then held to.
#[derive(Default)]
struct Uses {
    /// Some hook sends text the terminal showed somewhere: a text field or a capture in a template, or a
    /// capture handed to a program.
    reads_text: bool,
    folds: bool,
}

/// One hook, as the checks see it.
struct Cx<'a> {
    folder: &'a ProbeFolder,
    spec: &'static EventSpec,
    at: String,
    /// The hook's pattern, when it has one that analyzed.
    pattern: Option<Pattern>,
    can_read: bool,
}

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    let m = &folder.manifest;
    let can_read = m.permissions.terminal.contains(&TerminalAccess::Read);
    let mut uses = Uses::default();
    for (ix, hook) in m.contributes.hooks.iter().enumerate() {
        let mut cx = Cx {
            folder,
            spec: events::spec(hook.on),
            at: format!("contributes.hooks[{ix}]"),
            pattern: None,
            can_read,
        };
        check_hook(&mut cx, hook, &mut uses, out);
    }
    if m.probe.main.is_none() && !m.contributes.hooks.is_empty() {
        if can_read && !uses.reads_text {
            out.warning(
                "permission-terminal-read-unused",
                "permissions.terminal",
                "no hook sends text from a terminal anywhere; ask for the least",
            );
        }
        if m.permissions.fold && !uses.folds {
            out.warning(
                "permission-fold-unused",
                "permissions.fold",
                "no hook folds; a Probe that cannot hide output should not ask to",
            );
        }
    }
}

fn check_hook(cx: &mut Cx<'_>, hook: &Hook, uses: &mut Uses, out: &mut Findings) {
    check_match(cx, hook, out);
    match hook.actions.len() {
        0 => out.error(
            "hook-no-actions",
            format!("{}.do", cx.at),
            "a hook does something: give it at least one action",
        ),
        n if n > MAX_ACTIONS => out.error(
            "hook-too-many-actions",
            format!("{}.do", cx.at),
            format!("{n} actions; a hook may have {MAX_ACTIONS}"),
        ),
        _ => {}
    }
    for (ix, action) in hook.actions.iter().enumerate() {
        let at = format!("{}.do[{ix}]", cx.at);
        check_unused_fields(action, &at, out);
        match action.kind {
            HookActionKind::Notify => check_notify(cx, action, &at, uses, out),
            HookActionKind::Highlight => check_highlight(cx, hook.on, action, &at, out),
            HookActionKind::Fold => check_fold(cx, hook.on, action, &at, uses, out),
            HookActionKind::OpenPath => check_open_path(cx, action, &at, uses, out),
            HookActionKind::Run => check_run(cx, action, &at, uses, out),
            HookActionKind::Status => check_status(cx, action, &at, uses, out),
        }
    }
}

// ---- match ----------------------------------------------------------------------------------

fn check_match(cx: &mut Cx<'_>, hook: &Hook, out: &mut Findings) {
    for (field, value) in &hook.matcher {
        let at = format!("{}.match.{field}", cx.at);
        let Some(matcher) = cx.spec.matcher(field) else {
            out.error(
                "hook-match-field",
                at,
                format!(
                    "`{field}` is not a matcher field of `{}`: {} (a `match` names exactly these)",
                    hook.on.name(),
                    cx.spec.matcher_names()
                ),
            );
            continue;
        };
        let bad = |out: &mut Findings, why: String| out.error("hook-match-value", &at, why);
        match matcher.kind {
            MatcherKind::Number => {
                if let Err(why) = Comparison::parse(value) {
                    bad(out, why);
                }
            }
            MatcherKind::States => match list(value) {
                Err(why) => bad(out, format!("`{value}` {why}")),
                Ok(entries) => {
                    for state in entries.into_iter().filter(|s| !STATES.contains(s)) {
                        bad(out, format!("`{state}` is not a state: {}", STATES.join(", ")));
                    }
                }
            },
            MatcherKind::Names => match list(value) {
                Err(why) => bad(out, format!("`{value}` {why}")),
                Ok(entries) => {
                    for name in entries.into_iter().filter(|n| !is_agent_name(n)) {
                        bad(out, format!("`{name}` is not an id: 1 to 32 of a-z, 0-9, `-`, `_` and `/`"));
                    }
                }
            },
            MatcherKind::Globs => match list(value) {
                Err(why) => bad(out, format!("`{value}` {why}")),
                Ok(entries) => {
                    for glob in entries {
                        if let Err(why) = Glob::parse(glob) {
                            bad(out, why);
                        }
                    }
                }
            },
            MatcherKind::Pattern { .. } => cx.pattern = pattern::analyze(value, &at, out),
            MatcherKind::Front => {
                if value != "front" {
                    bad(out, format!("`{value}` is not `front`, the only thing `when` can wait for"));
                }
            }
            MatcherKind::TerminalKind => {
                if !matches!(value.as_str(), "agent" | "shell") {
                    bad(out, format!("`{value}` is not `agent` or `shell`"));
                }
            }
        }
    }
    if hook.on == HookEvent::OutputMatch && !hook.matcher.contains_key("pattern") {
        out.error(
            "hook-pattern-missing",
            format!("{}.match", cx.at),
            "`output.match` fires on a line that matches: give `pattern` (without one it would be every line)",
        );
    }
}

// ---- templates ------------------------------------------------------------------------------

/// Check a template: it parses, names only fields of the event (or groups of its pattern), holds no
/// control character, and text from the terminal goes only with `terminal:read`.
fn check_template(cx: &Cx<'_>, text: &str, at: &str, uses: &mut Uses, out: &mut Findings) {
    if has_hidden(text) {
        out.error(
            "hook-text-control",
            at,
            "must be plain text, without control, bidi or invisible characters",
        );
    }
    let pieces = match parse_template(text) {
        Ok(pieces) => pieces,
        Err(why) => {
            out.error("hook-template", at, why);
            return;
        }
    };
    for piece in pieces {
        let reads = match piece {
            Piece::Text(_) => false,
            Piece::Field(name) => match cx.spec.template_field(&name) {
                Some(field) => field.kind == FieldKind::Text,
                None => {
                    let known: Vec<_> = cx
                        .spec
                        .fields
                        .iter()
                        .chain(cx.spec.derived)
                        .chain(events::CONTEXT)
                        .map(|f| f.name)
                        .collect();
                    out.error(
                        "hook-template-field",
                        at,
                        format!(
                            "`{{{name}}}` is not a field of `{}`: {}",
                            cx.spec.event.name(),
                            known.join(", ")
                        ),
                    );
                    continue;
                }
            },
            Piece::Capture(n) => {
                match (&cx.pattern, cx.spec.has_pattern()) {
                    (Some(pattern), _) if n > pattern.groups => out.error(
                        "hook-template-group",
                        at,
                        format!("`{{{n}}}` is not a group: the pattern has {}", pattern.groups),
                    ),
                    (None, false) => out.error(
                        "hook-template-group",
                        at,
                        format!(
                            "`{{{n}}}` is a capture, and `{}` has no `pattern` to capture from",
                            cx.spec.event.name()
                        ),
                    ),
                    _ => {}
                }
                true
            }
        };
        if reads {
            uses.reads_text = true;
            if !cx.can_read {
                out.error(
                    "hook-needs-terminal-read",
                    at,
                    "this is text from a terminal, and sending it anywhere needs `terminal = [\"read\"]` in `[permissions]` (Touch ID)",
                );
            }
        }
    }
}

// ---- actions --------------------------------------------------------------------------------

/// The fields each action type reads; a field set for another type is a mistake worth saying.
fn check_unused_fields(action: &HookAction, at: &str, out: &mut Findings) {
    use HookActionKind as K;
    let set = [
        ("title", action.title.is_some(), &[K::Notify][..]),
        ("body", action.body.is_some(), &[K::Notify]),
        ("tone", action.tone.is_some(), &[K::Highlight, K::Status]),
        ("target", action.target.is_some(), &[K::Fold]),
        ("min_lines", action.min_lines.is_some(), &[K::Fold]),
        ("keep_head", action.keep_head.is_some(), &[K::Fold]),
        ("keep_tail", action.keep_tail.is_some(), &[K::Fold]),
        ("label", action.label.is_some(), &[K::Fold]),
        ("path", action.path.is_some(), &[K::OpenPath]),
        ("line", action.line.is_some(), &[K::OpenPath]),
        ("command", action.command.is_some(), &[K::Run]),
        ("timeout", action.timeout.is_some(), &[K::Run]),
        ("item", action.item.is_some(), &[K::Run, K::Status]),
        ("text", action.text.is_some(), &[K::Status]),
        ("tooltip", action.tooltip.is_some(), &[K::Status]),
        ("icon", action.icon.is_some(), &[K::Status]),
    ];
    for (field, is_set, used_by) in set {
        if is_set && !used_by.contains(&action.kind) {
            out.error(
                "hook-field-unused",
                format!("{at}.{field}"),
                format!("an action of type `{}` does not read `{field}`", action.kind.name()),
            );
        }
    }
}

fn check_tone(tone: Option<&str>, at: &str, required: bool, out: &mut Findings) {
    match tone {
        None if required => out.error("hook-tone", at, format!("needs a tone: one of {}", TONES.join(", "))),
        Some(tone) if !TONES.contains(&tone) => {
            out.error("hook-tone", at, format!("`{tone}` is not a tone: use one of {}", TONES.join(", ")));
        }
        _ => {}
    }
}

fn check_notify(cx: &Cx<'_>, action: &HookAction, at: &str, uses: &mut Uses, out: &mut Findings) {
    if !cx.folder.manifest.permissions.notify {
        out.error(
            "hook-notify-not-permitted",
            at,
            "`notify` is not allowed: set `notify = true` in `[permissions]`",
        );
    }
    match &action.title {
        None => out.error("hook-notify-title", at, "a notification needs a `title`"),
        Some(title) => check_template(cx, title, &format!("{at}.title"), uses, out),
    }
    if let Some(body) = &action.body {
        check_template(cx, body, &format!("{at}.body"), uses, out);
    }
}

fn check_highlight(cx: &Cx<'_>, on: HookEvent, action: &HookAction, at: &str, out: &mut Findings) {
    check_tone(action.tone.as_deref(), &format!("{at}.tone"), true, out);
    if !matches!(on, HookEvent::OutputMatch | HookEvent::CommandFinished) {
        out.error(
            "hook-highlight-event",
            at,
            format!("`highlight` draws on the text that matched or the block that finished: use it on `output.match` or `command.finished`, not `{}`", cx.spec.event.name()),
        );
    }
}

fn check_fold(
    cx: &Cx<'_>,
    on: HookEvent,
    action: &HookAction,
    at: &str,
    uses: &mut Uses,
    out: &mut Findings,
) {
    uses.folds = true;
    if !cx.folder.manifest.permissions.fold {
        out.error(
            "hook-fold-not-permitted",
            at,
            "`fold` is not allowed: set `fold = true` in `[permissions]` (it can hide parts of the terminals' output, so enabling it takes Touch ID)",
        );
    }
    match (action.target, on) {
        (None, _) => out.error("hook-fold-target", at, "needs a `target`: `block-output` or `match-run`"),
        (Some(FoldTarget::BlockOutput), HookEvent::CommandFinished)
        | (Some(FoldTarget::MatchRun), HookEvent::OutputMatch) => {}
        (Some(target), _) => out.error(
            "hook-fold-event",
            format!("{at}.target"),
            format!(
                "`{}` folds {}: it belongs to `{}`, not `{}`",
                target.name(),
                match target {
                    FoldTarget::BlockOutput => "the output of the command that finished",
                    FoldTarget::MatchRun => "the run of lines that matched",
                },
                match target {
                    FoldTarget::BlockOutput => "command.finished",
                    FoldTarget::MatchRun => "output.match",
                },
                on.name()
            ),
        ),
    }
    match &action.label {
        None => out.error("hook-fold-label", at, "a fold needs a `label`: the marker's words, fixed text"),
        Some(label) => {
            let at = format!("{at}.label");
            if label.trim().is_empty() || label.chars().count() > MAX_LABEL_CHARS {
                out.error("hook-fold-label", &at, format!("1 to {MAX_LABEL_CHARS} characters"));
            }
            if label.chars().any(|c| is_hidden(c) || c == '{' || c == '}') {
                out.error(
                    "hook-fold-label",
                    at,
                    "fixed plain text: no field or capture (output must not dress a marker up), no control, bidi or invisible characters",
                );
            }
        }
    }
    for (field, value) in [("keep_head", action.keep_head), ("keep_tail", action.keep_tail)] {
        if let Some(n) = value.filter(|n| *n > MAX_KEEP) {
            out.error("hook-fold-keep", format!("{at}.{field}"), format!("{n} rows; at most {MAX_KEEP}"));
        }
    }
    if let Some(n) = action.min_lines.filter(|n| !(1..=MAX_HIDDEN_ROWS).contains(n)) {
        out.error("hook-fold-min-lines", format!("{at}.min_lines"), format!("{n}: 1 to {MAX_HIDDEN_ROWS}"));
    }
    let kept = action.keep_head.unwrap_or(0).saturating_add(action.keep_tail.unwrap_or(0));
    if let Some(n) = action.min_lines
        && n < kept.saturating_add(MIN_HIDDEN_ROWS)
    {
        out.warning(
            "hook-fold-min-lines-low",
            format!("{at}.min_lines"),
            format!("a fold hides at least {MIN_HIDDEN_ROWS} rows beyond the {kept} it keeps, so fewer than {} lines are never folded", kept + MIN_HIDDEN_ROWS),
        );
    }
}

fn check_open_path(cx: &Cx<'_>, action: &HookAction, at: &str, uses: &mut Uses, out: &mut Findings) {
    let Some(path) = &action.path else {
        out.error("hook-open-path", at, "`open-path` needs a `path`");
        return;
    };
    let path_at = format!("{at}.path");
    if path.len() > MAX_PATH_BYTES || path.contains("://") || path.contains('\0') {
        out.error(
            "hook-open-path",
            &path_at,
            format!(
                "a file's path, at most {MAX_PATH_BYTES} bytes and not a link: a link is an `open-url` rule's"
            ),
        );
    }
    if path.split('/').any(|segment| segment == "..") {
        out.error(
            "hook-open-path",
            &path_at,
            "has a `..` segment, which walks the path up out of where it was written",
        );
    }
    check_template(cx, path, &path_at, uses, out);
    if let (Ok(pieces), Some(pattern)) = (parse_template(path), &cx.pattern) {
        for n in pieces.iter().filter_map(|p| if let Piece::Capture(n) = p { Some(*n) } else { None }) {
            if n <= pattern.groups && pattern.capture_hazard(n) == Some(pattern::CaptureHazard::Unbounded) {
                out.error(
                    "capture-unanchored",
                    &path_at,
                    format!(
                        "capture {n} can be longer than {} bytes or unbounded; bound it, as in `{{1,200}}`",
                        pattern::MAX_ARGUMENT_BYTES
                    ),
                );
            }
        }
    }
    if let Some(line) = &action.line {
        let line_at = format!("{at}.line");
        check_template(cx, line, &line_at, uses, out);
        let number = |text: &str| (1..=7).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_digit());
        let single = match parse_template(line).as_deref() {
            Ok([Piece::Text(text)]) => number(text),
            Ok([Piece::Capture(_)]) => true,
            Ok([Piece::Field(name)]) => {
                cx.spec.template_field(name).is_some_and(|f| f.kind == FieldKind::Number)
            }
            Ok(_) => false,
            Err(_) => true, // said already
        };
        if !single {
            out.error(
                "hook-open-path-line",
                line_at,
                "is a whole number, or one number field or capture such as `{2}`",
            );
        }
    }
}

fn check_run(cx: &Cx<'_>, action: &HookAction, at: &str, uses: &mut Uses, out: &mut Findings) {
    let m = &cx.folder.manifest;
    let Some(name) = &action.command else {
        out.error("hook-run-command", at, "`run` needs the id of one of this Probe's commands");
        return;
    };
    if let Some(timeout) = action.timeout.filter(|t| !(1..=RUN_TIMEOUT_MAX).contains(t)) {
        out.error(
            "hook-run-timeout",
            format!("{at}.timeout"),
            format!("{timeout} s is out of range: 1 to {RUN_TIMEOUT_MAX} s"),
        );
    }
    if let Some(item) = &action.item {
        check_item(cx, item, &format!("{at}.item"), out);
    }
    let path = format!("{at}.command");
    let Some(command) = m.contributes.commands.iter().find(|c| &c.id == name) else {
        out.error("hook-run-command", &path, format!("this Probe has no command `{name}`"));
        return;
    };
    if command.run_in != RunIn::Background {
        out.error(
            "hook-run-in-terminal",
            &path,
            format!("`{name}` opens a terminal, and an event can fire while nobody is looking; a hook runs only commands with `in = \"background\"`"),
        );
    }
    let Some(run) = &command.run else {
        if m.probe.main.is_none() {
            out.error(
                "hook-run-no-argv",
                &path,
                format!("`{name}` has no `run`, so only code could handle it"),
            );
        }
        return;
    };
    let captures = argv::captures_of(run);
    if captures.is_empty() {
        return;
    }
    // A capture that reaches a program is terminal text going to it.
    uses.reads_text = true;
    if !cx.can_read {
        out.error(
            "hook-needs-terminal-read",
            &path,
            format!("`{name}` takes captures, which are text from a terminal handed to a program: that needs `terminal = [\"read\"]` in `[permissions]` (Touch ID)"),
        );
    }
    if !cx.spec.has_pattern() {
        out.error(
            "hook-run-capture-without-pattern",
            &path,
            format!("`{name}` takes `$n`, and `{}` has no `pattern` to capture from", cx.spec.event.name()),
        );
    } else {
        check_command_target(command, cx.pattern.as_ref(), &path, out);
    }
}

fn check_status(cx: &Cx<'_>, action: &HookAction, at: &str, uses: &mut Uses, out: &mut Findings) {
    match &action.item {
        None => {
            out.error("hook-status-item", at, "`status` needs the id of one of this Probe's status items")
        }
        Some(item) => check_item(cx, item, &format!("{at}.item"), out),
    }
    match &action.text {
        None => out.error("hook-status-text", at, "`status` needs a `text`"),
        Some(text) => check_template(cx, text, &format!("{at}.text"), uses, out),
    }
    if let Some(tooltip) = &action.tooltip {
        check_template(cx, tooltip, &format!("{at}.tooltip"), uses, out);
    }
    check_tone(action.tone.as_deref(), &format!("{at}.tone"), false, out);
    if let Some(icon) = action.icon.as_deref().filter(|i| !is_icon_name(i)) {
        out.error(
            "hook-status-icon",
            format!("{at}.icon"),
            format!("`{icon}` is not an icon name: 1 to 32 of a-z, 0-9 and `-`"),
        );
    }
}

fn check_item(cx: &Cx<'_>, item: &str, at: &str, out: &mut Findings) {
    if !cx.folder.manifest.contributes.status.iter().any(|s| s.id == item) {
        out.error("hook-status-item", at, format!("this Probe has no status item `{item}`"));
    }
}

/// Whether a hook feeds the status item `id`: a status item with no `run` is fed this way, or by code.
pub(super) fn feeds_status(folder: &ProbeFolder, id: &str) -> bool {
    folder.manifest.contributes.hooks.iter().flat_map(|h| &h.actions).any(|a| {
        matches!(a.kind, HookActionKind::Status | HookActionKind::Run) && a.item.as_deref() == Some(id)
    })
}
