//! Restarting to use an installed update without ending work unasked. Restart looks at what runs
//! in every window by the rule for quitting: nothing, and the app restarts; something, and a sheet
//! lists it and offers to wait. "Restart When Idle" is remembered in `Updates` and settled from the
//! app's tick, once every terminal has been idle for a while (`IDLE_FOR`).

use std::rc::Rc;

use wormhole_core::agents::agent;

use super::session::{Session, shortened};
use super::updates::{self, UpdatePhase};
use super::*;

/// How long every terminal must stay idle, without a break, before the app restarts by itself. An
/// agent is idle for a moment between its steps, so a single quiet look would restart under it.
const IDLE_FOR: Duration = Duration::from_secs(30);

/// How many terminals the question names; past one more than this it ends in "and N more".
const LISTED: usize = 6;

/// A terminal a restart would cut off, as the question names it.
pub(super) struct Running {
    pub name: String,
    /// The agent in front by its short name ("Claude").
    pub agent: Option<&'static str>,
    pub status: Status,
    /// The program in front, as the terminal's row names it ("npm run dev").
    pub program: Option<String>,
}

impl Running {
    fn of(session: &Session, cx: &App) -> Self {
        let terminal = session.terminal().read(cx);
        Self {
            name: session.name(cx).to_string(),
            agent: terminal.agent.map(|id| agent(id).short_name()),
            status: terminal.status(),
            program: terminal.foreground().map(|foreground| foreground.command.clone()),
        }
    }

    /// What runs in it: the agent at work or waiting for an answer, else its program.
    fn what(&self) -> String {
        match (self.agent, self.status) {
            (Some(agent), Status::Busy) => format!("{agent} is working"),
            (Some(agent), Status::Attention) => format!("{agent} is waiting for an answer"),
            _ => match &self.program {
                Some(program) => shortened(program),
                None => match self.status {
                    Status::Busy => "still working",
                    Status::Attention => "waiting for you",
                    _ => "a command is running",
                }
                .to_string(),
            },
        }
    }

    /// "api — Claude is working", or just that when the terminal is named after what runs in it.
    fn line(&self) -> String {
        let (name, what) = (shortened(&self.name), self.what());
        if repeats(&name, &what) { what } else { format!("{name} — {what}") }
    }
}

/// Whether `what` already starts with the terminal's `name`, as a word ("Claude" and "Claude is
/// working"; a terminal is named after what runs in it unless it has a name of its own).
fn repeats(name: &str, what: &str) -> bool {
    let (name, what) = (name.to_lowercase(), what.to_lowercase());
    what == name || what.strip_prefix(&name).is_some_and(|rest| rest.starts_with(' '))
}

/// What the question before restarting says: the terminals that would be cut off, one per line, and
/// that they come back. A long list ends in "and N more".
pub(super) fn restart_detail(running: &[Running]) -> String {
    let shown = if running.len() > LISTED + 1 { LISTED } else { running.len() };
    let mut lines: Vec<String> = running.iter().take(shown).map(Running::line).collect();
    if shown < running.len() {
        lines.push(format!("and {} more", running.len() - shown));
    }
    let note = "Your terminals come back after the restart, but what runs in them ends.";
    if lines.is_empty() { note.to_string() } else { format!("{}\n\n{note}", lines.join("\n")) }
}

/// Whether a terminal lets "Restart When Idle" go ahead: no agent working or waiting for an
/// answer, and no command of its own in front. An agent sitting at its prompt has its program in
/// front too (`command_running`, `agent_in_front`), but it is waiting for you, not working.
pub(super) fn is_idle(status: Status, command_running: bool, agent_in_front: bool) -> bool {
    match status {
        Status::Busy | Status::Attention => false,
        Status::Exited => true,
        Status::Idle | Status::Done => !command_running || agent_in_front,
    }
}

/// How long the terminals have been idle, as seen from the tick.
#[derive(Default)]
pub(crate) struct IdleWatch {
    /// When the current stretch of looks that found everything idle began.
    since: Option<Instant>,
}

impl IdleWatch {
    /// One look at the terminals. True once every look for [`IDLE_FOR`] found them idle: a single
    /// busy one starts the wait over.
    fn seen(&mut self, idle: bool, now: Instant) -> bool {
        if !idle {
            self.since = None;
            return false;
        }
        let since = *self.since.get_or_insert(now);
        now.saturating_duration_since(since) >= IDLE_FOR
    }
}

fn workspaces(cx: &App) -> Vec<Entity<Workspace>> {
    app(cx).windows.iter().filter_map(|w| w.workspace.upgrade()).collect()
}

/// The terminals in any window that ending would cut something off: the rule for closing one, and
/// for quitting.
fn running_terminals(cx: &App) -> Vec<Running> {
    let mut running = Vec::new();
    for workspace in workspaces(cx) {
        let sessions = &workspace.read(cx).sessions;
        running
            .extend(sessions.iter().filter(|s| s.needs_close_confirmation(cx)).map(|s| Running::of(s, cx)));
    }
    running
}

fn all_idle(cx: &App) -> bool {
    workspaces(cx).iter().all(|workspace| {
        workspace.read(cx).sessions.iter().all(|session| {
            let terminal = session.terminal().read(cx);
            is_idle(terminal.status(), terminal.program_in_foreground(), terminal.agent.is_some())
        })
    })
}

/// Restart (Settings → Updates): at once when nothing would be cut off, else a sheet on `window`
/// says what would be. Restart restarts, Restart When Idle waits for the terminals, Later leaves
/// everything as it was.
pub(crate) fn request_restart(window: &mut Window, cx: &mut App) {
    let running = running_terminals(cx);
    if running.is_empty() {
        updates::restart_to_update(cx);
        return;
    }
    let question = dialog::CloseQuestion {
        title: "Restart to Update?".into(),
        detail: restart_detail(&running).into(),
        confirm: "Restart",
    };
    let when_idle: dialog::OnConfirm = Rc::new(|_, cx| updates::restart_when_idle(cx));
    dialog::ask_with_cancel(
        window,
        cx,
        question,
        "Later",
        Some(("Restart When Idle", when_idle)),
        |_, cx| updates::restart_to_update(cx),
        |_| {},
    );
}

/// Every app tick: restart if "Restart When Idle" was chosen and the terminals have been idle long
/// enough. Costs nothing while no request is pending.
pub(super) fn tick(cx: &mut App) {
    tick_at(Instant::now(), cx);
}

/// [`tick`] at `now`, so a test can let the minutes pass.
pub(super) fn tick_at(now: Instant, cx: &mut App) {
    let updates = &app(cx).updates;
    if !updates.restart_pending() {
        return;
    }
    // The line in Settings only shows beside an installed update: without one the request would
    // restart invisibly.
    if !matches!(updates.phase, UpdatePhase::Installed(_)) {
        updates::cancel_restart_when_idle(cx);
        return;
    }
    let idle = all_idle(cx);
    let due = app_mut(cx).updates.restart_watch().is_some_and(|watch| watch.seen(idle, now));
    if due {
        tracing::info!("restarting to update: the terminals have been idle");
        updates::restart_to_update(cx);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{IDLE_FOR, IdleWatch, Running, is_idle, repeats, restart_detail};
    use crate::terminal_view::Status;

    fn running(name: &str, agent: Option<&'static str>, status: Status, program: Option<&str>) -> Running {
        Running { name: name.into(), agent, status, program: program.map(str::to_string) }
    }

    #[test]
    fn each_terminal_says_what_runs_in_it() {
        let api = running("api", Some("Claude"), Status::Busy, Some("claude"));
        assert_eq!(api.line(), "api — Claude is working");
        let web = running("web", None, Status::Idle, Some("npm run dev"));
        assert_eq!(web.line(), "web — npm run dev");
        let notes = running("notes", Some("Codex"), Status::Attention, Some("codex"));
        assert_eq!(notes.line(), "notes — Codex is waiting for an answer");
    }

    #[test]
    fn an_agent_at_its_prompt_is_named_by_its_program() {
        // Quitting asks about it too: the agent ends with the restart.
        let idle = running("api", Some("Claude"), Status::Idle, Some("claude"));
        assert_eq!(idle.line(), "api — claude");
    }

    #[test]
    fn a_terminal_with_nothing_to_name_still_says_why_it_is_listed() {
        assert_eq!(running("web", None, Status::Idle, None).line(), "web — a command is running");
        assert_eq!(running("web", None, Status::Busy, None).line(), "web — still working");
        assert_eq!(running("web", None, Status::Attention, None).line(), "web — waiting for you");
    }

    #[test]
    fn a_terminal_named_after_what_runs_in_it_is_not_said_twice() {
        assert_eq!(running("npm run dev", None, Status::Idle, Some("npm run dev")).line(), "npm run dev");
        assert_eq!(
            running("Claude", Some("Claude"), Status::Busy, Some("claude")).line(),
            "Claude is working"
        );
        // A name that only begins like it is a name of its own.
        assert!(!repeats("web", "webpack"));
        assert!(repeats("web", "web dev server"));
    }

    #[test]
    fn long_names_and_programs_are_cut_short() {
        let long = "x".repeat(60);
        let line = running(&long, None, Status::Idle, Some(&long)).line();
        assert_eq!(line, format!("{0}…", "x".repeat(40)));
        let line = running(&long, None, Status::Idle, Some("make")).line();
        assert_eq!(line, format!("{}… — make", "x".repeat(40)));
    }

    #[test]
    fn the_question_lists_the_terminals_and_says_they_come_back() {
        let list = [
            running("api", Some("Claude"), Status::Busy, Some("claude")),
            running("web", None, Status::Idle, Some("npm run dev")),
        ];
        assert_eq!(
            restart_detail(&list),
            "api — Claude is working\nweb — npm run dev\n\n\
             Your terminals come back after the restart, but what runs in them ends."
        );
    }

    #[test]
    fn a_long_list_ends_in_how_many_more() {
        let many = |n: usize| -> Vec<Running> {
            (1..=n).map(|i| running(&format!("t{i}"), None, Status::Idle, Some("make"))).collect()
        };
        // One more than is listed still fits: "and 1 more" would be no shorter.
        let seven = restart_detail(&many(7));
        assert!(seven.contains("t7 — make") && !seven.contains("more"), "{seven}");
        let nine = restart_detail(&many(9));
        assert!(nine.contains("t6 — make\nand 3 more\n\n"), "{nine}");
        assert!(!nine.contains("t7"), "{nine}");
    }

    #[test]
    fn an_agent_is_working_or_waiting_not_idle() {
        assert!(!is_idle(Status::Busy, true, true), "an agent at work");
        assert!(!is_idle(Status::Attention, true, true), "an agent waiting for an answer");
        assert!(!is_idle(Status::Busy, false, false), "output still coming in");
        assert!(!is_idle(Status::Attention, false, false), "a bell nobody has answered");
    }

    #[test]
    fn a_command_in_front_is_not_idle_but_an_agent_at_its_prompt_is() {
        assert!(!is_idle(Status::Idle, true, false), "npm run dev");
        assert!(!is_idle(Status::Done, true, false), "a command that stopped printing");
        assert!(is_idle(Status::Idle, true, true), "claude, waiting at its prompt");
        assert!(is_idle(Status::Done, true, true), "codex, finished and unseen");
        assert!(is_idle(Status::Idle, false, false), "a shell at its prompt");
        assert!(is_idle(Status::Done, false, false));
        assert!(is_idle(Status::Exited, true, false), "nothing runs in an ended shell");
    }

    #[test]
    fn the_app_must_stay_idle_without_a_break() {
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        let mut watch = IdleWatch::default();
        assert!(!watch.seen(true, at(0)), "the wait starts at the first idle look");
        assert!(!watch.seen(true, at(IDLE_FOR.as_secs() - 1)));
        // An agent between two steps: busy for one look, and the wait starts over.
        assert!(!watch.seen(false, at(IDLE_FOR.as_secs())));
        assert!(!watch.seen(true, at(IDLE_FOR.as_secs() + 1)));
        assert!(!watch.seen(true, at(2 * IDLE_FOR.as_secs())));
        assert!(watch.seen(true, at(IDLE_FOR.as_secs() + 1 + IDLE_FOR.as_secs())));
    }
}
