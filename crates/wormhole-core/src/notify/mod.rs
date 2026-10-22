//! Who hears about a terminal, and how: the rules behind the notifications, the
//! Dock badge and bounce, and the status item. Pure, so every rule is tested here with a fake
//! clock; the app gathers the facts and posts or draws what comes out.
//!
//! Plain shells are first-class: a long command that ends (or fails, with its exit code, where the
//! shell has integration) and the bell are told like an agent's question or its finishing. So are
//! programs that report their own status (OSC 7501) without being a coding agent.
//!
//! Tier 4, beside `git`: it reads `config` (which events to tell, the badge and the sound) and
//! `view` (the sidebar's and VoiceOver's words), and nothing above. Never git, the socket or the
//! app: the facts come to it as values, which is what keeps it pure.

mod announce;
mod burst;
mod listing;
mod text;

use std::time::Duration;

pub use announce::{Announcement, Announcer, Priority, Said};
pub use burst::{Bursts, Grouped, Throttle};
pub use listing::{
    Counts, ItemState, MENU_BADGE_MAX, Sections, Terminal, badge, item_state, matches, next_waiting,
    sections, tooltip,
};
pub use text::{About, Text, duration, elapsed, summary, text};

use crate::config::{DockBadge, InFront, NotifySound, Settings};

/// How many notifications within [`NOTIFY_BURST_WINDOW`] become one summary.
pub const NOTIFY_BURST: usize = 3;
pub const NOTIFY_BURST_WINDOW: Duration = Duration::from_secs(10);
/// A terminal without an agent tells about its bell at most this often.
pub const BELL_RATE: Duration = Duration::from_secs(60);
/// A terminal whose status a program reports (OSC 7501) asks for you, and tells you it finished,
/// at most this often each: the program can change state as fast as it can write, and each change
/// would otherwise be a banner and a sound.
pub const REPORT_RATE: Duration = Duration::from_secs(5);
/// The optional working animation of the status item: four frames, a quarter second each.
pub const MENU_ANIM_FRAMES: usize = 4;
pub const MENU_ANIM_FRAME: Duration = Duration::from_millis(250);

/// What a terminal is doing, as the ambient surfaces count it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Idle,
    Working,
    /// An agent asks a question or for permission, or a terminal without one rang its bell.
    NeedsYou,
    /// Finished while nobody was looking, and not seen since.
    Done,
    /// A command ended with a non-zero exit code, or an agent or program stopped on an error,
    /// while nobody was looking, and not seen since.
    Failed,
    Exited,
}

/// What tells a terminal's state, which decides what a change of it means: the same "needs you"
/// is a question from an agent or a program, and a bell from a terminal without either.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Source {
    /// Neither: the output, the bell and the shell's marks are all there is.
    #[default]
    Plain,
    /// A coding agent, by its hooks or its own reports.
    Agent,
    /// A program that reports its own status (OSC 7501) and is no known coding agent: `brew`, a
    /// deploy script, a build tool. Its word is as good as an agent's.
    Program,
}

impl Source {
    /// The state is told by the program in the terminal, not guessed from what the terminal does.
    pub fn told(self) -> bool {
        self != Source::Plain
    }
}

/// What happened in a terminal that may be worth telling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An agent needs an answer or a permission.
    NeedsYou,
    /// An agent finished (its Stop hook, or its output settled).
    AgentDone,
    /// A command in a terminal without an agent ran past the threshold and ended.
    CommandDone,
    /// A command ended with a non-zero exit code (shell integration), past the same threshold.
    CommandFailed,
    /// An agent or a program stopped on an error (its hook, or its own report).
    AgentFailed,
    /// A terminal without an agent rang its bell.
    Bell,
    /// A program sent a notification of its own (OSC 9, 99 or 777), with its words: the bell
    /// with text, from a program that meant it.
    Notice,
}

/// How hard a banner interrupts (`UNNotificationInterruptionLevel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Notification Center only: no banner, no sound.
    Passive,
    Active,
    /// Breaks through a Focus that allows it.
    TimeSensitive,
}

/// The level a kind is delivered at: needs you is Time Sensitive when the build is entitled to
/// it (a Developer ID build with the provisioning profile), else Active; an agent finishing, a
/// failure of any kind and a program's own notification are Active; a command finishing and the
/// bell are Passive. A grouped summary is Active.
pub fn interruption_level(kind: Kind, has_time_sensitive: bool) -> Level {
    match kind {
        Kind::NeedsYou if has_time_sensitive => Level::TimeSensitive,
        Kind::NeedsYou | Kind::AgentDone | Kind::CommandFailed | Kind::AgentFailed | Kind::Notice => {
            Level::Active
        }
        Kind::CommandDone | Kind::Bell => Level::Passive,
    }
}

/// What reaches the user about one event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Nothing,
    /// A system banner.
    Banner,
}

/// Where the user's eyes are when it happens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// The terminal's tab shows in the key window and Wormhole is in front.
    pub looking: bool,
    /// Wormhole is the active app.
    pub app_active: bool,
}

/// Whether the settings ask to hear about `kind` at all (`notifications` is the master switch).
/// A program's own notification asks for you on purpose, so it follows "needs you" rather than
/// the bell, which is off by default because programs ring it by accident. A command that fails
/// has its own switch; an agent or program that stops on an error is the end of its work like a
/// finish is, so it follows "an agent finishes".
pub fn wanted(kind: Kind, settings: &Settings) -> bool {
    settings.notifications
        && match kind {
            Kind::NeedsYou | Kind::Notice => settings.notify_needs_you,
            Kind::AgentDone | Kind::AgentFailed => settings.notify_agent_done,
            Kind::CommandDone => settings.notify_command_done,
            Kind::CommandFailed => settings.notify_command_failed,
            Kind::Bell => settings.notify_bell,
        }
}

/// How `kind` reaches the user: not at all
/// while they look at the terminal (unless "only for terminals I'm not looking at" is off), as
/// `notify_in_front` says while Wormhole is in front, and as a banner otherwise.
pub fn delivery(kind: Kind, seen: Seen, settings: &Settings) -> Delivery {
    if !wanted(kind, settings) || (seen.looking && settings.notify_only_unseen) {
        return Delivery::Nothing;
    }
    if !seen.app_active {
        return Delivery::Banner;
    }
    match settings.notify_in_front {
        InFront::Banner => Delivery::Banner,
        InFront::None => Delivery::Nothing,
    }
}

/// Whether a banner plays the system sound: never a Passive one (Notification Center only), and
/// otherwise as `notify_sound` says.
pub fn sound(kind: Kind, level: Level, setting: NotifySound) -> bool {
    level != Level::Passive
        && match setting {
            NotifySound::All => true,
            NotifySound::NeedsYou => kind == Kind::NeedsYou,
            NotifySound::None => false,
        }
}

/// What a tab's change of state does to its notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Tell about it (the rules above decide how).
    Post(Kind),
    /// A notice that is up no longer says anything true: the terminal moved on.
    Retract,
    Keep,
}

/// A tab went from `before` to `after`, told by `source`. Needing you is the question of an agent
/// or a program that reports, or the bell where there is neither; finishing and failing count only
/// for those two (a plain terminal's output settling isn't news; its command ending is, see
/// [`command_finished`] and [`command_failed`]).
pub fn on_state(before: State, after: State, source: Source) -> Change {
    if before == after {
        return Change::Keep;
    }
    match (after, source.told()) {
        (State::NeedsYou, true) => Change::Post(Kind::NeedsYou),
        (State::NeedsYou, false) => Change::Post(Kind::Bell),
        (State::Done, true) => Change::Post(Kind::AgentDone),
        (State::Failed, true) => Change::Post(Kind::AgentFailed),
        (State::Done | State::Failed, false) => Change::Keep,
        (State::Working | State::Idle | State::Exited, _) => Change::Retract,
    }
}

/// Whether a command that left the foreground after `took` is worth a "finished" notice: only in
/// a terminal whose state nothing tells (`told`: an agent, or a program reporting its own status,
/// says when it is done itself, and its own work is not a command of the user's), only past the
/// threshold (`notify_command_after`, seconds), and never a program you sit in (vim, less, ssh)
/// or leave running (a dev server), which `sidebar::command::ends_by_itself` knows.
pub fn command_finished(command: &str, took: Duration, after_secs: u32, told: bool) -> bool {
    !told
        && !command.trim().is_empty()
        && took >= Duration::from_secs(after_secs.into())
        && crate::view::sidebar::command::ends_by_itself(command)
}

/// Whether a command that ended with a non-zero exit code is worth a "failed" notice: the same
/// rule as [`command_finished`], so a short failure only changes the row, as a short success does.
pub fn command_failed(command: &str, took: Duration, after_secs: u32, told: bool) -> bool {
    command_finished(command, took, after_secs, told)
}

/// The exit code shells give a foreground job that Ctrl-C ended: the job dies of SIGINT (signal
/// 2), and a shell reports a death by signal as 128 plus its number.
pub const INTERRUPTED: i32 = 130;

/// The exit code of a command that failed, if it did: any non-zero one but [`INTERRUPTED`]. The
/// user pressed Ctrl-C themselves, so it is not news and not an error to look at; a program that
/// handles SIGINT and exits 130 on purpose is the same to them. `None` for success, an
/// interrupt, or an exit the shell gave no code for.
pub fn failed_exit(code: Option<i32>) -> Option<i32> {
    code.filter(|&code| code != 0 && code != INTERRUPTED)
}

/// The Dock icon's badge: terminals that need you (bells of terminals without an agent
/// included), with "needs you + finished" also the finished and failed ones nobody has seen
/// (`done` counts both: [`Counts::results`]). Working never badges; `None` clears it.
pub fn dock_badge(mode: DockBadge, needs_you: usize, done: usize) -> Option<String> {
    let n = match mode {
        DockBadge::Off => 0,
        DockBadge::NeedsYou => needs_you,
        DockBadge::NeedsYouDone => needs_you + done,
    };
    (n > 0).then(|| crate::view::limits::count_label(n))
}

/// One informational bounce when a terminal starts needing you while Wormhole isn't the active
/// app; never for a bell or anything finishing.
pub fn bounces(kind: Kind, app_active: bool, dock_bounce: bool) -> bool {
    dock_bounce && !app_active && kind == Kind::NeedsYou
}

/// The identifier of a tab's notification: a newer one for the same tab replaces it.
pub fn notice_id(tab: i64) -> String {
    format!("{NOTICE_PREFIX}{tab}")
}

/// The tab a notification identifier was made for; `None` for anything else (the summary).
pub fn notice_tab(id: &str) -> Option<i64> {
    id.strip_prefix(NOTICE_PREFIX)?.parse().ok()
}

/// The identifier of the burst summary: one at a time, the next replaces it.
pub const SUMMARY_ID: &str = "wormhole/summary";
const NOTICE_PREFIX: &str = "wormhole/tab/";

/// The thread a terminal's notifications stack in (`threadIdentifier`): one per sidebar terminal,
/// whichever of its tabs they come from.
pub fn thread_id(session: i64) -> String {
    format!("wormhole/terminal/{session}")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        Change, Delivery, Kind, Level, Seen, Source, State, bounces, command_failed, command_finished,
        delivery, dock_badge, failed_exit, interruption_level, notice_id, notice_tab, on_state, sound,
        thread_id, wanted,
    };
    use crate::config::{DockBadge, InFront, NotifySound, Settings};

    const KINDS: [Kind; 7] = [
        Kind::NeedsYou,
        Kind::AgentDone,
        Kind::CommandDone,
        Kind::CommandFailed,
        Kind::AgentFailed,
        Kind::Bell,
        Kind::Notice,
    ];
    const AWAY: Seen = Seen { looking: false, app_active: false };
    const IN_FRONT: Seen = Seen { looking: false, app_active: true };
    const LOOKING: Seen = Seen { looking: true, app_active: true };

    fn settings() -> Settings {
        // The bell is off by default; most rules are easier to read with every kind on.
        Settings { notify_bell: true, ..Settings::default() }
    }

    #[test]
    fn needs_you_is_time_sensitive_only_with_the_entitlement() {
        assert_eq!(interruption_level(Kind::NeedsYou, true), Level::TimeSensitive);
        assert_eq!(interruption_level(Kind::NeedsYou, false), Level::Active);
        for entitled in [true, false] {
            assert_eq!(interruption_level(Kind::AgentDone, entitled), Level::Active);
            assert_eq!(interruption_level(Kind::Notice, entitled), Level::Active, "a program meant it");
            assert_eq!(interruption_level(Kind::CommandFailed, entitled), Level::Active);
            assert_eq!(interruption_level(Kind::AgentFailed, entitled), Level::Active);
            assert_eq!(interruption_level(Kind::CommandDone, entitled), Level::Passive);
            assert_eq!(interruption_level(Kind::Bell, entitled), Level::Passive);
        }
    }

    #[test]
    fn away_from_wormhole_everything_wanted_is_a_banner() {
        let s = settings();
        for kind in KINDS {
            assert_eq!(delivery(kind, AWAY, &s), Delivery::Banner, "{kind:?}");
        }
    }

    /// In front, the window itself says nothing unless asked for a banner: the sidebar, the
    /// Dock badge, the menu bar item and ⌥⌘J carry the status.
    #[test]
    fn in_front_the_setting_picks_banner_or_nothing() {
        let mut s = settings();
        assert_eq!(s.notify_in_front, InFront::None, "nothing is the default in front");
        for kind in KINDS {
            assert_eq!(delivery(kind, IN_FRONT, &s), Delivery::Nothing, "{kind:?}");
        }
        s.notify_in_front = InFront::Banner;
        assert_eq!(delivery(Kind::AgentDone, IN_FRONT, &s), Delivery::Banner);
        // Away from the app the in-front setting doesn't matter.
        s.notify_in_front = InFront::None;
        assert_eq!(delivery(Kind::NeedsYou, AWAY, &s), Delivery::Banner);
    }

    #[test]
    fn the_terminal_you_look_at_only_notifies_when_asked_to() {
        let mut s = settings();
        for kind in KINDS {
            assert_eq!(delivery(kind, LOOKING, &s), Delivery::Nothing, "{kind:?}");
        }
        s.notify_only_unseen = false;
        s.notify_in_front = InFront::Banner;
        assert_eq!(delivery(Kind::NeedsYou, LOOKING, &s), Delivery::Banner);
    }

    #[test]
    fn each_kind_has_its_switch_and_the_master_switch_silences_all() {
        let mut s = settings();
        s.notify_needs_you = false;
        assert_eq!(delivery(Kind::NeedsYou, AWAY, &s), Delivery::Nothing);
        assert!(wanted(Kind::AgentDone, &s));
        s.notify_agent_done = false;
        s.notify_command_done = false;
        assert!(!wanted(Kind::AgentDone, &s) && !wanted(Kind::CommandDone, &s));
        assert!(!wanted(Kind::Bell, &Settings::default()), "the bell is off by default");
        // A failed command has its own switch; an agent or program that stopped on an error follows
        // the one for an agent finishing.
        assert!(wanted(Kind::CommandFailed, &Settings::default()), "a failed command is on by default");
        assert!(!wanted(Kind::AgentFailed, &s), "follows an agent finishing");
        assert!(wanted(Kind::CommandFailed, &s), "and a command failing is its own");
        s.notify_command_failed = false;
        assert!(!wanted(Kind::CommandFailed, &s));
        assert!(wanted(Kind::Notice, &Settings::default()), "a program's own notification is on");
        assert!(!wanted(Kind::Notice, &s), "and follows needs you");
        let off = Settings { notifications: false, ..settings() };
        for kind in KINDS {
            assert_eq!(delivery(kind, AWAY, &off), Delivery::Nothing, "{kind:?}");
        }
    }

    #[test]
    fn sound_follows_the_setting_and_never_for_passive() {
        let needs = interruption_level(Kind::NeedsYou, false);
        assert!(sound(Kind::NeedsYou, needs, NotifySound::NeedsYou));
        assert!(!sound(Kind::AgentDone, Level::Active, NotifySound::NeedsYou));
        assert!(sound(Kind::AgentDone, Level::Active, NotifySound::All));
        assert!(!sound(Kind::Bell, Level::Passive, NotifySound::All), "Notification Center only");
        assert!(!sound(Kind::NeedsYou, Level::TimeSensitive, NotifySound::None));
    }

    #[test]
    fn a_state_change_posts_retracts_or_keeps() {
        use Source::{Agent, Plain};
        use State::*;
        assert_eq!(on_state(Working, NeedsYou, Agent), Change::Post(Kind::NeedsYou));
        assert_eq!(on_state(Idle, NeedsYou, Plain), Change::Post(Kind::Bell), "a plain terminal's bell");
        assert_eq!(on_state(Working, Done, Agent), Change::Post(Kind::AgentDone));
        assert_eq!(on_state(Working, Done, Plain), Change::Keep, "output settling isn't news");
        assert_eq!(on_state(Working, Failed, Agent), Change::Post(Kind::AgentFailed));
        assert_eq!(on_state(Working, Failed, Plain), Change::Keep, "a command's failure is told by its exit");
        for after in [Working, Idle, Exited] {
            assert_eq!(on_state(NeedsYou, after, Agent), Change::Retract, "{after:?}");
            assert_eq!(on_state(Failed, after, Plain), Change::Retract, "looked at: {after:?}");
        }
        for state in [Idle, Working, NeedsYou, Done, Failed, Exited] {
            assert_eq!(on_state(state, state, Agent), Change::Keep, "{state:?}");
        }
    }

    /// A program that reports its own status is told like an agent: not a bell, not silence.
    #[test]
    fn a_reporting_program_asks_finishes_and_fails_like_an_agent() {
        use State::*;
        for (after, kind) in
            [(NeedsYou, Kind::NeedsYou), (Done, Kind::AgentDone), (Failed, Kind::AgentFailed)]
        {
            assert_eq!(on_state(Working, after, Source::Program), Change::Post(kind), "{after:?}");
            assert_eq!(on_state(Working, after, Source::Program), on_state(Working, after, Source::Agent));
        }
        assert!(Source::Program.told() && Source::Agent.told() && !Source::Plain.told());
    }

    #[test]
    fn only_a_failure_the_user_did_not_cause_counts() {
        assert_eq!(failed_exit(Some(1)), Some(1));
        assert_eq!(failed_exit(Some(101)), Some(101));
        assert_eq!(failed_exit(Some(-1)), Some(-1));
        assert_eq!(failed_exit(Some(0)), None);
        assert_eq!(failed_exit(None), None, "the shell gave no code");
        assert_eq!(failed_exit(Some(130)), None, "Ctrl-C is the user's own doing");
        assert_eq!(failed_exit(Some(131)), Some(131));
    }

    #[test]
    fn a_command_is_told_only_past_the_threshold_and_never_for_editors_or_agents() {
        let long = Duration::from_secs(12);
        assert!(command_finished("cargo build --release", long, 10, false));
        assert!(!command_finished("cargo build --release", Duration::from_secs(9), 10, false));
        assert!(command_finished("make", Duration::from_secs(10), 10, false), "the threshold counts");
        assert!(!command_finished("vim src/main.rs", long, 10, false), "you were typing in it");
        assert!(!command_finished("npm run dev", long, 10, false), "a dev server is stopped, not done");
        assert!(!command_finished("cargo test", long, 10, true), "an agent's own work");
        assert!(!command_finished("  ", long, 10, false));
        // A failure is held to the same rules.
        assert!(command_failed("cargo build", long, 10, false));
        assert!(!command_failed("cargo build", Duration::from_secs(2), 10, false), "too quick to tell");
        assert!(!command_failed("vim src/main.rs", long, 10, false));
        assert!(!command_failed("cargo build", long, 10, true));
    }

    #[test]
    fn the_dock_badge_counts_by_its_mode() {
        assert_eq!(dock_badge(DockBadge::Off, 3, 2), None);
        assert_eq!(dock_badge(DockBadge::NeedsYou, 2, 5).as_deref(), Some("2"));
        assert_eq!(dock_badge(DockBadge::NeedsYou, 0, 5), None, "working and finished never badge");
        assert_eq!(dock_badge(DockBadge::NeedsYouDone, 2, 1).as_deref(), Some("3"));
        assert_eq!(dock_badge(DockBadge::NeedsYouDone, 0, 0), None);
        assert_eq!(dock_badge(DockBadge::NeedsYou, 1200, 0).as_deref(), Some("999+"));
    }

    #[test]
    fn the_dock_bounces_once_for_needs_you_while_away() {
        assert!(bounces(Kind::NeedsYou, false, true));
        assert!(!bounces(Kind::NeedsYou, true, true), "Wormhole is in front");
        assert!(!bounces(Kind::NeedsYou, false, false), "switched off");
        for kind in KINDS.into_iter().filter(|kind| *kind != Kind::NeedsYou) {
            assert!(!bounces(kind, false, true), "{kind:?}");
        }
    }

    #[test]
    fn ids_name_their_tab_and_threads_their_terminal() {
        assert_eq!(notice_tab(&notice_id(42)), Some(42));
        assert_ne!(notice_id(1), notice_id(2));
        assert_eq!(notice_tab(super::SUMMARY_ID), None);
        assert_eq!(notice_tab("gpui-component/notification/abc"), None);
        assert_eq!(notice_tab("wormhole/tab/x"), None);
        assert_eq!(thread_id(7), thread_id(7));
        assert_ne!(thread_id(7), thread_id(8));
    }
}
