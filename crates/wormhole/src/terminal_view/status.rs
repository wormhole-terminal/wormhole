//! A terminal's status (idle, busy, done, needs you, exited) from its output
//! and its agent's hooks, what the rest of the app shows about it, and when
//! it redraws.

use super::*;

/// A terminal's status (idle, busy, done, needs you, exited), from its output and its agent's hooks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Idle,
    Busy,
    /// Finished producing output while you weren't looking.
    Done,
    /// Rang the bell while you weren't looking, or its agent asks something.
    Attention,
    Exited,
}

/// What the rest of the app shows about a terminal (sidebar, title row, menu
/// bar). A copy, so readers don't have to hold the terminal during their render.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalSummary {
    pub status: Status,
    pub title: Option<String>,
    pub cwd: Option<PathBuf>,
    pub agent: Option<AgentId>,
    pub agent_message: Option<String>,
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

    fn set_status(&mut self, status: Status) {
        if status != self.status {
            self.status = status;
            self.status_since = Instant::now();
            // Seen, or at work again: the finished command is history.
            if status != Status::Done {
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
            None => self.finished_command.as_deref().filter(|_| self.status == Status::Done),
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
        self.program_in_foreground = pty.program_in_foreground() == Some(true);
        let foreground = if self.program_in_foreground { pty.foreground_command() } else { None };
        let command_changed = foreground != self.foreground;
        if command_changed {
            let ended = std::mem::replace(&mut self.foreground, foreground);
            self.finished_command = match &self.foreground {
                None => ended.map(|f| f.command),
                Some(_) => None,
            };
        }
        command_changed | self.refresh_foreground_agent()
    }

    fn refresh_foreground_agent(&mut self) -> bool {
        let Some(pty) = &self.pty else { return false };
        let agent = pty.foreground_agent();
        // An agent reporting through hooks is known even when its process
        // doesn't look like one; forget it only once the shell is back.
        if agent == self.agent || (agent.is_none() && self.hook_driven && !pty.shell_in_foreground()) {
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
        // The prompt's own words ("Approve cargo test?") beat the notification that the same
        // prompt is still waiting, which comes seconds later.
        let asked = self.status == Status::Attention && self.agent_message.is_some();
        let status;
        (status, self.agent_message) = match signal {
            Signal::Busy => (Status::Busy, None),
            Signal::Attention if asked => (Status::Attention, self.agent_message.take()),
            Signal::Attention => (Status::Attention, message),
            Signal::Done => (Status::Done, None),
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

    /// Called periodically by the workspace to settle busy → idle/done: once the output has been
    /// quiet for a while, or at once when the shell's marks said the command finished.
    pub fn tick(&mut self, is_active: bool, window_active: bool) -> bool {
        if self.hook_driven {
            return false;
        }
        let finished = std::mem::take(&mut self.finished_mark);
        if self.status == Status::Busy
            && (finished || self.last_output.elapsed() > Duration::from_millis(1800))
        {
            self.set_status(if is_active && window_active { Status::Idle } else { Status::Done });
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
        match mark {
            Mark::PromptStart(_) | Mark::CommandStart => self.at_prompt = Some(true),
            Mark::OutputStart => {
                self.at_prompt = Some(false);
                self.finished_mark = false;
                self.last_output = Instant::now();
                if self.status != Status::Attention {
                    self.set_status(Status::Busy);
                }
            }
            Mark::Finished(_) => {
                self.at_prompt = Some(true);
                self.finished_mark = true;
            }
        }
    }

    /// A program's own notification (OSC 9/99/777) is the bell with words: the bell's rules say
    /// whether it counts as needing you (an agent's hooks already say when), and the sweep tells
    /// about it with the program's text (`Kind::Notice`).
    fn on_notice(&mut self, notification: Notification) {
        if self.hook_driven || self.status == Status::Exited {
            return;
        }
        let since = self.bell_counted.map(|at| at.elapsed());
        let bell = on_bell(self.status, self.hook_driven, self.agent.is_some(), since);
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

    /// Clear "done"/"attention" and the bell once the user looks at this terminal. Returns
    /// whether anything the sidebar shows changed (a bell that didn't count as "needs you" changes
    /// only the row's bell).
    pub fn acknowledge(&mut self) -> bool {
        let rang = std::mem::take(&mut self.bell_rang);
        // A pending agent question stays visible until the agent moves on.
        if self.hook_driven && self.status == Status::Attention {
            return rang;
        }
        let seen = matches!(self.status, Status::Done | Status::Attention);
        if seen {
            self.set_status(Status::Idle);
        }
        rang || seen
    }

    /// The bell, as the settings ask for it: the system's alert sound (`bell_sound`) and a brief
    /// glow along the pane's edge (`bell_visual`). A program ringing in a loop gets one
    /// sound per glow, not a rattle. The attention state the sidebar shows is separate.
    fn ring(&mut self, cx: &mut Context<Self>) {
        let (sound, visual) = {
            let settings = settings(cx);
            (settings.bell_sound, settings.bell_visual)
        };
        if sound && !self.bell_glow {
            system_beep();
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
                // Ignore echo of our own typing and redraws caused by resizes, and whatever a
                // shell with integration prints at its prompt.
                let quiet = self.hook_driven
                    || self.at_prompt == Some(true)
                    || self.last_input.elapsed() < Duration::from_millis(300)
                    || self.last_resize.elapsed() < Duration::from_millis(700);
                if !quiet && self.status != Status::Exited {
                    self.last_output = Instant::now();
                    if self.status != Status::Attention {
                        self.set_status(Status::Busy);
                    }
                }
            }
            PtyEvent::Title(title) => self.set_title(wormhole_core::sidebar::names::title_name(&title), cx),
            PtyEvent::ResetTitle => self.set_title(None, cx),
            // The shell said where it is: no need to wait for the output to settle, and a program
            // in the foreground (a nested shell) may be the one saying.
            PtyEvent::Cwd => {
                self.refresh_cwd();
            }
            PtyEvent::Mark(mark) => self.on_mark(mark),
            PtyEvent::Notify(notification) => self.on_notice(notification),
            PtyEvent::Bell => {
                let since = self.bell_counted.map(|at| at.elapsed());
                let bell = on_bell(self.status, self.hook_driven, self.agent.is_some(), since);
                self.bell_rang |= bell.mark;
                if bell.needs_you {
                    self.bell_counted = Some(Instant::now());
                    self.set_status(Status::Attention);
                }
                self.ring(cx);
            }
            PtyEvent::ClipboardStore(text) => {
                // A copy on select takes the clipboard; keep what it held for ⌘V over the selection.
                let replaced = self
                    .app_selection
                    .copy_from_selection(Instant::now())
                    .then(|| cx.read_from_clipboard().and_then(|item| item.text()));
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                if let Some(replaced) = replaced {
                    self.app_selection.copied(text, replaced);
                }
            }
            // Answered through `reply`, which keeps the answer in its place among the replies the
            // PTY thread gives (a cursor position asked for after it waits for it).
            PtyEvent::ClipboardLoad(format, reply) => {
                let text = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
                reply.send(format(&text).into_bytes());
            }
            PtyEvent::ColorRequest(index, format, reply) => {
                let Some(pty) = &self.pty else { return };
                // What the program set (OSC 4/10/11/12) wins over the theme, as it does on screen.
                let color = cx.global::<SettingsStore>().palette.query(index, &pty.colors());
                reply.send(format(color).into_bytes());
            }
            // The screen stays: the last output (often why it exited) can still
            // be read, selected and saved.
            PtyEvent::Exited(code) => {
                self.exit_code = code;
                self.set_status(Status::Exited);
            }
        }
    }
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

/// What the bell does. An agent that reports through its hooks is ignored: the hooks say
/// exactly when it needs you. An agent without hooks rings to ask, so it needs you. A terminal
/// without an agent marks the bell until you look, and counts as needing you at most once a
/// minute (`since_counted`: how long ago it last did), so a program ringing in a loop doesn't keep
/// asking. An ended shell ignores it.
fn on_bell(status: Status, hook_driven: bool, agent: bool, since_counted: Option<Duration>) -> Bell {
    match (status, hook_driven, agent) {
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

/// The system's alert sound; never from a test instance in background mode.
fn system_beep() {
    #[cfg(target_os = "macos")]
    if !crate::background::on() {
        objc2_app_kit::NSBeep();
    }
}

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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use std::time::Instant;

    use super::{Bell, RedrawGate, Status, TITLE_EVERY, TitleGate, on_bell};

    #[test]
    fn the_bell_asks_for_you_only_where_it_means_something() {
        let (quiet, ask, mark) = (
            Bell { mark: false, needs_you: false },
            Bell { mark: false, needs_you: true },
            Bell { mark: true, needs_you: true },
        );
        assert_eq!(on_bell(Status::Busy, true, true, None), quiet, "hooks say when an agent needs you");
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
}
