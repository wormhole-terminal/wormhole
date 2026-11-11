//! What a `run` action's process printed and how it ended, as the effects it may have: the closed
//! exit-code and JSON rules of `repo-docs/docs/research/probes-hooks-and-fold.md`, section 3.4.
//!
//! - exit 0: stdout is read (at most [`RUN_OUTPUT_MAX`] bytes). One JSON object of the closed schema
//!   `{"status": {"text", "icon", "tone", "tooltip"}, "notify": {"title", "body"}}`, or else its first line
//!   is the text of the status item the action feeds. A key the schema does not know is ignored, with a
//!   warning; so is an effect the Probe did not ask for.
//! - any other exit, 2 included: no effect on the terminal; the status item the action feeds shows
//!   `failed (exit N)`. Nothing blocks, rewrites or answers: the JSON of other tools' hooks that decides
//!   (`decision`, `continue`, `updatedInput`) simply is not in the schema.
//!
//! Pure: the caller ran the process and passes what it printed.

use serde_json::{Map, Value};

use crate::probe::events::{
    MAX_BODY_CHARS, MAX_STATUS_CHARS, MAX_TITLE_CHARS, MAX_TOOLTIP_CHARS, RUN_OUTPUT_MAX,
};
use crate::probe::validate::TONES;

use super::plan::{clean, cut};

/// What a process may set on a status item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusUpdate {
    pub text: String,
    pub icon: Option<String>,
    pub tone: Option<String>,
    pub tooltip: Option<String>,
}

/// A notification a process asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyText {
    pub title: String,
    pub body: String,
}

/// What the Probe has allowed the output to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowed {
    /// The action feeds a status item.
    pub status: bool,
    /// The Probe has `notify = true`.
    pub notify: bool,
}

/// What a finished process comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    Failed { exit: i32 },
    Effects { status: Option<StatusUpdate>, notify: Option<NotifyText>, warnings: Vec<String> },
}

/// `stdout` cut to what is read, at a character boundary.
fn capped(stdout: &str) -> &str {
    if stdout.len() <= RUN_OUTPUT_MAX {
        return stdout;
    }
    let mut end = RUN_OUTPUT_MAX;
    while !stdout.is_char_boundary(end) {
        end -= 1;
    }
    &stdout[..end]
}

/// The effects of a process that exited with `exit` and printed `stdout`.
pub fn parse_run_output(exit: i32, stdout: &str, allowed: Allowed) -> RunOutcome {
    if exit != 0 {
        return RunOutcome::Failed { exit };
    }
    let text = capped(stdout);
    let mut warnings = Vec::new();
    let (mut status, mut notify) = (None, None);
    if text.trim_start().starts_with('{') {
        match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(object)) => {
                (status, notify) = from_object(&object, allowed, &mut warnings);
            }
            _ => warnings
                .push("the output starts with `{` but is not one JSON object (or was cut at 4 KiB)".into()),
        }
    } else if let Some(line) = text.lines().next().map(str::trim).filter(|l| !l.is_empty())
        && allowed.status
    {
        status = Some(StatusUpdate {
            text: cut(&clean(line), MAX_STATUS_CHARS),
            icon: None,
            tone: None,
            tooltip: None,
        });
    }
    RunOutcome::Effects { status, notify, warnings }
}

fn from_object(
    object: &Map<String, Value>,
    allowed: Allowed,
    warnings: &mut Vec<String>,
) -> (Option<StatusUpdate>, Option<NotifyText>) {
    for key in object.keys().filter(|k| !matches!(k.as_str(), "status" | "notify")) {
        warnings.push(format!("unknown key `{key}` ignored"));
    }
    let status = object.get("status").and_then(|value| {
        if !allowed.status {
            warnings.push("`status` ignored: this action feeds no status item".into());
            return None;
        }
        status_of(value, warnings)
    });
    let notify = object.get("notify").and_then(|value| {
        if !allowed.notify {
            warnings.push("`notify` ignored: the Probe has no `notify` permission".into());
            return None;
        }
        notify_of(value, warnings)
    });
    (status, notify)
}

/// The string at `key`, when there is one; a value of another type is a warning.
fn string(
    object: &Map<String, Value>,
    key: &str,
    within: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match object.get(key)? {
        Value::String(text) => Some(text.clone()),
        _ => {
            warnings.push(format!("`{within}.{key}` is not text: ignored"));
            None
        }
    }
}

fn warn_unknown(object: &Map<String, Value>, known: &[&str], within: &str, warnings: &mut Vec<String>) {
    for key in object.keys().filter(|k| !known.contains(&k.as_str())) {
        warnings.push(format!("unknown key `{within}.{key}` ignored"));
    }
}

fn status_of(value: &Value, warnings: &mut Vec<String>) -> Option<StatusUpdate> {
    let Value::Object(object) = value else {
        warnings.push("`status` is not an object: ignored".into());
        return None;
    };
    warn_unknown(object, &["text", "icon", "tone", "tooltip"], "status", warnings);
    let text = string(object, "text", "status", warnings).filter(|t| !t.trim().is_empty());
    let Some(text) = text else {
        warnings.push("`status.text` is missing: the status update is ignored".into());
        return None;
    };
    let tone = string(object, "tone", "status", warnings).filter(|tone| {
        let known = TONES.contains(&tone.as_str());
        if !known {
            warnings.push(format!("`status.tone` `{tone}` is not a tone ({}): ignored", TONES.join(", ")));
        }
        known
    });
    let icon = string(object, "icon", "status", warnings).filter(|icon| {
        (1..=32).contains(&icon.len())
            && icon.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    });
    Some(StatusUpdate {
        text: cut(&clean(&text), MAX_STATUS_CHARS),
        icon,
        tone,
        tooltip: string(object, "tooltip", "status", warnings).map(|t| cut(&clean(&t), MAX_TOOLTIP_CHARS)),
    })
}

fn notify_of(value: &Value, warnings: &mut Vec<String>) -> Option<NotifyText> {
    let Value::Object(object) = value else {
        warnings.push("`notify` is not an object: ignored".into());
        return None;
    };
    warn_unknown(object, &["title", "body"], "notify", warnings);
    let Some(title) = string(object, "title", "notify", warnings).filter(|t| !t.trim().is_empty()) else {
        warnings.push("`notify.title` is missing: the notification is ignored".into());
        return None;
    };
    Some(NotifyText {
        title: cut(&clean(&title), MAX_TITLE_CHARS),
        body: string(object, "body", "notify", warnings)
            .map_or_else(String::new, |b| cut(&clean(&b), MAX_BODY_CHARS)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: Allowed = Allowed { status: true, notify: true };

    fn effects(outcome: RunOutcome) -> (Option<StatusUpdate>, Option<NotifyText>, Vec<String>) {
        match outcome {
            RunOutcome::Effects { status, notify, warnings } => (status, notify, warnings),
            RunOutcome::Failed { exit } => panic!("failed {exit}"),
        }
    }

    #[test]
    fn any_exit_but_zero_is_a_failure_and_two_is_not_special() {
        for exit in [1, 2, 127, -1] {
            assert_eq!(
                parse_run_output(exit, "{\"status\":{\"text\":\"x\"}}", BOTH),
                RunOutcome::Failed { exit }
            );
        }
    }

    #[test]
    fn the_first_line_is_the_text_of_the_item_the_action_feeds() {
        let (status, notify, warnings) = effects(parse_run_output(0, "  3 failing\nmore\n", BOTH));
        assert_eq!(status.map(|s| s.text), Some("3 failing".into()));
        assert_eq!((notify, warnings), (None, vec![]));
        let (status, ..) = effects(parse_run_output(0, "3 failing", Allowed { status: false, notify: true }));
        assert_eq!(status, None, "no item to feed");
        let (status, ..) = effects(parse_run_output(0, "\n\n", BOTH));
        assert_eq!(status, None);
    }

    #[test]
    fn one_json_object_of_the_closed_schema_sets_a_status_and_a_notification() {
        let out = r#"{"status":{"text":"red","tone":"danger","icon":"alert","tooltip":"2 failed"},"notify":{"title":"Tests","body":"2 failed"}}"#;
        let (status, notify, warnings) = effects(parse_run_output(0, out, BOTH));
        assert_eq!(
            status,
            Some(StatusUpdate {
                text: "red".into(),
                icon: Some("alert".into()),
                tone: Some("danger".into()),
                tooltip: Some("2 failed".into())
            })
        );
        assert_eq!(notify, Some(NotifyText { title: "Tests".into(), body: "2 failed".into() }));
        assert_eq!(warnings, Vec::<String>::new());
    }

    #[test]
    fn what_decides_or_rewrites_is_not_in_the_schema() {
        let out = r#"{"continue":false,"decision":"block","hookSpecificOutput":{"updatedInput":{}},"terminalSequence":"\u001b[2J","status":{"text":"x","action":"y"}}"#;
        let (status, notify, warnings) = effects(parse_run_output(0, out, BOTH));
        assert_eq!(status.map(|s| s.text), Some("x".into()));
        assert_eq!(notify, None);
        for key in ["continue", "decision", "hookSpecificOutput", "terminalSequence", "status.action"] {
            assert!(warnings.iter().any(|w| w.contains(&format!("`{key}`"))), "{key}: {warnings:?}");
        }
    }

    #[test]
    fn an_effect_the_probe_did_not_ask_for_is_ignored_with_a_warning() {
        let out = r#"{"status":{"text":"x"},"notify":{"title":"t"}}"#;
        let (status, notify, warnings) =
            effects(parse_run_output(0, out, Allowed { status: false, notify: false }));
        assert_eq!((status, notify), (None, None));
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn a_bad_part_is_dropped_and_the_rest_stays() {
        let out = r##"{"status":{"text":"x","tone":"#ff0000","icon":"../x"},"notify":{"body":"no title"}}"##;
        let (status, notify, warnings) = effects(parse_run_output(0, out, BOTH));
        let status = status.expect("the text is fine");
        assert_eq!((status.tone, status.icon), (None, None));
        assert_eq!(notify, None);
        assert!(warnings.iter().any(|w| w.contains("not a tone")));
        assert!(warnings.iter().any(|w| w.contains("`notify.title` is missing")));
    }

    #[test]
    fn a_brace_that_is_not_one_object_has_no_effect() {
        for out in ["{not json", "{\"status\":", "[1]\n", "{\"status\":{\"text\":\"x\"}} trailing"] {
            let (status, notify, warnings) = effects(parse_run_output(0, out, BOTH));
            assert_eq!((status.is_some(), notify.is_some()), (out.starts_with('['), false), "{out}");
            assert_eq!(warnings.is_empty(), out.starts_with('['), "{out}");
        }
    }

    #[test]
    fn text_is_cleaned_and_cut_and_only_the_first_four_kib_are_read() {
        let out = format!("{{\"status\":{{\"text\":\"a\\u001b[31m{}\"}}}}", "b".repeat(200));
        let (status, ..) = effects(parse_run_output(0, &out, BOTH));
        let text = status.expect("status").text;
        assert_eq!(text.chars().count(), MAX_STATUS_CHARS);
        assert!(text.starts_with("a[31m") && text.ends_with('…'), "{text}");
        let big = format!("{}{}", "x".repeat(RUN_OUTPUT_MAX), "\n{ignored}");
        let (status, ..) = effects(parse_run_output(0, &big, BOTH));
        assert_eq!(status.map(|s| s.text.chars().count()), Some(MAX_STATUS_CHARS));
    }
}
