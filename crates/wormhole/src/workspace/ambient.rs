//! Every terminal of every window as the ambient surfaces see it: the notifications, the status
//! item, the Dock and the quick panel all start from [`all_tabs`] / [`terminals`], so they agree
//! on who needs you. Also ⌥⌘J, which goes to whoever has waited longest. A "tab" here
//! is one terminal (a row of the sidebar), named by the id its hooks report.

use std::time::{Instant, SystemTime};

use wormhole_core::agents::Foreground;
use wormhole_core::notify::{self, State};
use wormhole_core::sidebar::command::ends_by_itself;

use super::app::{app, open_tab};
use super::*;
use crate::terminal_view::ProgramNotice;

/// One terminal of one window, as read on this tick.
#[derive(Clone, Debug)]
pub(crate) struct TabFacts {
    /// The terminal's id as its session has it, and what its hooks report: the same number.
    pub session: i64,
    pub tab: i64,
    /// The terminal's name, as the sidebar shows it.
    pub name: String,
    pub state: State,
    /// When it started doing that (a plain terminal's command: when the command started).
    pub since: Instant,
    pub agent: Option<AgentId>,
    pub message: Option<String>,
    /// The program's last notification of its own (OSC 9/99/777), numbered.
    pub notice: Option<ProgramNotice>,
    pub foreground: Option<Foreground>,
    pub folder: Option<String>,
    pub branch: Option<String>,
    pub changed: Option<usize>,
    /// On screen in its window: the selected terminal, or a pane shown with it in its split.
    pub selected: bool,
    /// Its window is the key window.
    pub key: bool,
}

impl TabFacts {
    /// The user is looking at it: on screen in the key window while Wormhole is in front.
    pub(crate) fn looking(&self, app_active: bool) -> bool {
        self.selected && self.key && app_active
    }

    pub(crate) fn agent_name(&self) -> Option<&'static str> {
        self.agent.map(|id| wormhole_core::agents::agent(id).name)
    }
}

/// What a tab is doing, as the ambient surfaces count it. A command in a terminal without an
/// agent counts as work while it runs, even when it prints nothing (a long build), unless it is
/// a program you sit in (vim, ssh), whose terminal says what its output says.
pub(crate) fn state_of(status: Status, agent: bool, foreground: Option<&Foreground>) -> State {
    match status {
        Status::Attention => State::NeedsYou,
        Status::Exited => State::Exited,
        _ if !agent && foreground.is_some_and(|f| ends_by_itself(&f.command)) => State::Working,
        Status::Busy => State::Working,
        Status::Done => State::Done,
        Status::Idle => State::Idle,
    }
}

/// `started` (wall clock) as an instant, for times shown beside `Instant`s.
fn instant_of(started: SystemTime) -> Instant {
    let elapsed = SystemTime::now().duration_since(started).unwrap_or_default();
    Instant::now().checked_sub(elapsed).unwrap_or_else(Instant::now)
}

impl Workspace {
    /// Every terminal of this window (`key`: whether the window is the key window).
    pub(crate) fn tab_facts(&self, key: bool, cx: &App) -> Vec<TabFacts> {
        let mut out = Vec::new();
        for (ix, session) in self.sessions.iter().enumerate() {
            let name = session.name(cx).to_string();
            let status = session.git.status.as_ref();
            let terminal = session.terminal().read(cx);
            let foreground = terminal.foreground().cloned();
            let state = state_of(terminal.status(), terminal.agent.is_some(), foreground.as_ref());
            let since = match (&foreground, state, terminal.agent) {
                (Some(f), State::Working, None) => instant_of(f.started),
                _ => terminal.status_since(),
            };
            // The status is the one read for the folder the terminal is in.
            let same_folder = session.git.cwd == terminal.cwd;
            out.push(TabFacts {
                session: session.id,
                tab: session.id,
                name,
                state,
                since,
                agent: terminal.agent,
                message: terminal.agent_message.clone(),
                notice: terminal.notice().cloned(),
                foreground,
                folder: terminal
                    .cwd
                    .as_deref()
                    .and_then(|cwd| cwd.file_name())
                    .map(|name| name.to_string_lossy().into_owned()),
                branch: status.filter(|_| same_folder).and_then(|g| g.branch.clone()),
                changed: status.filter(|_| same_folder).map(|g| g.files.len()),
                selected: self.on_screen(ix),
                key,
            });
        }
        out
    }
}

/// Every tab of every window, oldest window first. A window that is being updated right now
/// can't be read: call this outside any window's update (from the app's tick, or deferred).
pub(crate) fn all_tabs(cx: &App) -> Vec<TabFacts> {
    let key = cx.active_window().map(|w| w.window_id());
    let mut tabs = Vec::new();
    for window in &app(cx).windows {
        let is_key = key == Some(window.handle.window_id());
        if let Ok(facts) = window.workspace.read_with(cx, |ws, cx| ws.tab_facts(is_key, cx)) {
            tabs.extend(facts);
        }
    }
    tabs
}

/// Every sidebar terminal of every window, for the status item, the Dock and the quick panel.
pub(crate) fn terminals(cx: &App) -> Vec<notify::Terminal> {
    let tabs = all_tabs(cx);
    let finished = super::notifications::finished_commands(cx);
    tabs.iter().map(|t| terminal(t, finished.and_then(|f| f.get(&t.tab)))).collect()
}

/// A terminal as a listed one; `finished`: the last command that ended there, and how long
/// it ran.
pub(crate) fn terminal(
    facts: &TabFacts,
    finished: Option<&(String, std::time::Duration)>,
) -> notify::Terminal {
    let running = facts.foreground.as_ref().map(|f| f.command.clone());
    let (command, took) = match (running, facts.state, finished) {
        (Some(command), _, _) => (Some(command), None),
        (None, State::Done, Some((command, took))) => (Some(command.clone()), Some(*took)),
        _ => (None, None),
    };
    notify::Terminal {
        session: facts.session,
        tab: facts.tab,
        name: facts.name.clone(),
        state: facts.state,
        since: facts.since,
        agent: facts.agent_name().map(str::to_string),
        message: facts.message.clone(),
        command,
        took,
        changed: facts.changed,
        folder: facts.folder.clone(),
        branch: facts.branch.clone(),
    }
}

/// Where ⌥⌘J went last, so pressing it again goes on to the next waiting terminal.
#[derive(Default)]
struct Jumped(Option<i64>);

impl Global for Jumped {}

/// Where ⇧⌥⌘J went last.
#[derive(Default)]
struct JumpedFinished(Option<i64>);

impl Global for JumpedFinished {}

/// ⌥⌘J: raise the window of the terminal that has waited longest, in any window, and
/// select it; again, the next one. Nothing waiting: nothing happens. Runs outside any window's
/// update (defer it from a key handler).
pub(crate) fn jump_to_waiting(cx: &mut App) {
    let terminals = terminals(cx);
    let waiting: Vec<i64> =
        notify::sections(&terminals).needs_you.iter().map(|&ix| terminals[ix].tab).collect();
    let last = cx.default_global::<Jumped>().0;
    let Some(tab) = notify::next_waiting(&waiting, last) else { return };
    cx.default_global::<Jumped>().0 = Some(tab);
    crate::background::activate(cx);
    open_tab(tab, cx);
}

/// ⇧⌥⌘J: the terminal that finished last and hasn't been looked at, then on each press the one
/// that finished before it (looking at one clears its "finished", so the next press moves on).
pub(crate) fn jump_to_finished(cx: &mut App) {
    let terminals = terminals(cx);
    let finished: Vec<i64> =
        notify::sections(&terminals).finished.iter().map(|&ix| terminals[ix].tab).collect();
    let last = cx.default_global::<JumpedFinished>().0;
    let Some(tab) = notify::next_waiting(&finished, last) else { return };
    cx.default_global::<JumpedFinished>().0 = Some(tab);
    crate::background::activate(cx);
    open_tab(tab, cx);
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use wormhole_core::agents::Foreground;
    use wormhole_core::notify::State;

    use super::state_of;
    use crate::terminal_view::Status;

    fn running(command: &str) -> Foreground {
        Foreground { command: command.into(), started: SystemTime::now() - Duration::from_secs(5) }
    }

    #[test]
    fn a_plain_command_counts_as_work_while_it_runs() {
        let build = running("cargo build");
        assert_eq!(state_of(Status::Idle, false, Some(&build)), State::Working, "a quiet build");
        assert_eq!(state_of(Status::Done, false, Some(&build)), State::Working);
        assert_eq!(state_of(Status::Idle, false, Some(&running("vim notes.md"))), State::Idle);
        assert_eq!(state_of(Status::Idle, true, Some(&build)), State::Idle, "an agent says it itself");
        assert_eq!(state_of(Status::Attention, false, Some(&build)), State::NeedsYou, "the bell wins");
        assert_eq!(state_of(Status::Exited, false, None), State::Exited);
        assert_eq!(state_of(Status::Busy, false, None), State::Working);
        assert_eq!(state_of(Status::Done, true, None), State::Done);
    }
}
