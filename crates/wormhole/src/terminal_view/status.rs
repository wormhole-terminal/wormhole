//! A terminal's status (idle, busy, done, failed, needs you, exited) from its output,
//! its agent's hooks and what programs report about themselves (`reported`), what the rest of
//! the app shows about it, and when it redraws.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::agents::integrations::{self, Signal};
use wormhole_core::agents::{AgentId, Foreground};
use wormhole_core::notify::Source;
use wormhole_term::{Mark, Notification, PtyEvent, WindowOp};

use super::TerminalView;
use crate::settings::settings_in;

pub use wormhole_core::view::sidebar::Status;

/// What the rest of the app shows about a terminal (sidebar, title row, menu
/// bar). A copy, so readers don't have to hold the terminal during their render.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalSummary {
    pub status: Status,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub agent: Option<AgentId>,
    pub agent_message: Option<String>,
    /// What tells the status; a program that reports takes the row's agent-style second line.
    pub source: Source,
    /// The exit code of the last command, while it counts as failed (the row's cue).
    pub failed: Option<i32>,
}

/// A program's own notification (OSC 9/99/777), numbered so the notification sweep sees each
/// new one even when the terminal's state doesn't change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramNotice {
    pub serial: u64,
    pub notification: Notification,
}

/// Emitted by [`TerminalView`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    /// [`TerminalView::summary`] changed (only emitted when it did; output alone doesn't).
    SummaryChanged,
    /// The exited shell's bar asks for a new shell in the same tab.
    Restart,
    /// The exited shell's bar asks to close the tab.
    Close,
    /// The find bar opened or closed (the workspace draws it).
    FindChanged,
    /// A program asked the window to move, resize, minimise or the like (`allow_window_ops`).
    WindowOp(WindowOp),
    /// The session host it runs in changed: it fell back to the app's process, or the host went
    /// away. Its row is saved again, with the host it has now.
    HostChanged,
    /// The user cleared the terminal (⌘K): its saved text goes from the database too, at once.
    Cleared,
}

impl TerminalView {
    /// What the sidebar, tab strip and menu bar show about this terminal.
    pub fn summary(&self) -> TerminalSummary {
        TerminalSummary {
            status: self.status,
            title: self.title.clone(),
            cwd: self.cwd.clone(),
            agent: self.agent,
            agent_message: self.agent_message.clone(),
            source: self.source(),
            failed: self.failure.as_ref().map(|failure| failure.code),
        }
    }

    /// Emit `SummaryChanged` (and notify, for observers) if the summary moved
    /// since it was last announced. Returns whether it did.
    pub(super) fn announce_summary(&mut self, cx: &mut Context<Self>) -> bool {
        let summary = self.summary();
        if summary == self.announced {
            return false;
        }
        self.announced = summary;
        cx.emit(TerminalEvent::SummaryChanged);
        self.notify_redraw(cx);
        true
    }

    pub(super) fn notify_redraw(&mut self, cx: &mut Context<Self>) {
        self.redraw.notified();
        cx.notify();
    }

    /// After a batch of PTY events: redraw if we're on screen, tell the app if
    /// what it shows about us changed. Output in a hidden tab costs no frames.
    pub(super) fn after_events(&mut self, cx: &mut Context<Self>) {
        if !self.announce_summary(cx) && self.redraw.output_should_notify() {
            self.notify_redraw(cx);
        }
        self.find_output(cx);
    }

    pub fn status(&self) -> Status {
        self.status
    }

    /// When the status last changed.
    pub fn status_since(&self) -> Instant {
        self.status_since
    }

    pub(super) fn set_status(&mut self, status: Status) {
        if status != self.status {
            self.status = status;
            self.status_since = Instant::now();
            // Seen, or at work again: the finished command is history.
            if !matches!(status, Status::Done | Status::Failed) {
                self.finished_command = None;
            }
        }
    }

    /// Whether a program other than the shell held the terminal when last checked.
    pub fn program_in_foreground(&self) -> bool {
        self.program_in_foreground
    }

    /// The shell's process id while it runs: what a question about the processes under it (the
    /// ports they listen on) is asked with, on a thread of its own.
    pub fn shell_pid(&self) -> Option<u32> {
        self.pty.as_ref()?.shell_pid()
    }

    /// The program in the foreground and since when, as last checked; `None` at the prompt.
    pub fn foreground(&self) -> Option<&Foreground> {
        self.foreground.as_ref()
    }

    /// What the tab is named after: the program in front, or while a command's result is
    /// unseen (finished, ✓) that command.
    pub fn naming_command(&self) -> Option<&str> {
        match &self.foreground {
            Some(foreground) => Some(&foreground.command),
            None => self
                .finished_command
                .as_deref()
                .filter(|_| matches!(self.status, Status::Done | Status::Failed)),
        }
    }

    /// The bell rang (in a terminal without an agent) and nobody has looked since.
    pub fn bell_rang(&self) -> bool {
        self.bell_rang
    }

    /// The program's last notification of its own, numbered.
    pub fn notice(&self) -> Option<&ProgramNotice> {
        self.notice.as_ref()
    }

    /// Re-detect the foreground agent and command. Returns true when either changed.
    pub fn refresh_agent(&mut self) -> bool {
        let Some(pty) = &self.pty else { return false };
        let in_front = pty.program_in_foreground();
        let left = program_left_the_terminal(self.program_in_foreground, in_front);
        self.program_in_foreground = in_front == Some(true);
        let foreground = if self.program_in_foreground { pty.foreground_command() } else { None };
        let command_changed = foreground != self.foreground;
        if command_changed {
            let ended = std::mem::replace(&mut self.foreground, foreground);
            self.finished_command = match &self.foreground {
                None => ended.map(|f| f.command),
                Some(_) => None,
            };
        }
        let dropped = left && self.program_left();
        command_changed | dropped | self.refresh_foreground_agent()
    }

    fn refresh_foreground_agent(&mut self) -> bool {
        let Some(pty) = &self.pty else { return false };
        let agent = pty.foreground_agent();
        // An agent reporting through hooks, or a program saying what it is, is known even when its
        // process doesn't look like one (it may run over ssh); forget it only once the shell is
        // back.
        if agent == self.agent || (agent.is_none() && self.reported() && !pty.shell_in_foreground()) {
            return false;
        }
        // Hooks can't report an agent that has quit (or crashed): hand the
        // status back to the output heuristic.
        if agent.is_none() && self.hook_driven {
            self.hook_driven = false;
            self.agent_message = None;
            if matches!(self.status, Status::Busy | Status::Attention) {
                self.set_status(Status::Idle);
            }
        }
        self.agent = agent;
        true
    }

    /// Apply a status event from an agent's hooks. Returns the new status.
    pub fn apply_signal(&mut self, agent: AgentId, signal: Signal, message: Option<String>) -> Status {
        if self.status == Status::Exited {
            return self.status;
        }
        self.agent = Some(agent);
        self.hook_driven = signal != Signal::Ended;
        // The last word is the hook's: a program's records, if any, don't reset what it set.
        self.program_driven = false;
        // The prompt's own words ("Approve cargo test?") beat the notification that the same
        // prompt is still waiting, which comes seconds later.
        let asked = self.status == Status::Attention && self.agent_message.is_some();
        let status;
        (status, self.agent_message) = match signal {
            Signal::Busy => (Status::Busy, None),
            Signal::Attention if asked => (Status::Attention, self.agent_message.take()),
            Signal::Attention => (Status::Attention, message),
            Signal::Done => (Status::Done, None),
            Signal::Failed => (Status::Failed, message),
            Signal::Quiet | Signal::Ended => (Status::Idle, None),
        };
        self.set_status(status);
        self.status
    }

    /// A key typed while the agent's question is up: Return or a choice answers it, Esc turns
    /// it down. No hook says so until the tool has run, which can take minutes. Returns whether
    /// the status changed.
    pub(crate) fn answered(&mut self, key: &str, control: bool) -> bool {
        if !self.hook_driven || self.status != Status::Attention {
            return false;
        }
        let (Some(agent), Some(signal)) = (self.agent, integrations::answer_key(key, control)) else {
            return false;
        };
        self.apply_signal(agent, signal, None);
        true
    }

    /// Called periodically by the workspace to settle busy → idle/done/failed: once the output has
    /// been quiet for a while, or at once when the shell's marks said the command finished. Seen
    /// at once, there is nothing to settle into: the result is on screen (and a failure's cue stays
    /// on the row, see [`Failure`]).
    pub fn tick(&mut self, is_active: bool, window_active: bool) -> bool {
        if self.reported() {
            return false;
        }
        let finished = std::mem::take(&mut self.finished_mark);
        let looking = is_active && window_active;
        // `failure` is this command's when the mark just came: it replaced the last one's.
        let failed = finished && self.failure.is_some();
        if self.status == Status::Busy
            && (finished || self.last_output.elapsed() > Duration::from_millis(1800))
        {
            self.set_status(match (looking, failed) {
                (true, _) => Status::Idle,
                (false, true) => Status::Failed,
                (false, false) => Status::Done,
            });
            return true;
        }
        // A command that went quiet counts as done; its exit code, when it comes, can say it failed.
        if failed && !looking && self.status == Status::Done {
            self.set_status(Status::Failed);
            return true;
        }
        false
    }

    /// A shell with integration says exactly where it is (OSC 133): a command starts running
    /// (`C`, busy even before it prints), finishes (`D`, settled on the next tick), and the shell
    /// is back at its prompt, where output is prompts and echoes, not work.
    fn on_mark(&mut self, mark: Mark) {
        if self.hook_driven || self.status == Status::Exited {
            return;
        }
        // Where the shell is stays known while a program's reports decide the status (they end
        // with the program, and the heuristics then need to know whether it is at its prompt); the
        // reports' records themselves end with a prompt in the terminal's PTY thread.
        let heuristics = !self.reported();
        match mark {
            Mark::PromptStart(_) | Mark::CommandStart => self.at_prompt = Some(true),
            Mark::OutputStart => {
                self.at_prompt = Some(false);
                // The next command: the last one's failure is history.
                self.failure = None;
                if heuristics {
                    self.finished_mark = false;
                    self.last_output = Instant::now();
                    if self.status != Status::Attention {
                        self.set_status(Status::Busy);
                    }
                }
            }
            Mark::Finished(code) => {
                self.at_prompt = Some(true);
                self.finished_mark |= heuristics;
                // A program that reports itself says how it went (the status is its own); a
                // command's exit code is only ever a command's.
                if heuristics {
                    self.command_ended(code);
                }
            }
        }
    }

    /// A program's own notification (OSC 9/99/777) is the bell with words: the bell's rules say
    /// whether it counts as needing you (an agent's hooks already say when), and the sweep tells
    /// about it with the program's text (`Kind::Notice`).
    fn on_notice(&mut self, notification: Notification) {
        if self.reported() || self.status == Status::Exited {
            return;
        }
        let since = self.bell_counted.map(|at| at.elapsed());
        let bell = on_bell(self.status, self.reported(), self.agent.is_some(), since);
        self.bell_rang |= bell.mark;
        if bell.needs_you {
            self.bell_counted = Some(Instant::now());
            self.set_status(Status::Attention);
        }
        let serial = self.notice.as_ref().map_or(0, |n| n.serial) + 1;
        self.notice = Some(ProgramNotice { serial, notification });
    }

    /// A notification sent through the control socket (`Wormhole notify`): told in its words like
    /// a program's own (`Kind::Notice`), whatever the agent's hooks say, and without touching the
    /// status (the caller said something, it didn't ring to ask for you).
    pub fn notice_from_control(&mut self, notification: Notification) {
        let serial = self.notice.as_ref().map_or(0, |n| n.serial) + 1;
        self.notice = Some(ProgramNotice { serial, notification });
    }

    /// Clear "done"/"failed"/"attention" and the bell once the user looks at this terminal. Returns
    /// whether anything the sidebar shows changed (a bell that didn't count as "needs you" changes
    /// only the row's bell).
    pub fn acknowledge(&mut self) -> bool {
        let rang = std::mem::take(&mut self.bell_rang);
        // A pending question, from an agent's hooks or a program's own report, stays visible until
        // the program moves on.
        if (self.hook_driven || self.blocked_by_program()) && self.status == Status::Attention {
            return rang;
        }
        let seen = matches!(self.status, Status::Done | Status::Failed | Status::Attention);
        if seen {
            // What a program reported as finished has been seen: forgotten, so the next derivation
            // doesn't bring "done" back. What it still has going on is the status now, if it is
            // the program's status at all.
            self.acknowledge_reported();
            if !self.program_driven {
                self.set_status(Status::Idle);
            }
        }
        rang || seen
    }

    /// The bell, as the settings ask for it: the system's alert sound (`bell_sound`) and a brief
    /// glow along the pane's edge (`bell_visual`). A program ringing in a loop gets one
    /// sound per glow, not a rattle. The attention state the sidebar shows is separate.
    fn ring(&mut self, cx: &mut Context<Self>) {
        let (sound, visual) = {
            let settings = settings_in(&self.profile, cx);
            (settings.bell_sound, settings.bell_visual)
        };
        if sound && !self.bell_glow {
            crate::platform::sound::beep();
        }
        if !visual {
            return;
        }
        self.bell_glow = true;
        self.bell_glow_off = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(BELL_GLOW).await;
            this.update(cx, |this, cx| {
                this.bell_glow = false;
                this.notify_redraw(cx);
            })
            .ok();
        }));
        self.notify_redraw(cx);
    }

    /// Whether the visual bell's glow is showing.
    pub(super) fn bell_glow(&self) -> bool {
        self.bell_glow
    }

    /// The program's new title names the tab right away, unless the last rename was less than
    /// [`TITLE_EVERY`] ago: then the latest title waits out the rest, and the ones in between
    /// never show. A program counting its progress in the title renames at that pace, not per
    /// frame (every rename redraws the window, its sidebar and the tab row).
    fn set_title(&mut self, title: Option<String>, cx: &mut Context<Self>) {
        let wait = self.title_gate.offer(title, Instant::now());
        if let Some(title) = self.title_gate.take_due(Instant::now()) {
            self.title = title;
            return;
        }
        let Some(wait) = wait else { return };
        self.title_gate.flush = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| {
                this.title_gate.flush = None;
                // Offered again: it shows now, or waits once more if the timer came early.
                if let Some(title) = this.title_gate.pending.take() {
                    this.set_title(title, cx);
                    this.announce_summary(cx);
                }
            })
            .ok();
        }));
    }

    pub(super) fn handle_event(&mut self, event: PtyEvent, cx: &mut Context<Self>) {
        match event {
            PtyEvent::Output => {
                self.check_cwd_soon(cx);
                // Ignore what answers us, and whatever a shell with integration prints at its
                // prompt.
                let quiet = self.reported()
                    || self.at_prompt == Some(true)
                    || answers_us(Instant::now(), self.last_input, self.last_resize, self.last_focus_report);
                if !quiet && self.status != Status::Exited {
                    self.last_output = Instant::now();
                    if self.status != Status::Attention {
                        self.set_status(Status::Busy);
                    }
                }
            }
            PtyEvent::Title(title) => {
                self.set_title(wormhole_core::view::sidebar::names::title_name(&title), cx)
            }
            PtyEvent::ResetTitle => self.set_title(None, cx),
            // The shell said where it is: no need to wait for the output to settle, and a program
            // in the foreground (a nested shell) may be the one saying.
            PtyEvent::Cwd => {
                self.refresh_cwd();
            }
            PtyEvent::Mark(mark) => self.on_mark(mark),
            PtyEvent::Notify(notification) => self.on_notice(notification),
            PtyEvent::ProgramStatus => {
                self.sync_reported(true);
            }
            PtyEvent::Bell => {
                let since = self.bell_counted.map(|at| at.elapsed());
                let bell = on_bell(self.status, self.reported(), self.agent.is_some(), since);
                self.bell_rang |= bell.mark;
                if bell.needs_you {
                    self.bell_counted = Some(Instant::now());
                    self.set_status(Status::Attention);
                }
                self.ring(cx);
            }
            PtyEvent::ClipboardStore(text) => self.clipboard_store(text, cx),
            // Answered through `reply`, which keeps the answer in its place among the replies the
            // PTY thread gives (a cursor position asked for after it waits for it).
            PtyEvent::ClipboardLoad(format, reply) => self.clipboard_load(format, reply, cx),
            // The workspace owns the window.
            PtyEvent::WindowOp(op) => cx.emit(TerminalEvent::WindowOp(op)),
            // The screen stays: the last output (often why it exited) can still
            // be read, selected and saved.
            PtyEvent::Exited(code) => {
                self.exit_code = code;
                self.exit_was_host_lost(cx);
                self.set_status(Status::Exited);
            }
        }
    }
}

/// The shell has the terminal back after a program had it: that program's process is gone, and with
/// it whatever only it could say (a crash leaves no word of its own). `now` is `None` where the
/// platform can't tell, which says nothing either way.
fn program_left_the_terminal(was_in_front: bool, now: Option<bool>) -> bool {
    was_in_front && now == Some(false)
}

/// What a bell does to a terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bell {
    /// The row shows the bell until someone looks.
    mark: bool,
    /// The terminal needs you now.
    needs_you: bool,
}

/// How often a terminal without an agent may count as needing you for its bell.
const BELL_COUNTS_EVERY: Duration = Duration::from_secs(60);

/// What the bell does. A terminal whose status is told (`reported`: an agent's hooks, or a
/// program's own reports) ignores it: they say exactly when it needs you. An agent without hooks
/// rings to ask, so it needs you. A terminal without an agent marks the bell until you look, and
/// counts as needing you at most once a minute (`since_counted`: how long ago it last did), so a
/// program ringing in a loop doesn't keep asking. An ended shell ignores it.
fn on_bell(status: Status, reported: bool, agent: bool, since_counted: Option<Duration>) -> Bell {
    match (status, reported, agent) {
        (Status::Exited, _, _) | (_, true, _) => Bell { mark: false, needs_you: false },
        (_, false, true) => Bell { mark: false, needs_you: true },
        (_, false, false) => {
            Bell { mark: true, needs_you: since_counted.is_none_or(|since| since >= BELL_COUNTS_EVERY) }
        }
    }
}

/// How long the visual bell's glow lasts. One step, without a fade, so Reduce Motion
/// changes nothing about it.
const BELL_GLOW: Duration = Duration::from_millis(150);

/// How often the program's title may rename a tab.
pub(super) const TITLE_EVERY: Duration = Duration::from_millis(250);

/// Holds a program's title changes to one rename per [`TITLE_EVERY`]; the latest one wins.
#[derive(Default)]
pub(super) struct TitleGate {
    /// The title waiting to be shown, `Some(None)` for a reset.
    pub(super) pending: Option<Option<String>>,
    /// The title as last shown, so a change back to it is no change.
    shown: Option<String>,
    shown_at: Option<Instant>,
    /// The timer that shows `pending` once it is due.
    pub(super) flush: Option<Task<()>>,
}

impl TitleGate {
    /// A new title from the program. Returns how long until it may show, when it must wait and
    /// no timer is out for it yet.
    pub(super) fn offer(&mut self, title: Option<String>, now: Instant) -> Option<Duration> {
        if title == self.shown {
            self.pending = None;
            return None;
        }
        self.pending = Some(title);
        let wait = self.wait(now);
        (!wait.is_zero() && self.flush.is_none()).then_some(wait)
    }

    /// The title to show now, if one is waiting and the pace allows it.
    pub(super) fn take_due(&mut self, now: Instant) -> Option<Option<String>> {
        if self.pending.is_none() || !self.wait(now).is_zero() {
            return None;
        }
        let title = self.pending.take()?;
        self.shown.clone_from(&title);
        self.shown_at = Some(now);
        Some(title)
    }

    fn wait(&self, now: Instant) -> Duration {
        self.shown_at
            .map_or(Duration::ZERO, |at| TITLE_EVERY.saturating_sub(now.saturating_duration_since(at)))
    }
}

/// Whether output should redraw. Every read of a terminal during a window's
/// draw (the sidebar reads every tab's status) makes a notify of it redraw that
/// window, so a hidden tab that notified per output batch would redraw the
/// window at display rate. After a notify, output stays quiet until the view is
/// rendered again: until then the pending redraw already covers it, or the
/// view isn't on screen at all.
#[derive(Debug, Default)]
pub(super) struct RedrawGate {
    /// A notify is out that no render has picked up yet.
    pending: bool,
}

impl RedrawGate {
    pub(super) fn rendered(&mut self) {
        self.pending = false;
    }

    pub(super) fn notified(&mut self) {
        self.pending = true;
    }

    pub(super) fn output_should_notify(&self) -> bool {
        !self.pending
    }
}

/// Whether output arriving `now` answers something Wormhole did rather than being the program's
/// own work: the echo of typing, or the redraw after a resize or a focus report. An agent without
/// hooks redraws when Wormhole goes behind; counted as work, that settles into "done" and a
/// notification nothing finished for.
fn answers_us(now: Instant, input: Instant, resize: Instant, focus: Instant) -> bool {
    now.saturating_duration_since(input) < Duration::from_millis(300)
        || now.saturating_duration_since(resize) < Duration::from_millis(700)
        || now.saturating_duration_since(focus) < Duration::from_millis(700)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use std::time::Instant;

    use super::{
        Bell, RedrawGate, Status, TITLE_EVERY, TitleGate, answers_us, on_bell, program_left_the_terminal,
    };

    #[test]
    fn a_program_has_left_when_the_shell_has_the_terminal_back() {
        assert!(program_left_the_terminal(true, Some(false)));
        assert!(!program_left_the_terminal(true, Some(true)), "still there");
        assert!(!program_left_the_terminal(false, Some(false)), "never seen in front");
        assert!(!program_left_the_terminal(true, None), "the platform can't tell");
    }

    #[test]
    fn the_bell_asks_for_you_only_where_it_means_something() {
        let (quiet, ask, mark) = (
            Bell { mark: false, needs_you: false },
            Bell { mark: false, needs_you: true },
            Bell { mark: true, needs_you: true },
        );
        assert_eq!(
            on_bell(Status::Busy, true, true, None),
            quiet,
            "hooks or reports say when an agent needs you"
        );
        assert_eq!(on_bell(Status::Busy, false, true, None), ask, "an agent without hooks rings to ask");
        assert_eq!(
            on_bell(Status::Idle, false, false, None),
            mark,
            "a plain terminal: the bell, and needs you"
        );
        assert_eq!(on_bell(Status::Exited, false, false, None), quiet);
        // Again within the minute: marked, but it doesn't ask again.
        let soon = on_bell(Status::Idle, false, false, Some(Duration::from_secs(10)));
        assert_eq!(soon, Bell { mark: true, needs_you: false });
        assert_eq!(on_bell(Status::Idle, false, false, Some(Duration::from_secs(60))), mark);
    }

    /// Plays output batches and renders against the gate; returns the notifies.
    fn notifies(script: &str) -> String {
        let mut gate = RedrawGate::default();
        script
            .chars()
            .map(|step| match step {
                'o' if gate.output_should_notify() => {
                    gate.notified();
                    'N'
                }
                'o' => '.',
                'r' => {
                    gate.rendered();
                    'r'
                }
                _ => unreachable!(),
            })
            .collect()
    }

    #[test]
    fn output_on_screen_notifies_once_per_frame() {
        assert_eq!(notifies("ooorooro"), "N..rN.rN");
    }

    #[test]
    fn output_in_a_hidden_tab_notifies_once() {
        // Hidden: nothing renders it, however much it prints.
        assert_eq!(notifies("oooooooo"), "N.......");
        // Shown again: its next frame re-arms it.
        assert_eq!(notifies("oooroo"), "N..rN.");
    }

    #[test]
    fn a_title_that_keeps_changing_renames_the_tab_at_a_steady_pace() {
        let start = Instant::now();
        let mut gate = TitleGate::default();
        let title = |t: &str| Some(t.to_string());
        // The first title shows at once.
        assert_eq!(gate.offer(title("1%"), start), None);
        assert_eq!(gate.take_due(start), Some(title("1%")));
        // The next ones within the pace wait; only the first of them asks for a timer.
        let soon = start + Duration::from_millis(50);
        assert_eq!(gate.offer(title("2%"), soon), Some(TITLE_EVERY - Duration::from_millis(50)));
        assert_eq!(gate.take_due(soon), None);
        gate.flush = Some(gpui_kit::Task::ready(()));
        assert_eq!(gate.offer(title("3%"), soon), None, "a timer is out already");
        // When it is due, the latest shows; the ones in between never do.
        assert_eq!(gate.take_due(start + TITLE_EVERY), Some(title("3%")));
        // The same title again is no rename, and drops one waiting.
        gate.flush = None;
        let later = start + TITLE_EVERY + Duration::from_millis(10);
        assert!(gate.offer(title("4%"), later).is_some());
        assert_eq!(gate.offer(title("3%"), later), None);
        assert_eq!(gate.take_due(later + TITLE_EVERY), None);
        // A reset is a change like any other.
        assert_eq!(gate.offer(None, later + TITLE_EVERY), None);
        assert_eq!(gate.take_due(later + TITLE_EVERY), Some(None));
    }

    #[test]
    fn a_redraw_after_a_focus_report_is_not_work() {
        let long_ago = Instant::now();
        let at = |ms| long_ago + Duration::from_secs(60) + Duration::from_millis(ms);
        // Wormhole went behind and told the program; it redraws: no work, so no "done" later.
        assert!(answers_us(at(200), long_ago, long_ago, at(0)));
        assert!(answers_us(at(100), at(0), long_ago, long_ago), "the echo of typing");
        assert!(answers_us(at(500), long_ago, at(0), long_ago), "a resize's redraw");
        // Output well after any of them is the program's own.
        assert!(!answers_us(at(800), at(0), at(0), at(0)));
    }
}
