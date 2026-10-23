//! Running a project's actions (run, prompt, open, chain), its terminal layout and its prompts, and
//! matching action keys. What an action does to a terminal is the window's: this decides what is
//! allowed and says it as events.

use std::path::PathBuf;

use gpui_kit::*;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::project::{ActionKind, ChainStep, Feature, ProjectSnapshot, Target};
use wormhole_core::storage::paths::ProfileId;

use super::{Project, hub_of};
use crate::window::feature::{FeatureEvent, NewTerminal, TerminalInput, TerminalTarget};

/// Longest action chain we follow (chains are validated for cycles, this is a backstop).
const MAX_CHAIN_DEPTH: usize = 16;

impl Project {
    /// Run action `id` of the project terminal `session` is in, if the project is trusted and has
    /// Actions on.
    pub(super) fn run_action(&mut self, session: i64, profile: &ProfileId, id: &str, cx: &mut Context<Self>) {
        let Some(project) = self.project(session, profile, cx) else { return };
        if !hub_of(profile, cx).enabled(project, Feature::Actions) {
            return;
        }
        // The snapshot trust was just checked against: its actions and prompt texts are what run.
        let snapshot = project.snapshot.clone();
        let Some(action) = snapshot.actions().iter().find(|a| a.id == id) else { return };
        tracing::info!(action = %action.id, "running project action");
        run_kind(&snapshot, &action.kind, 0, cx);
    }

    /// `layout.json`: open the project's terminals (reusing ones with the same name).
    pub(super) fn open_layout(&mut self, session: i64, profile: &ProfileId, cx: &mut Context<Self>) {
        let Some(project) = self.project(session, profile, cx) else { return };
        if !hub_of(profile, cx).enabled(project, Feature::Layout) {
            return;
        }
        let (folder, layout) =
            (project.folder.clone(), project.snapshot.layout().cloned().unwrap_or_default());
        let terminals: Vec<NewTerminal> = layout
            .sessions
            .iter()
            .map(|session| NewTerminal {
                name: session.name.clone(),
                cwd: folder.resolve_cwd(session.cwd.as_deref()),
                run: session.run.clone(),
                focus: session.focus,
            })
            .collect();
        if !terminals.is_empty() {
            cx.emit(FeatureEvent::OpenTerminals { terminals });
        }
    }

    /// The id of the action of the project terminal `session` is in that `keystroke` is the key of, if the
    /// project is trusted and has Actions on.
    pub(super) fn action_for_key(
        &self,
        session: i64,
        profile: &ProfileId,
        keystroke: &Keystroke,
        cx: &App,
    ) -> Option<String> {
        let project = self.project(session, profile, cx)?;
        // The keys first: this runs for every ⌘ key typed, and asking whether the feature is on resolves
        // the project's folder on disk.
        let (_, id) = project.keys.iter().find(|(spec, _)| key_matches(spec, keystroke))?;
        hub_of(profile, cx).enabled(project, Feature::Actions).then(|| id.clone())
    }

    /// Paste prompt `id` into terminal `session`, with Enter after it when `submit`.
    pub(super) fn send_prompt(
        &mut self,
        session: i64,
        profile: &ProfileId,
        id: &str,
        submit: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(body) = self.enabled_prompt(session, profile, id, cx) else { return };
        cx.emit(FeatureEvent::SendText { session, text: body.trim().to_string().into(), submit });
    }

    /// The text of prompt `id` of the project terminal `session` is in, if the user has Prompts switched on
    /// for it (checked here, not only by hiding the buttons).
    pub(super) fn enabled_prompt(
        &self,
        session: i64,
        profile: &ProfileId,
        id: &str,
        cx: &App,
    ) -> Option<String> {
        let project = self.project(session, profile, cx)?;
        if !hub_of(profile, cx).enabled(project, Feature::Prompts) {
            return None;
        }
        project.snapshot.prompts().iter().find(|p| p.id == id).map(|p| p.body.clone())
    }
}

/// Where in the window an action's text goes, for the terminal to be found or made when it is typed.
fn terminal_target(target: &Target, cwd: Option<PathBuf>) -> TerminalTarget {
    match target {
        Target::Current => TerminalTarget::Active,
        Target::NewSession => TerminalTarget::New { cwd },
        Target::Session(name) => TerminalTarget::Named { name: name.clone(), cwd },
    }
}

fn run_kind(snapshot: &ProjectSnapshot, kind: &ActionKind, depth: usize, cx: &mut Context<Project>) {
    let folder = snapshot.folder();
    if depth > MAX_CHAIN_DEPTH {
        return;
    }
    match kind {
        ActionKind::Run { command, target, cwd } => {
            let cwd = folder.resolve_cwd(cwd.as_deref());
            cx.emit(FeatureEvent::Terminal {
                target: terminal_target(target, Some(cwd)),
                input: TerminalInput::Command(command.clone()),
            });
        }
        ActionKind::Prompt { text, file, target, submit } => {
            let body = match (text, file) {
                (Some(text), _) => Some(text.clone()),
                (None, Some(file)) => snapshot.action_prompt(file).map(str::to_string),
                (None, None) => None,
            };
            let Some(body) = body else { return };
            cx.emit(FeatureEvent::Terminal {
                target: terminal_target(target, Some(folder.root.clone())),
                input: TerminalInput::Paste { text: body, submit: *submit },
            });
        }
        ActionKind::Open { url } => open_url(url, cx),
        ActionKind::Chain { steps } => {
            for step in steps {
                match step {
                    ChainStep::Ref(id) => {
                        if let Some(action) = snapshot.actions().iter().find(|a| &a.id == id) {
                            run_kind(snapshot, &action.kind, depth + 1, cx);
                        }
                    }
                    ChainStep::Inline(kind) => run_kind(snapshot, kind, depth + 1, cx),
                }
            }
        }
    }
}

fn open_url(url: &str, cx: &App) {
    // Loaders only accept http(s) URLs; the system hands them to the default browser.
    cx.open_url(url);
}

/// Is this keystroke the key `spec` names? `fn` doesn't count, as before; a US keyboard's
/// shifted character is ⇧ and its key (`cmd-}` is `shift-cmd-]`).
fn key_matches(spec: &KeySpec, keystroke: &Keystroke) -> bool {
    spec.same_key(&crate::terminal_view::keystroke_spec(keystroke))
}

#[cfg(test)]
mod tests {
    use super::key_matches;
    use gpui_kit::Keystroke;
    use wormhole_core::keys::keyspec::KeySpec;

    fn spec(text: &str) -> KeySpec {
        text.parse().expect("valid spec")
    }

    #[test]
    fn matches_key_specs() {
        let ks = Keystroke::parse("cmd-shift-r").expect("valid keystroke");
        assert!(key_matches(&spec("cmd-shift-r"), &ks));
        assert!(key_matches(&spec("Command-Shift-R"), &ks));
        assert!(!key_matches(&spec("cmd-r"), &ks));
        assert!(!key_matches(&spec("ctrl-shift-r"), &ks));
        // The minus key couldn't be bound before (the spec was split on every "-").
        assert!(key_matches(&spec("cmd--"), &Keystroke::parse("cmd--").expect("valid keystroke")));
        assert!(key_matches(&spec("alt-return"), &Keystroke::parse("alt-enter").expect("valid keystroke")));
        // A US keyboard reports ⇧⌘] as ⌘}: the key as written still matches.
        assert!(key_matches(&spec("shift-cmd-]"), &Keystroke::parse("cmd-}").expect("valid keystroke")));
    }
}
