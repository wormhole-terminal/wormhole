//! `actions.json`: user-invokable commands (palette entries, optionally bound to a key).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use super::trust::TrustLine;
use super::{ProjectWarning, check_relative};
use crate::keys::keymap;
use crate::keys::keyspec::KeySpec;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Key binding, e.g. `"cmd-shift-r"` (see [`crate::keys::keyspec`]); checked at load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(flatten)]
    pub kind: ActionKind,
}

impl Action {
    /// The parsed `key`, for a loaded (so validated) action: a key that couldn't be used was
    /// dropped at load (see [`validate`]).
    pub fn key_spec(&self) -> Option<KeySpec> {
        self.key.as_deref()?.parse().ok()
    }
}

/// A project action's `key`: well-formed, a key that exists, with ⌘ or an F-key (⌃ and ⌥
/// keys are the shell's, a plain key is typing), and one nobody uses yet: not an app key (the
/// keymap's table, planned features' and the reserved ones included) and not one macOS keeps.
/// Project keys are matched before the app's and the terminal's, so taking one of those would
/// break it. The error says why, in the Project tab's words ("⌘T is New Terminal").
fn validate_key(spec: &str) -> Result<(), String> {
    let key: KeySpec = spec.parse().map_err(|e| format!("{spec:?}: {e}"))?;
    if !key.has_known_key() {
        return Err(format!("{spec:?}: unknown key {:?}", key.key));
    }
    if !(key.mods.cmd || key.is_function_key()) {
        return Err(format!(
            "{spec:?} needs ⌘ or an F-key (⌃ and ⌥ keys are the shell's, the rest is typing)"
        ));
    }
    match keymap::uses(&key) {
        Some(taken) => Err(taken.to_string()),
        None => Ok(()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    /// Type `command` into a terminal and press Enter.
    Run {
        command: String,
        #[serde(default)]
        target: Target,
        /// Relative to the project root; only applies when a new session is created.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    /// Paste a prompt (inline `text`, or `file` relative to `prompts/`) into a session.
    Prompt {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        #[serde(default)]
        target: Target,
        /// Press Enter after pasting. Off by default so the user can review first.
        #[serde(default)]
        submit: bool,
    },
    Open {
        url: String,
    },
    /// Run steps in order: ids of other actions or inline action bodies.
    Chain {
        steps: Vec<ChainStep>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChainStep {
    Ref(String),
    Inline(Box<ActionKind>),
}

/// Where a run/prompt action goes: `"current"`, `"new_session"` or `"session:<name>"`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Target {
    #[default]
    Current,
    NewSession,
    Session(String),
}

impl TryFrom<String> for Target {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        match s.as_str() {
            "current" => Ok(Self::Current),
            "new_session" => Ok(Self::NewSession),
            _ => match s.strip_prefix("session:") {
                Some(name) if !name.trim().is_empty() => Ok(Self::Session(name.to_string())),
                _ => Err(format!(
                    "invalid target {s:?}: expected \"current\", \"new_session\" or \"session:<name>\""
                )),
            },
        }
    }
}

impl From<Target> for String {
    fn from(t: Target) -> Self {
        t.to_string()
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => f.write_str("current"),
            Self::NewSession => f.write_str("new_session"),
            Self::Session(name) => write!(f, "session:{name}"),
        }
    }
}

/// The text of a prompt file as an action pastes it, by the action's `file`.
pub(crate) type PromptText<'a> = &'a dyn Fn(&str) -> Option<&'a str>;

/// Longest prompt text quoted in full in the approval dialog; the fingerprint covers all of it.
const PROMPT_PREVIEW_CHARS: usize = 400;

impl ActionKind {
    /// One line per thing this would execute, for the trust dialog: what runs it (`prefix`,
    /// the action and its key), what it runs or types, and where. A prompt file's text is
    /// quoted, since pasting (and submitting) it is what runs.
    pub(crate) fn describe(&self, out: &mut Vec<TrustLine>, prefix: &str, prompt_text: PromptText<'_>) {
        match self {
            Self::Run { command, target, cwd } => {
                let cwd = cwd.as_deref().map(|c| format!(" in {c}")).unwrap_or_default();
                out.push(TrustLine::new(format!("{prefix}: run"), command, format!("({target}{cwd})")));
            }
            Self::Prompt { text, file, target, submit } => {
                let verb = if *submit { "send prompt" } else { "paste prompt" };
                let (what, body) = match (text, file) {
                    (_, Some(file)) => {
                        (format!("{prefix}: {verb} prompts/{file}"), prompt_text(file).map(preview))
                    }
                    (text, None) => (format!("{prefix}: {verb}"), text.clone()),
                };
                out.push(TrustLine::new(what, body.unwrap_or_default(), format!("({target})")));
            }
            Self::Open { url } => out.push(TrustLine::new(format!("{prefix}: open"), url, "")),
            Self::Chain { steps } => {
                for (ix, step) in steps.iter().enumerate() {
                    match step {
                        ChainStep::Ref(id) => {
                            out.push(TrustLine::new(format!("{prefix} step {}: action", ix + 1), id, ""))
                        }
                        ChainStep::Inline(kind) => {
                            kind.describe(out, &format!("{prefix} step {}", ix + 1), prompt_text)
                        }
                    }
                }
            }
        }
    }
}

/// A prompt's text on one line (line breaks become `\n` when the line is shown), cut short with
/// a count of what is left out.
fn preview(text: &str) -> String {
    let total = text.chars().count();
    if total <= PROMPT_PREVIEW_CHARS {
        return text.to_string();
    }
    let head: String = text.chars().take(PROMPT_PREVIEW_CHARS).collect();
    format!("{head}… ({} more characters)", total - PROMPT_PREVIEW_CHARS)
}

/// The `file` of every prompt action, chain steps included: the files trust has to cover.
pub(super) fn prompt_files(actions: &[Action]) -> BTreeSet<&str> {
    fn walk<'a>(kind: &'a ActionKind, out: &mut BTreeSet<&'a str>) {
        match kind {
            ActionKind::Prompt { file: Some(file), .. } => {
                out.insert(file);
            }
            ActionKind::Chain { steps } => {
                for step in steps {
                    if let ChainStep::Inline(inner) = step {
                        walk(inner, out);
                    }
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    for action in actions {
        walk(&action.kind, &mut out);
    }
    out
}

/// Whether a prompt action's `file` can be used; the error says why not ("does not exist").
pub(super) type PromptCheck<'a> = &'a dyn Fn(&str) -> Result<(), String>;

/// Check what serde can't: ids, references, cycles and per-kind invariants. Returns a message
/// naming the offending action. A key that can't be used (taken, without ⌘, misspelt) is not
/// a reason to refuse the file: that key alone is dropped, with a warning for the Project tab,
/// and the action still runs from the palette and the menu.
pub(super) fn validate(
    actions: &mut [Action],
    prompt_ok: PromptCheck<'_>,
) -> Result<Vec<ProjectWarning>, String> {
    let mut ids = BTreeSet::new();
    let mut warnings = Vec::new();
    for (ix, action) in actions.iter_mut().enumerate() {
        let at = format!("actions[{ix}] ({:?})", action.id);
        if action.id.trim().is_empty() {
            return Err(format!("actions[{ix}]: \"id\" must not be empty"));
        }
        if action.title.trim().is_empty() {
            return Err(format!("{at}: \"title\" must not be empty"));
        }
        if !ids.insert(action.id.clone()) {
            return Err(format!("{at}: duplicate id {:?}", action.id));
        }
        if let Some(why) = action.key.as_deref().and_then(|key| validate_key(key).err()) {
            let key = action.key.take().unwrap_or_default();
            warnings.push(ProjectWarning::ActionKey { action: action.id.clone(), key, why });
        }
        validate_kind(&action.kind, prompt_ok).map_err(|m| format!("{at}: {m}"))?;
    }
    let by_id: BTreeMap<&str, &Action> = actions.iter().map(|action| (action.id.as_str(), action)).collect();
    for action in actions.iter() {
        let mut refs = Vec::new();
        collect_refs(&action.kind, &mut refs);
        if let Some(missing) = refs.iter().find(|r| !by_id.contains_key(**r)) {
            return Err(format!("action {:?}: chain step refers to unknown action {missing:?}", action.id));
        }
    }
    // Depth-first search with an explicit path, so the error can show the whole cycle.
    let mut done = BTreeSet::new();
    for action in actions.iter() {
        let mut path = Vec::new();
        visit(&action.id, &by_id, &mut path, &mut done)?;
    }
    Ok(warnings)
}

fn visit<'a>(
    id: &'a str,
    by_id: &BTreeMap<&'a str, &'a Action>,
    path: &mut Vec<&'a str>,
    done: &mut BTreeSet<&'a str>,
) -> Result<(), String> {
    if done.contains(id) {
        return Ok(());
    }
    if let Some(start) = path.iter().position(|p| *p == id) {
        let mut cycle = path[start..].to_vec();
        cycle.push(id);
        return Err(format!("chain cycle: {}", cycle.join(" -> ")));
    }
    let Some(action) = by_id.get(id) else { return Ok(()) };
    path.push(id);
    let mut refs = Vec::new();
    collect_refs(&action.kind, &mut refs);
    for next in refs {
        visit(next, by_id, path, done)?;
    }
    path.pop();
    done.insert(id);
    Ok(())
}

fn collect_refs<'a>(kind: &'a ActionKind, out: &mut Vec<&'a str>) {
    if let ActionKind::Chain { steps } = kind {
        for step in steps {
            match step {
                ChainStep::Ref(id) => out.push(id),
                ChainStep::Inline(inner) => collect_refs(inner, out),
            }
        }
    }
}

fn validate_kind(kind: &ActionKind, prompt_ok: PromptCheck<'_>) -> Result<(), String> {
    match kind {
        ActionKind::Run { command, cwd, .. } => {
            if command.trim().is_empty() {
                return Err("\"command\" must not be empty".into());
            }
            if let Some(cwd) = cwd {
                check_relative(cwd).map_err(|m| format!("\"cwd\" {m}"))?;
            }
        }
        ActionKind::Prompt { text, file, .. } => match (text, file) {
            (Some(_), Some(_)) | (None, None) => {
                return Err("prompt actions need exactly one of \"text\" or \"file\"".into());
            }
            (None, Some(file)) => {
                check_relative(file).map_err(|m| format!("\"file\" {m}"))?;
                prompt_ok(file).map_err(|why| format!("prompt file prompts/{file} {why}"))?;
            }
            (Some(_), None) => {}
        },
        ActionKind::Open { url } => {
            // Only web links: `file:` or custom schemes could launch arbitrary local apps.
            if !(url.starts_with("https://") || url.starts_with("http://")) {
                return Err(format!("\"url\" must start with http:// or https://, got {url:?}"));
            }
        }
        ActionKind::Chain { steps } => {
            if steps.is_empty() {
                return Err("\"steps\" must not be empty".into());
            }
            for step in steps {
                if let ChainStep::Inline(inner) = step {
                    validate_kind(inner, prompt_ok)?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::tests::{project, strings};
    use super::super::{ACTIONS_FILE, ProjectError};
    use super::*;

    fn load(text: &str) -> Result<Option<Vec<Action>>, ProjectError> {
        let (_tmp, p) = project(&[(ACTIONS_FILE, text), ("prompts/review.md", "# Review\nGo")]);
        p.actions()
    }

    fn invalid_message(text: &str) -> String {
        match load(text) {
            Err(ProjectError::Invalid { message, .. }) => message,
            other => panic!("expected invalid, got {other:?}"),
        }
    }

    #[test]
    fn parses_all_kinds() {
        let text = r#"[
            {"id": "test", "title": "Test", "key": "cmd-shift-y", "type": "run", "command": "cargo test"},
            {"id": "web", "title": "Web", "type": "run", "command": "npm run dev",
             "target": "new_session", "cwd": "frontend"},
            {"id": "review", "title": "Review", "type": "prompt", "file": "review.md",
             "target": "session:Claude", "submit": true},
            {"id": "hi", "title": "Hi", "type": "prompt", "text": "hello"},
            {"id": "docs", "title": "Docs", "description": "Open docs", "type": "open", "url": "https://x.dev"},
            {"id": "all", "title": "All", "type": "chain",
             "steps": ["test", {"type": "open", "url": "https://ci.dev"}]},
        ]"#;
        let actions = load(text).expect("ok").expect("present");
        assert_eq!(actions.len(), 6);
        assert_eq!(actions[0].key.as_deref(), Some("cmd-shift-y"));
        assert_eq!(
            actions[1].kind,
            ActionKind::Run {
                command: "npm run dev".into(),
                target: Target::NewSession,
                cwd: Some("frontend".into())
            }
        );
        assert!(matches!(
            &actions[2].kind,
            ActionKind::Prompt { target: Target::Session(name), submit: true, .. } if name == "Claude"
        ));
        assert!(matches!(
            &actions[3].kind,
            ActionKind::Prompt { target: Target::Current, submit: false, .. }
        ));
        assert!(matches!(&actions[5].kind, ActionKind::Chain { steps } if steps.len() == 2
            && matches!(&steps[1], ChainStep::Inline(k) if matches!(**k, ActionKind::Open { .. }))));
        // Serialization round-trips through the same schema.
        let json = serde_json::to_string(&actions).expect("serialize");
        assert_eq!(serde_json::from_str::<Vec<Action>>(&json).expect("parse"), actions);
    }

    #[test]
    fn duplicate_ids() {
        let m = invalid_message(
            r#"[{"id": "a", "title": "A", "type": "open", "url": "https://a"},
                {"id": "a", "title": "B", "type": "open", "url": "https://b"}]"#,
        );
        assert!(m.contains("duplicate id \"a\""), "{m}");
    }

    #[test]
    fn missing_chain_reference() {
        let m = invalid_message(
            r#"[{"id": "c", "title": "C", "type": "chain",
                 "steps": [{"type": "chain", "steps": ["nope"]}]}]"#,
        );
        assert!(m.contains("unknown action \"nope\""), "{m}");
    }

    #[test]
    fn cycles() {
        let m = invalid_message(
            r#"[{"id": "a", "title": "A", "type": "chain", "steps": ["b"]},
                {"id": "b", "title": "B", "type": "chain", "steps": [{"type": "chain", "steps": ["c"]}]},
                {"id": "c", "title": "C", "type": "chain", "steps": ["a"]}]"#,
        );
        assert!(m.contains("a -> b -> c -> a"), "{m}");
        let m = invalid_message(r#"[{"id": "s", "title": "S", "type": "chain", "steps": ["s"]}]"#);
        assert!(m.contains("s -> s"), "{m}");
        // A diamond (shared step) is not a cycle.
        let ok = r#"[{"id": "a", "title": "A", "type": "chain", "steps": ["b", "c"]},
                     {"id": "b", "title": "B", "type": "chain", "steps": ["d"]},
                     {"id": "c", "title": "C", "type": "chain", "steps": ["d"]},
                     {"id": "d", "title": "D", "type": "run", "command": "true"}]"#;
        assert!(load(ok).is_ok());
    }

    #[test]
    fn kind_invariants() {
        for (bad, needle) in [
            (r#"[{"id": "", "title": "A", "type": "run", "command": "x"}]"#, "\"id\""),
            (r#"[{"id": "a", "title": " ", "type": "run", "command": "x"}]"#, "\"title\""),
            (r#"[{"id": "a", "title": "A", "type": "run", "command": ""}]"#, "\"command\""),
            (r#"[{"id": "a", "title": "A", "type": "run", "command": "x", "cwd": "../x"}]"#, "\"cwd\""),
            (r#"[{"id": "a", "title": "A", "type": "prompt"}]"#, "exactly one"),
            (
                r#"[{"id": "a", "title": "A", "type": "prompt", "text": "x", "file": "review.md"}]"#,
                "exactly one",
            ),
            (r#"[{"id": "a", "title": "A", "type": "prompt", "file": "missing.md"}]"#, "does not exist"),
            (r#"[{"id": "a", "title": "A", "type": "prompt", "file": "../x.md"}]"#, "\"file\""),
            (r#"[{"id": "a", "title": "A", "type": "open", "url": "file:///etc"}]"#, "\"url\""),
            (r#"[{"id": "a", "title": "A", "type": "chain", "steps": []}]"#, "\"steps\""),
        ] {
            let m = invalid_message(bad);
            assert!(m.contains(needle), "{bad}: {m}");
        }
    }

    #[test]
    fn syntax_errors_have_positions() {
        match load("[\n  {\"id\": \"a\", \"title\": \"A\", \"type\": \"fly\"}\n]") {
            Err(ProjectError::Parse { line, message, .. }) => {
                assert_eq!(line, 2);
                assert!(message.contains("fly"), "{message}");
            }
            other => panic!("expected parse error, got {other:?}"),
        }
        assert!(matches!(
            load(r#"[{"id": "a", "title": "A", "type": "run", "command": "x", "target": "session:"}]"#),
            Err(ProjectError::Parse { .. })
        ));
    }

    #[test]
    fn describe_lists_commands() {
        let kind = ActionKind::Chain {
            steps: vec![
                ChainStep::Ref("x".into()),
                ChainStep::Inline(Box::new(ActionKind::Run {
                    command: "make".into(),
                    target: Target::Current,
                    cwd: None,
                })),
            ],
        };
        let mut out = Vec::new();
        kind.describe(&mut out, "action c", &|_| None);
        assert_eq!(strings(&out), ["action c step 1: action `x`", "action c step 2: run `make` (current)"]);
    }

    #[test]
    fn describe_quotes_prompt_files_and_lists_them() {
        let prompt = |file: &str, submit| ActionKind::Prompt {
            text: None,
            file: Some(file.into()),
            target: Target::NewSession,
            submit,
        };
        let kind = ActionKind::Chain {
            steps: vec![
                ChainStep::Inline(Box::new(prompt("a.md", true))),
                ChainStep::Inline(Box::new(ActionKind::Chain {
                    steps: vec![ChainStep::Inline(Box::new(prompt("sub/b.md", false)))],
                })),
            ],
        };
        let long = "x".repeat(PROMPT_PREVIEW_CHARS + 5);
        let text = |file: &str| match file {
            "a.md" => Some("make deploy"),
            _ => Some(long.as_str()),
        };
        let mut out = Vec::new();
        kind.describe(&mut out, "action c", &text);
        let out = strings(&out);
        assert_eq!(out[0], "action c step 1: send prompt prompts/a.md `make deploy` (new_session)");
        assert!(
            out[1].starts_with("action c step 2 step 1: paste prompt prompts/sub/b.md `xxx"),
            "{}",
            out[1]
        );
        assert!(out[1].ends_with("… (5 more characters)` (new_session)"), "{}", out[1]);

        let action = Action { id: "c".into(), title: "C".into(), description: None, key: None, kind };
        let files: Vec<_> = prompt_files(std::slice::from_ref(&action)).into_iter().collect();
        assert_eq!(files, ["a.md", "sub/b.md"]);
    }

    /// A key spec was any string, checked only when a key was pressed, so a typo
    /// never fired and a binding could take over ⌘V or plain typing. Keys are checked at load;
    /// one that can't be used is dropped with a warning that says why, and the action
    /// stays, without it.
    #[test]
    fn keys_are_checked_at_load() {
        let with_key = |key: &str| {
            format!(r#"[{{"id": "a", "title": "A", "key": "{key}", "type": "run", "command": "make"}}]"#)
        };
        for good in ["cmd-shift-r", "ctrl-cmd-t", "cmd+shift+space", "alt-f5", "cmd-enter"] {
            let loaded = load(&with_key(good)).expect("loads").expect("present");
            assert!(loaded[0].key_spec().is_some(), "{good}");
        }
        for (bad, needle) in [
            ("cmd-shfit-r", "unknown modifier"),
            ("cmd-nosuchkey", "unknown key"),
            ("r", "needs ⌘"),
            ("shift-a", "needs ⌘"),
            ("cmd-v", "⌘V is Paste"),
            ("cmd-q", "⌘Q is Quit Wormhole"),
            ("cmd-p", "⌘P is kept for Print, as in Terminal.app"),
            ("cmd-", "key"),
        ] {
            let mut list: Vec<Action> = serde_json::from_str(&with_key(bad)).expect("parses");
            let warnings = validate(&mut list, &|_| Ok(())).expect("the file still loads");
            assert_eq!(list[0].key, None, "{bad}: dropped");
            let [ProjectWarning::ActionKey { action, key, why }] = warnings.as_slice() else {
                panic!("{bad}: {warnings:?}");
            };
            assert_eq!((action.as_str(), key.as_str()), ("a", bad));
            assert!(why.contains(needle), "{bad}: {why}");
            assert!(!why.contains("\"key\""), "{bad}: not the file's field, the Project tab's words: {why}");
        }
    }

    /// A project's key carries ⌘ or is an F-key; ⌃- and ⌥-letters belong to the shell, and
    /// a key the app or macOS already uses is refused with what uses it.
    #[test]
    fn project_keys_need_command_or_an_f_key_and_a_free_key() {
        for good in
            ["cmd-shift-r", "ctrl-cmd-t", "ctrl-cmd-l", "ctrl-cmd-o", "f5", "shift-f5", "alt-f5", "cmd-y"]
        {
            assert_eq!(validate_key(good), Ok(()), "{good}");
        }
        for (bad, why) in [
            ("ctrl-t", "needs ⌘"),
            ("alt-t", "needs ⌘"),
            ("ctrl-alt-t", "needs ⌘"),
            ("shift-t", "needs ⌘"),
            ("t", "needs ⌘"),
            ("cmd-t", "⌘T is New Terminal"),
            ("cmd-f", "⌘F is Find…"),
            ("ctrl-cmd-s", "⌃⌘S is Show / Hide Sidebar"),
            ("cmd-}", "⇧⌘] is Next Terminal"),
            ("f6", "F6 is Focus Next Area"),
            ("shift-cmd-p", "⇧⌘P is Command Palette…"),
            ("cmd-space", "macOS uses it for Spotlight"),
            ("ctrl-cmd-d", "macOS uses it for Look Up"),
        ] {
            let message = validate_key(bad).expect_err(bad);
            assert!(message.contains(why), "{bad}: {message}");
        }
    }
}
