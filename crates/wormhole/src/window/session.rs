//! A session: one terminal, a row of the sidebar, with the notes, tasks and git state that belong
//! to it. Several of them may be shown together as a split (`splits.rs`); each is still a row of its
//! own, with its own notes and tasks.
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::storage::store::SessionRecord;
use wormhole_core::view::panes::PaneId;
use wormhole_core::view::sidebar::{SplitId, names};

use crate::app_state::app;
use crate::dialog::shortened;
use crate::terminal_view::{Status, TerminalEvent, TerminalView};
use crate::window::Workspace;
use crate::window::feature::Stake;
use crate::window::repo::GitState;

/// A session's terminal: what the shell's agent hooks report is its id.
pub(super) struct Pane {
    /// What the shell's agent hooks report (`WORMHOLE_TAB_ID`): the session's own id, unique
    /// across every session of every window.
    pub id: PaneId,
    /// The shell profile it was opened with (core `shells`' key; `None`: the default), kept so a
    /// restart and a restored session run the same thing.
    pub shell: Option<String>,
    pub terminal: Entity<TerminalView>,
    _observer: Subscription,
    _events: Subscription,
    _focus: Subscription,
}

impl Pane {
    pub(super) fn new(
        id: PaneId,
        shell: Option<String>,
        terminal: Entity<TerminalView>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Self {
        let observer = cx.observe(&terminal, |_, _, cx| cx.notify());
        let events = cx.subscribe_in(&terminal, window, move |this, _, event: &TerminalEvent, window, cx| {
            this.on_terminal_event(id, *event, window, cx)
        });
        // The keyboard coming into the terminal (a click, ⌥⌘arrow, F6) selects its row: the one
        // whose notes, git state and keys the window is about, and ⌘D, ⌘W and the find bar act on.
        let handle = terminal.read(cx).focus_handle.clone();
        let focus = cx.on_focus(&handle, window, move |this, window, cx| this.pane_focused(id, window, cx));
        Self { id, shell, terminal, _observer: observer, _events: events, _focus: focus }
    }

    /// The terminal's own name: the title its program set, else what runs in it, the agent,
    /// the program in front or the shell.
    pub(super) fn label(&self, cx: &App) -> SharedString {
        names::process_name(&name_facts(self.terminal.read(cx))).into()
    }

    /// What runs in front, whatever title it set: the agent, the running command or the shell.
    pub(super) fn program(&self, cx: &App) -> SharedString {
        names::program_name(&name_facts(self.terminal.read(cx))).into()
    }
}

pub(super) struct Session {
    /// The session's id, which is its terminal's too.
    pub id: i64,
    pub number: usize,
    /// The name a project gave it (`layout.json`, a `session:<name>` action), which also finds it
    /// again. People don't name terminals: a terminal is named after what runs in it.
    pub custom_name: Option<String>,
    /// Its terminal.
    pub pane: Pane,
    /// The split it is shown in with other terminals (the window's `splits`), if any.
    pub split: Option<SplitId>,
    /// What git last said of its folder.
    pub git: GitState,
    /// Where the session's folder lives on disk, for grouping the sidebar by folder.
    pub repo_root: RepoRoot,
    /// Pinned to the top of its group in the sidebar (saved). Every member of a split carries it.
    pub pinned: bool,
    /// Its text is never saved for a restart ("Don't keep this terminal's text", saved).
    pub forget_text: bool,
    /// What the title row's extras know about it: its agent's model and context, its ports.
    pub extras: crate::window::title::extras::SessionExtras,
}

impl Session {
    /// The session's terminal: what has the keyboard when the session is selected, and whose
    /// folder the panels show the repository of.
    pub(super) fn terminal(&self) -> &Entity<TerminalView> {
        &self.pane.terminal
    }

    pub(super) fn status(&self, cx: &App) -> Status {
        self.terminal().read(cx).status()
    }

    /// The session's name: the project's, else what runs in its terminal.
    pub(super) fn name(&self, cx: &App) -> SharedString {
        if let Some(name) = &self.custom_name {
            return name.clone().into();
        }
        let name = self.pane.label(cx);
        if name.is_empty() { format!("Terminal {}", self.number).into() } else { name }
    }

    /// Who a question about closing it names as at stake: the agent by its short name, else the
    /// program that reports its own status ("brew is still working").
    pub(super) fn told_name(&self, cx: &App) -> Option<String> {
        let terminal = self.terminal().read(cx);
        terminal
            .agent
            .map(|id| wormhole_core::agents::agent(id).short_name().to_string())
            .or_else(|| terminal.program_name())
    }

    /// Whether a program other than the shell is in the foreground.
    pub(super) fn command_running(&self, cx: &App) -> bool {
        self.terminal().read(cx).program_in_foreground()
    }

    /// The program running in the terminal, as its row names it ("claude", "npm run dev"): none
    /// when the question already names its agent (`skip_agent`).
    pub(super) fn running_programs(&self, skip_agent: bool, cx: &App) -> Vec<String> {
        let foreground = self.terminal().read(cx).foreground().map(|f| f.command.clone());
        foreground.into_iter().filter(|_| !skip_agent).collect()
    }

    /// Closing the session would cut something off: an agent at work or waiting for an answer, or
    /// a command running. Notes and tasks alone never ask; they are the user's to keep or not.
    pub(super) fn needs_close_confirmation(&self, cx: &App) -> bool {
        close_needs_confirmation(self.status(cx), self.command_running(cx))
    }

    /// The window's part of the saved record; the features add theirs (`Workspace::session_record`).
    pub(super) fn record(&self, cx: &App) -> SessionRecord {
        SessionRecord {
            id: self.id,
            name: self.custom_name.clone(),
            tasks_collapsed: false,
            notes_collapsed: false,
            notes: Vec::new(),
            tasks: Vec::new(),
            pinned: self.pinned,
            forget_text: self.forget_text,
            cwd: self.terminal().read(cx).cwd.as_ref().map(|p| p.to_string_lossy().into_owned()),
            shell: self.pane.shell.clone(),
            scrollback: None,
            split_id: self.split,
            host_id: self.persisted_host(cx),
        }
    }

    /// The session host the row names: the terminal's own, unless it was lost or every host is
    /// being ended on the way out (core `hosts::persisted_host`).
    fn persisted_host(&self, cx: &App) -> Option<String> {
        let terminal = self.terminal().read(cx);
        wormhole_core::ipc::hosts::persisted_host(
            terminal.host_id(),
            terminal.host_lost(),
            app(cx).ending_hosts,
        )
    }

    /// Its program runs in a session host: quitting leaves it running (unless the question before
    /// quitting is answered with the box unticked).
    pub(super) fn is_hosted(&self, cx: &App) -> bool {
        self.terminal().read(cx).is_hosted()
    }
}

/// What runs in a terminal, for its name.
fn name_facts(terminal: &TerminalView) -> names::NameFacts<'_> {
    names::NameFacts {
        title: terminal.title.as_deref(),
        agent: terminal.agent.map(|id| wormhole_core::agents::agent(id).command),
        command: terminal.naming_command(),
        shell: terminal.shell_name(),
    }
}
/// Whether closing a session whose terminal is in `status`, and which has a command running if
/// `command_running`, asks first. Closing a window asks by the same rule for each of its
/// sessions.
fn close_needs_confirmation(status: Status, command_running: bool) -> bool {
    command_running || matches!(status, Status::Busy | Status::Attention)
}

/// What the question before closing a session says is at stake: its agent by name, then the
/// other programs that end with it (`programs`, without the agent's own).
pub(super) fn close_session_detail(
    agent: Option<&str>,
    stakes: &[Stake],
    status: Status,
    command_running: bool,
    programs: &[String],
) -> String {
    let who = agent.unwrap_or("Its agent");
    let mut sentences = Vec::new();
    match status {
        Status::Busy => sentences.push(format!("{who} is still working.")),
        Status::Attention => sentences.push(format!("{who} is waiting for an answer.")),
        Status::Idle | Status::Done | Status::Failed | Status::Exited => {}
    }
    let ending = sentences.len() + programs_named(programs);
    match running_list(programs) {
        Some(list) => sentences.push(format!("Still running: {list}.")),
        None if command_running && sentences.is_empty() => {
            sentences.push("A command is still running.".to_string())
        }
        None => {}
    }
    sentences.push(match (sentences.is_empty(), ending) {
        (true, _) => "Its shell ends.".to_string(),
        (false, 0 | 1) => "Closing ends the process.".to_string(),
        (false, _) => "Closing ends them.".to_string(),
    });
    sentences.extend(stakes.iter().map(|stake| format!("Its {}", stake.sentence)));
    sentences.join(" ")
}

/// How many names the question lists before "and N more".
const PROGRAMS_LISTED: usize = 4;

/// How many different programs `programs` holds.
fn programs_named(programs: &[String]) -> usize {
    let mut names: Vec<&String> = programs.iter().collect();
    names.sort_unstable();
    names.dedup();
    names.len()
}

/// The programs that end, as Terminal's question lists them: "claude, npm run dev", each once, in
/// the order of the terminals, a long command cut short and a long list ending in "and N more".
fn running_list(programs: &[String]) -> Option<String> {
    let mut names: Vec<&str> = Vec::new();
    for program in programs {
        if !names.contains(&program.as_str()) {
            names.push(program);
        }
    }
    let shown: Vec<String> = names.iter().take(PROGRAMS_LISTED).map(|name| shortened(name)).collect();
    let mut list = shown.join(", ");
    if names.len() > PROGRAMS_LISTED {
        list.push_str(&format!(" and {} more", names.len() - PROGRAMS_LISTED));
    }
    (!names.is_empty()).then_some(list)
}

/// The question before closing a session, naming it (“Close “sidebar polish”?”).
pub(super) fn close_session_title(name: &str) -> String {
    format!("Close \u{201c}{name}\u{201d}?")
}

/// What the question before closing a window says: how many terminals end, the most pressing
/// thing still running among `running` (the lead status and whether a command runs, of each
/// session that asks), the programs that end by name, and whether notes and tasks go with them.
pub(super) fn close_window_detail(
    terminals: usize,
    running: &[(Status, bool)],
    programs: &[String],
    stakes: &[Stake],
) -> String {
    let mut detail = match terminals {
        1 => "The terminal in this window will end.".to_string(),
        n => format!("The {n} terminals in this window will end."),
    };
    detail.push_str(&still_running(running, programs));
    for stake in stakes {
        detail.push_str(&format!(" Their {}", stake.sentence));
    }
    detail
}

/// The title of the question before closing a group.
pub(super) fn close_group_title(name: &str) -> String {
    format!("Close the group \u{201c}{name}\u{201d}?")
}

/// What the question before closing a group says: how many terminals end, and what still runs.
pub(super) fn close_group_detail(
    terminals: usize,
    running: &[(Status, bool)],
    programs: &[String],
    stakes: &[Stake],
) -> String {
    let mut detail = match terminals {
        1 => "The terminal in this group will end.".to_string(),
        n => format!("The {n} terminals in this group will end."),
    };
    detail.push_str(&still_running(running, programs));
    for stake in stakes {
        detail.push_str(&format!(" Their {}", stake.sentence));
    }
    detail
}

/// What the question before quitting says: the terminals come back, what runs in them doesn't.
pub(super) fn quit_detail(
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
) -> String {
    quit_detail_with(
        "Your terminals come back next time, but what runs in them ends.",
        running,
        programs,
        other_profiles,
    )
}

/// [`quit_detail`] under another opening sentence.
fn quit_detail_with(
    head: &str,
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
) -> String {
    let mut elsewhere = String::new();
    for (profile, count) in other_profiles {
        let (many, verb) = if *count == 1 { ("terminal", "is") } else { ("terminals", "are") };
        elsewhere.push_str(&format!(" {count} {many} in {profile} {verb} running an agent or a command."));
    }
    format!("{head}{}{elsewhere}", still_running(running, programs))
}

/// How the question before quitting is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QuitAsk {
    /// Quit / Cancel only: the sheet GPUI draws.
    Plain,
    /// With the box "Keep terminals running in the background". `local_running`: some terminal
    /// that runs something can't be kept (it runs in the app's own process), so it ends either way.
    KeepBox { local_running: bool },
}

/// How ⌘Q asks, given how many terminals that run something are hosted (`hosted_running`) or local
/// (`local_running`), and whether any terminal at all is hosted (`any_hosted`: the box is about
/// those, idle ones too). `None`: nothing runs, so quit without asking. Quit and End All Terminals
/// (`end_hosts`) ends everything, so it has nothing to choose.
pub(super) fn quit_ask(
    end_hosts: bool,
    hosted_running: usize,
    local_running: usize,
    any_hosted: bool,
) -> Option<QuitAsk> {
    if hosted_running + local_running == 0 {
        return None;
    }
    Some(if end_hosts || !any_hosted {
        QuitAsk::Plain
    } else {
        QuitAsk::KeepBox { local_running: local_running > 0 }
    })
}

/// Whether quitting ends the session hosts too: the box "Keep terminals running in the background"
/// left unticked.
pub(super) fn quit_ends_hosts(keep_running: bool) -> bool {
    !keep_running
}

/// What the question before quitting says when it has the box: what runs in terminals that are
/// kept goes on, what runs in the others ends. Without a kept terminal that runs something there is
/// nothing to add to [`quit_detail`], which stays true whichever way the box is.
pub(super) fn quit_detail_keeping(
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
    hosted_running: bool,
    local_running: bool,
) -> String {
    if !hosted_running {
        return quit_detail(running, programs, other_profiles);
    }
    let except = if local_running { ", except in terminals that can't run in the background" } else { "" };
    let head = format!(
        "Your terminals come back next time and what runs in them keeps going{except}. Untick the box to \
         end them."
    );
    quit_detail_with(&head, running, programs, other_profiles)
}

/// What still runs among sessions that ask before closing (`running`: each one's lead status and
/// whether a command runs; `programs`: their terminals' programs), each sentence after a space: an
/// agent waiting or working, then the programs by name.
fn still_running(running: &[(Status, bool)], programs: &[String]) -> String {
    let any = |status: Status| running.iter().any(|&(s, _)| s == status);
    let mut detail = String::new();
    if any(Status::Attention) {
        detail.push_str(" An agent is waiting for an answer.");
    } else if any(Status::Busy) {
        detail.push_str(" An agent is still working.");
    }
    if let Some(list) = running_list(programs) {
        detail.push_str(&format!(" Still running: {list}."));
    } else if detail.is_empty() && running.iter().any(|&(_, command)| command) {
        detail.push_str(" A command is still running.");
    }
    detail
}

/// The root of the repository `cwd` is in (else `cwd` itself), found on disk once per folder.
/// `git::find_root` stats `.git` in every ancestor, which grouping the sidebar by folder asked
/// for every session on every frame, and frames come as fast as a terminal prints.
#[derive(Default)]
pub(super) struct RepoRoot(RefCell<Option<(PathBuf, PathBuf)>>);

impl RepoRoot {
    pub(super) fn of(&self, cwd: &Path) -> PathBuf {
        self.of_with(cwd, wormhole_core::git::find_root)
    }

    fn of_with(&self, cwd: &Path, find: impl FnOnce(&Path) -> Option<PathBuf>) -> PathBuf {
        if let Some((from, root)) = &*self.0.borrow()
            && from == cwd
        {
            return root.clone();
        }
        let root = find(cwd).unwrap_or_else(|| cwd.to_path_buf());
        *self.0.borrow_mut() = Some((cwd.to_path_buf(), root.clone()));
        root
    }

    /// Look again next time: a `git init` or a removed `.git` doesn't change the folder.
    pub(super) fn forget(&self) {
        self.0.take();
    }
}

/// The selected session after session `closed` is removed from `len` sessions. `None`: it was the only one,
/// so the caller starts a new session. `Some((ix, switched))`: the selection is now session `ix`, and
/// `switched` says whether that is a different session from before (then its panels start fresh);
/// closing another session in the background must leave the selected one exactly as it was.
pub(super) fn session_after_close(active: usize, closed: usize, len: usize) -> Option<(usize, bool)> {
    debug_assert!(active < len && closed < len);
    if len < 2 {
        return None;
    }
    Some(match closed.cmp(&active) {
        // Everything after the closed session moves one place up, the selected one with it.
        std::cmp::Ordering::Less => (active - 1, false),
        // The session that slid into its place, or the last one if it was last.
        std::cmp::Ordering::Equal => (active.min(len - 2), true),
        std::cmp::Ordering::Greater => (active, false),
    })
}

/// `active` moved by `delta` places, wrapping around `len` items.
pub(super) fn step_wrapping(active: usize, delta: isize, len: usize) -> usize {
    (active as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::path::{Path, PathBuf};

    use super::{
        RepoRoot, close_needs_confirmation, close_session_detail, close_session_title, close_window_detail,
        quit_ask, quit_detail, quit_detail_keeping, quit_ends_hosts, running_list, session_after_close,
        step_wrapping,
    };
    use crate::terminal_view::Status;
    use crate::window::feature::Stake;

    #[test]
    fn closing_sessions_keeps_the_selection_sensible() {
        // The only session: nothing is left, a new one takes over.
        assert_eq!(session_after_close(0, 0, 1), None);
        // The selected session closes: the one below slides up and is new on screen...
        assert_eq!(session_after_close(1, 1, 3), Some((1, true)));
        // ...or the one above when it was the last.
        assert_eq!(session_after_close(2, 2, 3), Some((1, true)));
        assert_eq!(session_after_close(0, 0, 2), Some((0, true)));
        // A session above the selected one closes: the selection moves up with its session, unchanged.
        assert_eq!(session_after_close(2, 0, 4), Some((1, false)));
        // One below it closes (a background session's ×): nothing changes at all.
        assert_eq!(session_after_close(1, 3, 4), Some((1, false)));
        assert_eq!(session_after_close(0, 1, 2), Some((0, false)));
    }

    #[test]
    fn stepping_wraps_in_both_directions() {
        assert_eq!(step_wrapping(0, 1, 3), 1);
        assert_eq!(step_wrapping(2, 1, 3), 0);
        assert_eq!(step_wrapping(0, -1, 3), 2);
        assert_eq!(step_wrapping(1, -1, 3), 0);
        assert_eq!(step_wrapping(0, 1, 1), 0);
    }

    #[test]
    fn a_folders_root_is_looked_up_once_until_it_changes_or_is_forgotten() {
        let lookups = Cell::new(0);
        let cache = RepoRoot::default();
        let find = |root: &'static str| {
            let lookups = &lookups;
            move |_: &Path| {
                lookups.set(lookups.get() + 1);
                Some(PathBuf::from(root))
            }
        };
        let (a, b) = (Path::new("/work/app/src"), Path::new("/work/other"));
        assert_eq!(cache.of_with(a, find("/work/app")), PathBuf::from("/work/app"));
        // Every later frame for the same folder reads the cache; the finder must not run.
        for _ in 0..100 {
            assert_eq!(cache.of_with(a, |_| unreachable!("cached")), PathBuf::from("/work/app"));
        }
        assert_eq!(lookups.get(), 1);
        // The session went somewhere else.
        assert_eq!(cache.of_with(b, find("/work/other")), PathBuf::from("/work/other"));
        assert_eq!(lookups.get(), 2);
        // Back again, after being told to look afresh (a `.git` may have appeared).
        cache.forget();
        assert_eq!(cache.of_with(b, find("/work")), PathBuf::from("/work"));
        assert_eq!(lookups.get(), 3);
    }

    #[test]
    fn outside_a_repository_the_folder_is_its_own_root() {
        let cache = RepoRoot::default();
        let cwd = Path::new("/tmp/scratch");
        assert_eq!(cache.of_with(cwd, |_| None), cwd);
        assert_eq!(cache.of_with(cwd, |_| unreachable!("cached")), cwd);
    }

    /// What the cache saves: `find_root` on a deep folder (stat of `.git` in each ancestor) per
    /// session and frame. Run with `--ignored --nocapture`.
    #[test]
    #[ignore = "micro-benchmark"]
    #[allow(clippy::print_stdout, reason = "the numbers are the output")]
    fn bench_root_lookup_per_frame() {
        let mut deep = std::env::temp_dir().join(format!("hd-root-bench-{}", std::process::id()));
        let base = deep.clone();
        for level in 0..12 {
            deep.push(format!("level{level}"));
        }
        std::fs::create_dir_all(&deep).expect("creates the folders");
        let (sessions, frames) = (20, 1000);

        let start = std::time::Instant::now();
        for _ in 0..frames {
            for _ in 0..sessions {
                std::hint::black_box(wormhole_core::git::find_root(&deep));
            }
        }
        let uncached = start.elapsed() / frames;

        let caches: Vec<RepoRoot> = (0..sessions).map(|_| RepoRoot::default()).collect();
        let start = std::time::Instant::now();
        for _ in 0..frames {
            for cache in &caches {
                std::hint::black_box(cache.of(&deep));
            }
        }
        let cached = start.elapsed() / frames;
        std::fs::remove_dir_all(&base).expect("cleans up");
        println!(
            "grouped sidebar, {sessions} sessions at depth 15: {uncached:?} per frame uncached, {cached:?} cached"
        );
    }

    /// What the notes feature says is at stake when `open` items would be discarded.
    fn stakes(open: usize) -> Vec<Stake> {
        wormhole_core::view::notes::stake::discarded(open)
            .map(|sentence| Stake { count: open, sentence: sentence.to_string() })
            .into_iter()
            .collect()
    }

    fn names(programs: &[&str]) -> Vec<String> {
        programs.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn closing_a_window_names_what_is_still_running() {
        assert_eq!(
            close_window_detail(3, &[(Status::Busy, false), (Status::Attention, false)], &[], &[]),
            "The 3 terminals in this window will end. An agent is waiting for an answer."
        );
        assert_eq!(
            close_window_detail(1, &[(Status::Busy, true)], &names(&["claude", "npm run dev"]), &stakes(2)),
            "The terminal in this window will end. An agent is still working. Still running: claude, npm \
             run dev. Their notes and tasks are discarded."
        );
        assert_eq!(
            close_window_detail(2, &[(Status::Idle, true)], &names(&["cargo watch"]), &[]),
            "The 2 terminals in this window will end. Still running: cargo watch."
        );
        // A command whose name isn't known yet is still mentioned.
        assert_eq!(
            close_window_detail(2, &[(Status::Idle, true)], &[], &[]),
            "The 2 terminals in this window will end. A command is still running."
        );
    }

    #[test]
    fn quitting_says_the_terminals_come_back_but_their_programs_end() {
        assert_eq!(
            quit_detail(&[(Status::Idle, true)], &names(&["claude", "caffeinate"]), &[]),
            "Your terminals come back next time, but what runs in them ends. Still running: claude, \
             caffeinate."
        );
    }

    #[test]
    fn quitting_names_the_other_profiles_that_still_run_something() {
        assert_eq!(
            quit_detail(&[(Status::Busy, false)], &[], &[("Work".into(), 1), ("Client".into(), 3)]),
            "Your terminals come back next time, but what runs in them ends. An agent is still working. \
             1 terminal in Work is running an agent or a command. \
             3 terminals in Client are running an agent or a command."
        );
    }

    #[test]
    fn quitting_with_the_box_says_what_goes_on_and_what_ends() {
        let running = [(Status::Idle, true)];
        let programs = names(&["claude"]);
        // Nothing kept runs something: the plain wording is true either way.
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[], false, true),
            quit_detail(&running, &programs, &[])
        );
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[], true, false),
            "Your terminals come back next time and what runs in them keeps going. Untick the box to end \
             them. Still running: claude."
        );
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[("Work".into(), 1)], true, true),
            "Your terminals come back next time and what runs in them keeps going, except in terminals \
             that can't run in the background. Untick the box to end them. Still running: claude. \
             1 terminal in Work is running an agent or a command."
        );
    }

    #[test]
    fn the_box_shows_when_a_terminal_is_hosted_and_something_runs() {
        use super::QuitAsk::{KeepBox, Plain};
        // Nothing runs: no question, hosted or not.
        assert_eq!(quit_ask(false, 0, 0, true), None);
        assert_eq!(quit_ask(true, 0, 0, true), None);
        // No hosted terminal at all: today's question.
        assert_eq!(quit_ask(false, 0, 2, false), Some(Plain));
        // A hosted terminal that runs something, or an idle one beside a local one that does.
        assert_eq!(quit_ask(false, 1, 0, true), Some(KeepBox { local_running: false }));
        assert_eq!(quit_ask(false, 1, 2, true), Some(KeepBox { local_running: true }));
        assert_eq!(quit_ask(false, 0, 1, true), Some(KeepBox { local_running: true }));
        // Quit and End All Terminals has no choice to offer.
        assert_eq!(quit_ask(true, 1, 1, true), Some(Plain));
    }

    #[test]
    fn the_box_ticked_keeps_the_hosts_and_unticked_ends_them() {
        assert!(!quit_ends_hosts(true));
        assert!(quit_ends_hosts(false));
    }

    #[test]
    fn the_running_programs_are_listed_once_and_briefly() {
        assert_eq!(running_list(&[]), None);
        assert_eq!(running_list(&names(&["node", "claude", "node"])).as_deref(), Some("node, claude"));
        assert_eq!(
            running_list(&names(&["a", "b", "c", "d", "e", "f"])).as_deref(),
            Some("a, b, c, d and 2 more")
        );
        let long = "x".repeat(50);
        assert_eq!(running_list(&[long]), Some(format!("{}…", "x".repeat(40))));
    }

    #[test]
    fn closing_a_session_asks_only_while_something_is_running() {
        assert!(!close_needs_confirmation(Status::Idle, false), "a shell at its prompt");
        assert!(!close_needs_confirmation(Status::Done, false), "a finished agent has nothing left to lose");
        assert!(!close_needs_confirmation(Status::Exited, false));
        assert!(close_needs_confirmation(Status::Busy, false), "an agent at work");
        assert!(close_needs_confirmation(Status::Attention, false), "an agent waiting for an answer");
        assert!(close_needs_confirmation(Status::Idle, true), "a command in the foreground");
    }

    #[test]
    fn the_question_names_what_would_be_lost() {
        assert_eq!(close_session_title("api"), "Close \u{201c}api\u{201d}?");
        // The agent by name, then what closing does.
        assert_eq!(
            close_session_detail(Some("Claude"), &[], Status::Busy, true, &[]),
            "Claude is still working. Closing ends the process."
        );
        assert_eq!(close_session_detail(None, &[], Status::Idle, false, &[]), "Its shell ends.");
        assert_eq!(
            close_session_detail(None, &stakes(3), Status::Idle, false, &[]),
            "Its shell ends. Its notes and tasks are discarded."
        );
        assert_eq!(
            close_session_detail(None, &[], Status::Idle, true, &[]),
            "A command is still running. Closing ends the process."
        );
        assert_eq!(
            close_session_detail(None, &[], Status::Idle, true, &names(&["npm run dev"])),
            "Still running: npm run dev. Closing ends the process."
        );
        // The agent's other programs end with it.
        assert_eq!(
            close_session_detail(Some("Claude"), &[], Status::Busy, true, &names(&["cargo watch"])),
            "Claude is still working. Still running: cargo watch. Closing ends them."
        );
        assert_eq!(
            close_session_detail(Some("Codex"), &stakes(2), Status::Attention, false, &[]),
            "Codex is waiting for an answer. Closing ends the process. Its notes and tasks are discarded."
        );
    }
}
