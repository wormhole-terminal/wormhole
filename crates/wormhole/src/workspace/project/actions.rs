//! Running a project's actions (run, prompt, open, chain), its session layout and its prompts, and
//! matching action keys.

use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::keyspec::KeySpec;
use wormhole_core::project::{ActionKind, ChainStep, Feature, ProjectSnapshot, Target};
use wormhole_core::store::SessionRecord;

use crate::terminal_view::TerminalView;
use crate::workspace::Workspace;
use crate::workspace::app::app;

/// Longest action chain we follow (chains are validated for cycles, this is a backstop).
const MAX_CHAIN_DEPTH: usize = 16;

impl Workspace {
    pub(in crate::workspace) fn run_project_action(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.active_project(cx) else { return };
        if !app(cx).hub.enabled(project, Feature::Actions) {
            return;
        }
        // The snapshot trust was just checked against: its actions and prompt texts are what run.
        let snapshot = project.snapshot.clone();
        let Some(action) = snapshot.actions().iter().find(|a| a.id == id) else { return };
        tracing::info!(action = %action.id, "running project action");
        self.run_action_kind(&snapshot, &action.kind, 0, window, cx);
    }

    fn run_action_kind(
        &mut self,
        snapshot: &ProjectSnapshot,
        kind: &ActionKind,
        depth: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let folder = snapshot.folder();
        if depth > MAX_CHAIN_DEPTH {
            return;
        }
        match kind {
            ActionKind::Run { command, target, cwd } => {
                let cwd = folder.resolve_cwd(cwd.as_deref());
                let terminal = self.target_terminal(target, Some(cwd), window, cx);
                terminal.update(cx, |t, _| t.write_input(format!("{command}\r").into_bytes()));
            }
            ActionKind::Prompt { text, file, target, submit } => {
                let body = match (text, file) {
                    (Some(text), _) => Some(text.clone()),
                    (None, Some(file)) => snapshot.action_prompt(file).map(str::to_string),
                    (None, None) => None,
                };
                let Some(body) = body else { return };
                let terminal = self.target_terminal(target, Some(folder.root.clone()), window, cx);
                paste_into(&terminal, &body, *submit, cx);
            }
            ActionKind::Open { url } => open_url(url, cx),
            ActionKind::Chain { steps } => {
                for step in steps {
                    match step {
                        ChainStep::Ref(id) => {
                            if let Some(action) = snapshot.actions().iter().find(|a| &a.id == id) {
                                self.run_action_kind(snapshot, &action.kind, depth + 1, window, cx);
                            }
                        }
                        ChainStep::Inline(kind) => {
                            self.run_action_kind(snapshot, kind, depth + 1, window, cx)
                        }
                    }
                }
            }
        }
    }

    /// The session an action targets, creating it if needed.
    fn target_terminal(
        &mut self,
        target: &Target,
        cwd: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        match target {
            Target::Current => self.active_terminal(),
            Target::NewSession => {
                let ix = self.spawn_session(None, cwd, window, cx);
                self.sessions[ix].terminal().clone()
            }
            Target::Session(name) => {
                let existing = self.sessions.iter().position(|s| s.name(cx).as_ref() == name.as_str());
                let ix = match existing {
                    Some(ix) => ix,
                    None => self.spawn_session(Some(name.clone()), cwd, window, cx),
                };
                self.activate(ix, window, cx);
                self.sessions[ix].terminal().clone()
            }
        }
    }

    /// Open a new session (optionally named, in `cwd`) and focus it.
    pub(in crate::workspace) fn spawn_session(
        &mut self,
        name: Option<String>,
        cwd: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let id = self.alloc_id(cx);
        let record = SessionRecord::new(id, name, cwd.map(|p| p.to_string_lossy().into_owned()));
        let ix = self.add_session(Some(record), window, cx);
        self.activate(ix, window, cx);
        self.dirty = true;
        ix
    }

    /// `layout.json`: open the project's sessions (reusing ones with the same name).
    pub(in crate::workspace) fn open_project_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.active_project(cx) else { return };
        if !app(cx).hub.enabled(project, Feature::Layout) {
            return;
        }
        let (folder, layout) =
            (project.folder.clone(), project.snapshot.layout().cloned().unwrap_or_default());
        let mut focus = None;
        for session in &layout.sessions {
            if self.sessions.iter().any(|s| s.custom_name.as_deref() == Some(session.name.as_str())) {
                continue;
            }
            let cwd = folder.resolve_cwd(session.cwd.as_deref());
            let ix = self.spawn_session(Some(session.name.clone()), Some(cwd), window, cx);
            if let Some(run) = &session.run {
                self.sessions[ix]
                    .terminal()
                    .update(cx, |t, _| t.write_input(format!("{run}\r").into_bytes()));
            }
            if session.focus {
                focus = Some(ix);
            }
        }
        if let Some(ix) = focus {
            self.activate(ix, window, cx);
        }
    }

    /// Match a keystroke against the active project's action shortcuts.
    pub(in crate::workspace) fn project_action_for_key(
        &self,
        keystroke: &Keystroke,
        cx: &App,
    ) -> Option<String> {
        let project = self.active_project(cx)?;
        // The keys first: this runs for every ⌘ key typed, and asking whether the feature is
        // on resolves the project's folder on disk.
        let (_, id) = project.keys.iter().find(|(spec, _)| key_matches(spec, keystroke))?;
        app(cx).hub.enabled(project, Feature::Actions).then(|| id.clone())
    }

    pub(in crate::workspace) fn send_prompt(
        &mut self,
        id: &str,
        submit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(body) = self.enabled_prompt(id, cx) else { return };
        self.send_text(body, submit, window, cx);
    }

    pub(in crate::workspace) fn prompt_to_note(
        &mut self,
        id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(body) = self.enabled_prompt(id, cx) else { return };
        self.new_note(Some(body), window, cx);
    }

    /// The text of the active project's prompt `id`, if the user has Prompts switched on for it
    /// (checked here, not only by hiding the buttons).
    fn enabled_prompt(&self, id: &str, cx: &App) -> Option<String> {
        let project = self.active_project(cx)?;
        if !app(cx).hub.enabled(project, Feature::Prompts) {
            return None;
        }
        project.snapshot.prompts().iter().find(|p| p.id == id).map(|p| p.body.clone())
    }
}

fn paste_into(terminal: &Entity<TerminalView>, text: &str, submit: bool, cx: &mut App) {
    terminal.update(cx, |t, _| t.paste(text));
    if submit {
        let terminal = terminal.clone();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(Duration::from_millis(60)).await;
            terminal.update(cx, |t, _| t.write(b"\r".to_vec()));
        })
        .detach();
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
    use wormhole_core::keyspec::KeySpec;

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
