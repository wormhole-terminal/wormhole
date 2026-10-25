//! `[[contributes.rules]]`: patterns that decorate or act on the terminal's text
//! (section 3.1 of the design).

use super::Findings;
use super::argv;
use super::pattern::{self, CaptureHazard, Pattern};
use super::template;
use crate::probe::hidden::{has_hidden, is_hidden};
use crate::probe::manifest::{Action, ActionKind, Command, KeepSpec, Look, Rule};
use crate::probe::read::ProbeFolder;

/// The design tokens a rule's `tone` may name, resolved through the theme by the app.
pub(crate) const TONES: &[&str] = &["link", "danger", "warning", "success", "muted"];
/// The longest hover text, in characters.
pub(crate) const MAX_HOVER_CHARS: usize = 120;
/// The longest link, in bytes.
const MAX_URL_BYTES: usize = 2048;

pub(super) fn check(folder: &ProbeFolder, out: &mut Findings) {
    let m = &folder.manifest;
    for (ix, rule) in m.contributes.rules.iter().enumerate() {
        let at = |field: &str| format!("contributes.rules[{ix}].{field}");
        let pattern = pattern::analyze(&rule.pattern, &at("pattern"), out);
        check_look(rule, &at, out);
        check_tone(rule, &at, out);
        let groups = pattern.as_ref().map(|p| p.groups);
        if let Some(hover) = &rule.hover {
            check_hover(hover, groups, &at("hover"), out);
        }
        if let Some(action) = &rule.action {
            check_action(folder, action, pattern.as_ref(), &at, out);
        }
        if rule.look == Look::None
            && rule.tone.is_none()
            && rule.action.as_ref().is_none_or(|a| a.kind == ActionKind::None)
        {
            out.warning(
                "rule-does-nothing",
                format!("contributes.rules[{ix}]"),
                "draws nothing and does nothing: it needs a look, a tone or an action",
            );
        }
    }
}

fn check_look(rule: &Rule, at: &dyn Fn(&str) -> String, out: &mut Findings) {
    match (&rule.keep, rule.look) {
        (Some(keep), Look::Shorten) => {
            if let Err(why) = KeepSpec::parse(keep) {
                out.error("rule-keep", at("keep"), why);
            }
        }
        (Some(_), _) => out.error(
            "rule-keep-needs-shorten",
            at("keep"),
            "`keep` says what `look = \"shorten\"` leaves visible",
        ),
        (None, Look::Shorten) => out.warning(
            "rule-shorten-keeps-nothing",
            at("look"),
            "shortens without `keep`, so nothing of the text stays visible",
        ),
        (None, _) => {}
    }
    if rule.reveal.is_some() && rule.look == Look::None {
        out.error(
            "rule-reveal-needs-look",
            at("reveal"),
            "there is nothing to reveal when `look` is \"none\"",
        );
    }
}

fn check_tone(rule: &Rule, at: &dyn Fn(&str) -> String, out: &mut Findings) {
    let Some(tone) = &rule.tone else { return };
    if TONES.contains(&tone.as_str()) {
        return;
    }
    let colour = tone.starts_with('#')
        || tone.starts_with("rgb")
        || tone.starts_with("hsl")
        || (tone.len() >= 3 && tone.bytes().all(|b| b.is_ascii_hexdigit()));
    let message = if colour {
        format!(
            "`{tone}` is a colour; a tone is a design token's name, so it follows the theme: {}",
            TONES.join(", ")
        )
    } else {
        format!("`{tone}` is not a tone: use one of {}", TONES.join(", "))
    };
    out.error(if colour { "tone-colour" } else { "tone-unknown" }, at("tone"), message);
}

fn check_hover(hover: &str, groups: Option<usize>, path: &str, out: &mut Findings) {
    let chars = hover.chars().count();
    if chars > MAX_HOVER_CHARS {
        out.error("hover-too-long", path, format!("{chars} characters; at most {MAX_HOVER_CHARS}"));
    }
    if has_hidden(hover) {
        out.error("hover-control", path, "must be plain text, without control, bidi or invisible characters");
    }
    check_template_groups(hover, groups, path, "hover-template", "hover-group", out);
}

/// A template's `{n}` are well formed, and name groups the pattern has.
fn check_template_groups(
    text: &str,
    groups: Option<usize>,
    path: &str,
    bad_code: &'static str,
    group_code: &'static str,
    out: &mut Findings,
) {
    match template::placeholders(text) {
        Err(why) => out.error(bad_code, path, why),
        Ok(used) => {
            if let Some(groups) = groups {
                for n in used.into_iter().filter(|&n| n > groups) {
                    out.error(
                        group_code,
                        path,
                        format!("`{{{n}}}` is not a group: the pattern has {groups}"),
                    );
                }
            }
        }
    }
}

fn check_action(
    folder: &ProbeFolder,
    action: &Action,
    pattern: Option<&Pattern>,
    at: &dyn Fn(&str) -> String,
    out: &mut Findings,
) {
    let permissions = &folder.manifest.permissions;
    let groups = pattern.map(|p| p.groups);
    let action_at = |field: &str| at(&format!("action.{field}"));
    // A field the type does not read is a mistake worth saying, not a silent no-op.
    for (field, set, used) in [
        ("url", action.url.is_some(), action.kind == ActionKind::OpenUrl),
        ("command", action.command.is_some(), action.kind == ActionKind::Command),
        ("text", action.text.is_some(), action.kind == ActionKind::Copy),
    ] {
        if set && !used {
            out.error(
                "action-field-unused",
                action_at(field),
                format!("an action of this type does not read `{field}`"),
            );
        }
    }
    match action.kind {
        ActionKind::None => {}
        ActionKind::Copy => {
            if let Some(text) = &action.text {
                check_template_groups(
                    text,
                    groups,
                    &action_at("text"),
                    "action-template",
                    "action-group",
                    out,
                );
            }
        }
        ActionKind::OpenUrl => {
            let Some(url) = &action.url else {
                out.error("action-url-missing", at("action"), "`open-url` needs a `url`");
                return;
            };
            let path = action_at("url");
            if url.len() > MAX_URL_BYTES || url.chars().any(|c| is_hidden(c) || c.is_whitespace()) {
                out.error(
                    "action-url",
                    &path,
                    format!("must be one line without spaces, at most {MAX_URL_BYTES} bytes"),
                );
            }
            if template::has_dot_segment(url) {
                out.error(
                    "action-url-dot-segment",
                    &path,
                    "has a `.` or `..` segment in its path, which walks the link up out of where it was written",
                );
            }
            match template::https_host(url) {
                Err(why) => out.error(
                    if why.contains("capture") { "action-url-host-capture" } else { "action-url" },
                    &path,
                    format!("`{url}` {why}"),
                ),
                Ok(host) => {
                    if !permissions.open.iter().any(|allowed| allowed == host) {
                        out.error(
                            "action-url-host-not-permitted",
                            &path,
                            format!(
                                "`{host}` is not in `permissions.open`, which lists the hosts a rule may open"
                            ),
                        );
                    }
                }
            }
            check_template_groups(url, groups, &path, "action-template", "action-group", out);
        }
        ActionKind::Command => {
            let Some(name) = &action.command else {
                out.error(
                    "action-command-missing",
                    at("action"),
                    "`command` needs the id of one of this Probe's commands",
                );
                return;
            };
            let path = action_at("command");
            let Some(command) = folder.manifest.contributes.commands.iter().find(|c| &c.id == name) else {
                out.error("action-command-unknown", path, format!("this Probe has no command `{name}`"));
                return;
            };
            check_command_target(command, pattern, &path, out);
        }
    }
}

/// A rule's captures arrive in `command` as its `$n`. Every capture that does must exist, and be
/// anchored (see [`Pattern::capture_hazard`]).
fn check_command_target(command: &Command, pattern: Option<&Pattern>, path: &str, out: &mut Findings) {
    let Some(pattern) = pattern else { return };
    let used: Vec<usize> = match &command.run {
        Some(run) => argv::captures_of(run),
        // Handled by code: whatever it does with the captures is not visible, so all of them count.
        None => (0..=pattern.groups).collect(),
    };
    for &index in &used {
        if index > pattern.groups {
            out.error(
                "action-capture-missing",
                path,
                format!("the command uses `${index}` but the pattern has {} group(s)", pattern.groups),
            );
            continue;
        }
        let what = if index == 0 { "the whole match".to_string() } else { format!("capture {index}") };
        match pattern.capture_hazard(index) {
            Some(CaptureHazard::Unbounded) => out.error(
                "capture-unanchored",
                path,
                format!(
                    "{what} goes to a program as an argument but can be longer than {} bytes or unbounded; bound it, as in `{{1,39}}`",
                    pattern::MAX_ARGUMENT_BYTES
                ),
            ),
            Some(CaptureHazard::MayStartWithDash) => out.error(
                "capture-unanchored",
                path,
                format!(
                    "{what} goes to a program as an argument but can start with `-`, which a program reads as an option; start it with a class that leaves `-` out, as in `([0-9a-f][0-9a-f]{{6,39}})`"
                ),
            ),
            Some(CaptureHazard::MayBeEmpty) => out.error(
                "capture-unanchored",
                path,
                format!(
                    "{what} goes to a program as an argument but can be empty or not take part in a match; an empty argument is not a value, so require at least one character in a group that always takes part"
                ),
            ),
            None => {}
        }
    }
}
