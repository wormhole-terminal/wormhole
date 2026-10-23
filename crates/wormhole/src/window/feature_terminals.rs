//! What the window does when a feature asks for terminals (`FeatureEvent::Terminal`,
//! `FeatureEvent::OpenTerminals`): finding or making the one asked for, typing or pasting into it, and
//! opening a set. A project's actions and layout use them, and any terminal opened in a folder.

use std::path::PathBuf;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::storage::store::SessionRecord;

use crate::terminal_view::TerminalView;
use crate::window::Workspace;
use crate::window::feature::{NewTerminal, TerminalInput, TerminalTarget};

/// The pause between a pasted text and the Enter that submits it.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

impl Workspace {
    /// Open a new session (optionally named, in `cwd`) and focus it.
    pub(crate) fn spawn_session(
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

    /// The terminal `target` names, made when there is none to find.
    fn target_terminal(
        &mut self,
        target: TerminalTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<TerminalView> {
        match target {
            TerminalTarget::Active => self.active_terminal(),
            TerminalTarget::New { cwd } => {
                let ix = self.spawn_session(None, cwd, window, cx);
                self.sessions[ix].terminal().clone()
            }
            TerminalTarget::Named { name, cwd } => {
                let existing = self.sessions.iter().position(|s| s.name(cx).as_ref() == name.as_str());
                let ix = match existing {
                    Some(ix) => ix,
                    None => self.spawn_session(Some(name), cwd, window, cx),
                };
                self.activate(ix, window, cx);
                self.sessions[ix].terminal().clone()
            }
        }
    }

    /// `input` into the terminal `target` names.
    pub(in crate::window) fn run_in_terminal(
        &mut self,
        target: TerminalTarget,
        input: TerminalInput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let terminal = self.target_terminal(target, window, cx);
        match input {
            TerminalInput::Command(command) => {
                terminal.update(cx, |t, _| t.write_input(format!("{command}\r").into_bytes()));
            }
            TerminalInput::Paste { text, submit } => {
                terminal.update(cx, |t, _| t.paste(&text));
                if submit {
                    let terminal = terminal.clone();
                    cx.spawn(async move |_, cx| {
                        cx.background_executor().timer(SUBMIT_DELAY).await;
                        terminal.update(cx, |t, _| t.write(b"\r".to_vec()));
                    })
                    .detach();
                }
            }
        }
    }

    /// Opens `terminals` one after the other, each selected as it is made, unless a terminal of that name
    /// is open (the ones just made count), and then selects the last that asks for it of those it opened.
    pub(in crate::window) fn open_terminals(
        &mut self,
        terminals: Vec<NewTerminal>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut focus = None;
        for NewTerminal { name, cwd, run, focus: wants_focus } in terminals {
            if self.sessions.iter().any(|s| s.custom_name.as_deref() == Some(name.as_str())) {
                continue;
            }
            let ix = self.spawn_session(Some(name), Some(cwd), window, cx);
            if let Some(run) = run {
                self.sessions[ix]
                    .terminal()
                    .update(cx, |t, _| t.write_input(format!("{run}\r").into_bytes()));
            }
            if wants_focus {
                focus = Some(ix);
            }
        }
        if let Some(ix) = focus {
            self.activate(ix, window, cx);
        }
    }
}
