//! What an action, a prompt or the layout does, as data for the host to perform: the rules are core's
//! (an action's kind, its chain, its target) and the decision to allow it is the hub's `enabled`, made
//! before any of this runs. Rust never types into a terminal itself; it says what to type and where, and
//! only for what trust allowed.

use std::path::PathBuf;

use wormhole_core::project::{ActionKind, ChainStep, ProjectFolder, ProjectSnapshot, Target};

/// Longest action chain followed (chains are validated for cycles; this is a backstop).
const MAX_CHAIN_DEPTH: usize = 16;

/// Which terminal an effect is for.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum TerminalPlace {
    /// The one on screen.
    Current,
    /// A new one, in `cwd`, selected.
    New { cwd: Option<String> },
    /// The one called `name`, made in `cwd` if there is none, and selected.
    Named { name: String, cwd: Option<String> },
}

/// A terminal the layout opens.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct NewTerminalRecord {
    pub name: String,
    pub cwd: String,
    /// Typed in once it starts, then Enter.
    pub run: Option<String>,
    /// Selected at the end, of the ones opened.
    pub focus: bool,
}

/// One thing for the host to do, in order.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum ProjectEffect {
    /// Type `command` into the terminal, then Enter.
    Command { place: TerminalPlace, command: String },
    /// Paste `text`; Enter a moment later with `submit`.
    Paste { place: TerminalPlace, text: String, submit: bool },
    /// Hand an http(s) URL (the loaders accept no other) to the default browser.
    OpenUrl { url: String },
    /// Open these terminals (reusing ones with the same name).
    OpenTerminals { terminals: Vec<NewTerminalRecord> },
}

fn text(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn place(target: &Target, cwd: Option<PathBuf>) -> TerminalPlace {
    let cwd = cwd.map(text);
    match target {
        Target::Current => TerminalPlace::Current,
        Target::NewSession => TerminalPlace::New { cwd },
        Target::Session(name) => TerminalPlace::Named { name: name.clone(), cwd },
    }
}

/// The effects of action `id` of `snapshot`, chains followed; empty when there is no such action.
pub(super) fn of_action(snapshot: &ProjectSnapshot, id: &str) -> Vec<ProjectEffect> {
    let mut effects = Vec::new();
    if let Some(action) = snapshot.actions().iter().find(|a| a.id == id) {
        tracing::info!(action = %action.id, "running project action");
        push_kind(snapshot, &action.kind, 0, &mut effects);
    }
    effects
}

fn push_kind(snapshot: &ProjectSnapshot, kind: &ActionKind, depth: usize, out: &mut Vec<ProjectEffect>) {
    let folder = snapshot.folder();
    if depth > MAX_CHAIN_DEPTH {
        return;
    }
    match kind {
        ActionKind::Run { command, target, cwd } => {
            let cwd = folder.resolve_cwd(cwd.as_deref());
            out.push(ProjectEffect::Command { place: place(target, Some(cwd)), command: command.clone() });
        }
        ActionKind::Prompt { text, file, target, submit } => {
            let body = match (text, file) {
                (Some(text), _) => Some(text.clone()),
                // The bytes trust was checked against, not whatever the file holds by now.
                (None, Some(file)) => snapshot.action_prompt(file).map(str::to_string),
                (None, None) => None,
            };
            if let Some(body) = body {
                out.push(ProjectEffect::Paste {
                    place: place(target, Some(folder.root.clone())),
                    text: body,
                    submit: *submit,
                });
            }
        }
        ActionKind::Open { url } => out.push(ProjectEffect::OpenUrl { url: url.clone() }),
        ActionKind::Chain { steps } => {
            for step in steps {
                match step {
                    ChainStep::Ref(id) => {
                        if let Some(action) = snapshot.actions().iter().find(|a| &a.id == id) {
                            push_kind(snapshot, &action.kind, depth + 1, out);
                        }
                    }
                    ChainStep::Inline(kind) => push_kind(snapshot, kind, depth + 1, out),
                }
            }
        }
    }
}

/// The terminals `layout.json` opens, none when it has none.
pub(super) fn of_layout(folder: &ProjectFolder, snapshot: &ProjectSnapshot) -> Vec<ProjectEffect> {
    let Some(layout) = snapshot.layout().filter(|layout| !layout.sessions.is_empty()) else {
        return Vec::new();
    };
    let terminals = layout
        .sessions
        .iter()
        .map(|session| NewTerminalRecord {
            name: session.name.clone(),
            cwd: text(folder.resolve_cwd(session.cwd.as_deref())),
            run: session.run.clone(),
            focus: session.focus,
        })
        .collect();
    vec![ProjectEffect::OpenTerminals { terminals }]
}

/// A prompt's text pasted into the current terminal (not submitted: the person reviews it first).
pub(super) fn of_prompt(body: &str) -> Vec<ProjectEffect> {
    vec![ProjectEffect::Paste { place: TerminalPlace::Current, text: body.trim().to_string(), submit: false }]
}
