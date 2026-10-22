//! The control protocol's app side (`wormhole_core::ipc::control`): what
//! `Wormhole list`, `status`, `notify`, `send-text` and `new-terminal` do here (and the first-run
//! welcome's requests go on to `welcome`).
//!
//! Requests come from the socket thread through `AppState`'s message task (`app::dispatch_hook`),
//! outside any window's update, so every window can be read and the one a request is about updated
//! through `AppState::with_workspace`. The rules (who may do what, the caps, the budgets) are
//! core's; this module finds the windows and terminals a request names and carries out what
//! passes. A terminal, its one tab and its pane are the same thing now (a row of the sidebar, named
//! by its id), so the protocol's `tab` and `pane` ids are the terminal's.

use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::ipc::control::{
    Action, ErrorCode, Limited, Listing, Opened, PROTOCOL_VERSION, PaneInfo, RateLimits, Reached, Reply,
    Request, SEND_TEXT_MAX_BYTES, SendTarget, StatusInfo, Summary, TabInfo, Target, TerminalInfo,
    TerminalState, WindowInfo, notice_text, send_text_check, writes_off,
};
use wormhole_term::Notification;

use crate::settings::settings;
use crate::terminal_view::TerminalView;
use crate::workspace::ambient::state_of;
use crate::workspace::app::{AppState, OpenWindow, app};
use crate::workspace::{Workspace, notifications};

/// Each caller's budgets, app-wide (a tab's requests count the same whichever window they reach).
#[derive(Default)]
struct Budgets(RateLimits);

impl Global for Budgets {}

/// Answer `request`. Never fails: whatever is wrong with it is said in the reply.
pub(crate) fn handle(request: Request, cx: &mut App) -> Reply {
    let from = request.from_tab();
    tracing::debug!(?from, action = request.action.name(), "control request");
    if request.version != PROTOCOL_VERSION {
        return Reply::error(
            ErrorCode::Version,
            format!(
                "this Wormhole speaks protocol {PROTOCOL_VERSION}; the request is version {}",
                request.version
            ),
        );
    }
    match request.action {
        Action::List => Reply::ok(listing(cx)),
        Action::Status { target } => status(target, from, cx),
        Action::Notify { target, title, body } => notify(target, &title, &body, from, cx),
        Action::SendText { target, text, submit } => send_text(target, &text, submit, from, cx),
        Action::NewTerminal { folder, command, worktree, window } => {
            new_terminal(folder, command, worktree, window, from, cx)
        }
        // Not behind `control_write`: the token the app put in the welcome terminal is the key.
        Action::Welcome { token, op } => crate::workspace::welcome::handle(token, op, from, cx),
    }
}

/// Every window with its terminals, oldest window first.
fn listing(cx: &App) -> Listing {
    let front = AppState::front_window(cx).map(|window| window.id);
    let windows = app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            let terminals = window.workspace.read_with(cx, |ws, cx| ws.control_terminals(cx)).ok()?;
            Some(WindowInfo { id: window.id, front: front == Some(window.id), terminals })
        })
        .collect();
    Listing { windows }
}

/// `status`: the target's terminal, the caller's own without one, the app's summary outside any tab.
fn status(target: Option<Target>, from: Option<i64>, cx: &App) -> Reply {
    let Some(target) = target.or(from.map(Target::Tab)) else {
        let listing = listing(cx);
        let states = listing.windows.iter().flat_map(|w| w.terminals.iter().map(|t| t.status));
        return Reply::ok(Summary::of(listing.windows.len(), states));
    };
    let Some(found) = find(target, cx) else { return not_found(target) };
    let terminal =
        found.window.workspace.read_with(cx, |ws, cx| ws.control_terminal(found.terminal, cx)).ok();
    match terminal.flatten() {
        Some(terminal) => Reply::ok(StatusInfo { window: found.window.id, terminal, tab: found.tab }),
        None => not_found(target),
    }
}

/// `notify`: a notification about the target's tab (the caller's own without one), in the
/// caller's words, delivered by the notification rules like a program's own.
fn notify(target: Option<Target>, title: &str, body: &str, from: Option<i64>, cx: &mut App) -> Reply {
    let Some(words) = notice_text(title, body) else {
        return Reply::error(ErrorCode::BadRequest, "nothing to say once the text is cleaned");
    };
    let Some(target) = target.or(from.map(Target::Tab)) else {
        return Reply::error(
            ErrorCode::BadRequest,
            "name a terminal (--terminal, --tab or --pane); inside one of Wormhole's terminals its own is the default",
        );
    };
    let Some(found) = find(target, cx) else { return not_found(target) };
    if let Err(reply) = budget(Limited::Notify, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let told = AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_notice(tab, words, cx));
    if told != Some(true) {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    }
    // At once rather than on the next tick; the window's update is over, so it can be read.
    notifications::sweep(cx);
    Reply::ok(Reached { tab })
}

/// `send-text`: type into the target's tab as a paste, where the rules allow it.
fn send_text(target: Target, text: &str, submit: bool, from: Option<i64>, cx: &mut App) -> Reply {
    // The switch before anything about the terminal, as the rules order it.
    let enabled = settings(cx).control_write;
    if !enabled {
        let error = writes_off();
        return Reply::error(error.code, error.message);
    }
    let Some(found) = find(target, cx) else { return not_found(target) };
    let state =
        found.window.workspace.read_with(cx, |ws, cx| ws.control_tab_state(found.tab, cx)).ok().flatten();
    let Some(state) = state else { return not_found(target) };
    if let Err(error) = send_text_check(enabled, text, from, SendTarget { tab: found.tab, state }) {
        return Reply::error(error.code, error.message);
    }
    if let Err(reply) = budget(Limited::SendText, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let text = text.trim_end_matches(['\n', '\r']).to_string();
    let typed =
        AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_send_text(tab, &text, submit, cx));
    if typed != Some(true) {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    }
    tracing::info!(tab, ?from, submit, bytes = text.len(), "control: text sent to a terminal");
    Reply::ok(Reached { tab })
}

/// `new-terminal`: a terminal in `folder` (or where the window's current one is), in `window`
/// (or the front one), running `command` in its shell when given.
fn new_terminal(
    folder: Option<String>,
    command: Option<String>,
    worktree: Option<String>,
    window: Option<i64>,
    from: Option<i64>,
    cx: &mut App,
) -> Reply {
    if !settings(cx).control_write {
        let error = writes_off();
        return Reply::error(error.code, error.message);
    }
    if let Some(folder) = &folder {
        let path = Path::new(folder);
        if !path.is_absolute() || !path.is_dir() {
            return Reply::error(ErrorCode::NotFound, format!("no folder at {folder}"));
        }
    }
    if let Some(command) = &command {
        if command.contains(['\n', '\r']) {
            return Reply::error(ErrorCode::BadRequest, "--command takes one line");
        }
        if command.len() > SEND_TEXT_MAX_BYTES {
            return Reply::error(
                ErrorCode::TooLarge,
                format!("--command takes at most {SEND_TEXT_MAX_BYTES} bytes"),
            );
        }
    }
    let target = match window {
        Some(id) => app(cx).windows.iter().find(|w| w.id == id).cloned(),
        None => AppState::front_window(cx),
    };
    let Some(target) = target else {
        return match window {
            Some(id) => Reply::error(ErrorCode::NotFound, format!("no window {id}")),
            None => Reply::error(ErrorCode::Failed, "no window is open"),
        };
    };
    if let Err(reply) = budget(Limited::NewTerminal, from, cx) {
        return reply;
    }
    // A worktree is never made on a script's say alone: the dialog asks the user, in the window.
    if let Some(branch) = worktree {
        let asked = AppState::with_workspace(&target, cx, |ws, window, cx| {
            ws.control_ask_worktree(folder, &branch, command, window, cx)
        });
        return match asked {
            Some(Ok(asked)) => Reply::ok(asked),
            Some(Err(error)) => Reply::error(error.code, error.message),
            None => Reply::error(ErrorCode::Failed, "the window could not be reached"),
        };
    }
    let opened = AppState::with_workspace(&target, cx, |ws, window, cx| {
        ws.control_new_terminal(folder, command.as_deref(), window, cx)
    });
    match opened {
        Some(opened) => {
            tracing::info!(terminal = opened.terminal, ?from, "control: a terminal was opened");
            Reply::ok(opened)
        }
        None => Reply::error(ErrorCode::Failed, "the window could not be reached"),
    }
}

/// Count a `kind` request of `from` against its budget, or say when to try again.
pub(super) fn budget(kind: Limited, from: Option<i64>, cx: &mut App) -> Result<(), Reply> {
    let now = Instant::now();
    let budgets = &mut cx.default_global::<Budgets>().0;
    if budgets.allow(kind, from, now) {
        return Ok(());
    }
    let wait = budgets.retry_after(kind, from, now).as_secs().max(1);
    Err(Reply::error(
        ErrorCode::RateLimited,
        format!("too many requests of this kind lately; try again in {wait}s"),
    ))
}

fn not_found(target: Target) -> Reply {
    Reply::error(ErrorCode::NotFound, format!("no {} {}", target.kind(), target.id()))
}

/// Where a target is: its window, its terminal (session) and the tab the request means.
pub(super) struct Found {
    pub(super) window: OpenWindow,
    terminal: i64,
    tab: i64,
}

pub(super) fn find(target: Target, cx: &App) -> Option<Found> {
    app(cx).windows.iter().find_map(|window| {
        let (terminal, tab) =
            window.workspace.read_with(cx, |ws, _| ws.control_find(target)).ok().flatten()?;
        Some(Found { window: window.clone(), terminal, tab })
    })
}

/// An agent's id as the hooks spell it (`claude`, `codex`).
fn agent_id(agent: AgentId) -> Option<String> {
    serde_json::to_value(agent).ok().and_then(|v| v.as_str().map(str::to_string))
}

impl Workspace {
    /// The terminal and tab `target` names in this window: one id, whichever way it is named
    /// (what its hooks report).
    fn control_find(&self, target: Target) -> Option<(i64, i64)> {
        let (Target::Terminal(id) | Target::Tab(id) | Target::Pane(id)) = target;
        self.find_session(id).map(|ix| (self.sessions[ix].id, id))
    }

    /// Every terminal of this window, in the sidebar's order.
    fn control_terminals(&self, cx: &App) -> Vec<TerminalInfo> {
        self.session_order(cx).into_iter().map(|ix| self.terminal_info(ix, cx)).collect()
    }

    fn control_terminal(&self, session: i64, cx: &App) -> Option<TerminalInfo> {
        let ix = self.sessions.iter().position(|s| s.id == session)?;
        Some(self.terminal_info(ix, cx))
    }

    fn control_tab_state(&self, tab: i64, cx: &App) -> Option<TerminalState> {
        Some(tab_state(self.pane_terminal(tab)?.read(cx)))
    }

    fn terminal_info(&self, ix: usize, cx: &App) -> TerminalInfo {
        let session = &self.sessions[ix];
        let terminal = session.terminal().read(cx);
        // The status is the one read for the folder the terminal is in.
        let repo = session.git.status.as_ref().filter(|_| session.git.cwd == terminal.cwd);
        let folder = terminal.cwd.as_deref().map(|p| p.to_string_lossy().into_owned());
        TerminalInfo {
            id: session.id,
            name: session.name(cx).to_string(),
            status: tab_state(terminal),
            agent: terminal.agent.and_then(agent_id),
            folder: folder.clone(),
            branch: repo.and_then(|g| g.branch.clone()),
            changed: repo.map(|g| g.files.len()),
            pinned: session.pinned,
            profile: {
                let profiles = &app(cx).profiles;
                (profiles.profiles().len() > 1)
                    .then(|| profiles.get(&self.profile).map(|p| p.name.clone()))
                    .flatten()
            },
            selected: ix == self.active,
            // The protocol still lists tabs: a terminal has the one, itself, with its one pane.
            tabs: vec![TabInfo {
                id: session.id,
                name: session.pane.label(cx).to_string(),
                status: tab_state(terminal),
                agent: terminal.agent.and_then(agent_id),
                message: terminal.agent_message.clone(),
                folder,
                shell: terminal.shell_name().to_string(),
                command: terminal.foreground().map(|f| f.command.clone()),
                since_seconds: terminal.status_since().elapsed().as_secs(),
                selected: true,
                panes: vec![PaneInfo { id: session.id }],
            }],
        }
    }

    /// `notify`'s words into `tab`'s terminal, for the notification sweep to tell.
    fn control_notice(&mut self, tab: i64, words: Notification, cx: &mut Context<Self>) -> bool {
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        terminal.update(cx, |t, _| t.notice_from_control(words));
        cx.notify();
        true
    }

    /// Paste `text` into `tab` and, with `submit`, press Return a moment later (the program reads
    /// the paste first, as `type_reply` does for a notification's Reply).
    pub(super) fn control_send_text(
        &mut self,
        tab: i64,
        text: &str,
        submit: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(terminal) = self.pane_terminal(tab) else { return false };
        terminal.update(cx, |t, _| t.paste(text));
        if submit {
            cx.spawn(async move |_, cx| {
                cx.background_executor().timer(Duration::from_millis(60)).await;
                terminal.update(cx, |t, _| t.write(b"\r".to_vec()));
            })
            .detach();
        }
        true
    }

    /// A new terminal in `folder` (where the current one is without), selected, with `command`
    /// typed into its shell and run.
    pub(super) fn control_new_terminal(
        &mut self,
        folder: Option<String>,
        command: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Opened {
        let cwd = folder.or_else(|| self.default_cwd(cx));
        let ix = self.open_session(None, cwd.clone(), window, cx);
        let terminal = self.sessions[ix].id;
        let tab = terminal;
        if let Some(command) = command.map(str::trim).filter(|c| !c.is_empty()) {
            self.control_send_text(tab, command, true, cx);
        }
        Opened { window: self.window_id, terminal, tab, pane: tab, folder: cwd, worktree: None }
    }
}

/// A terminal's state as the sidebar counts it.
fn tab_state(terminal: &TerminalView) -> TerminalState {
    state_of(terminal.status(), terminal.source().told(), terminal.foreground()).into()
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;

    use super::agent_id;

    #[test]
    fn agents_are_named_as_their_hooks_name_them() {
        assert_eq!(agent_id(AgentId::Claude).as_deref(), Some("claude"));
        assert_eq!(agent_id(AgentId::OpenCode).as_deref(), Some("opencode"));
    }
}
