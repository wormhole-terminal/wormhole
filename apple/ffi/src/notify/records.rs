//! The notification rules' values as Swift reads and writes them. Plain records; times are milliseconds on
//! two clocks the caller passes in (a monotonic one, and the Unix wall clock), never read here.

use wormhole_core::config::{DockBadge, InFront, NotifySound, Settings};
use wormhole_core::notify::{Kind, Level, Source, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoticeStateRecord {
    Idle,
    Working,
    NeedsYou,
    Done,
    Failed,
    Exited,
}

impl From<NoticeStateRecord> for State {
    fn from(state: NoticeStateRecord) -> Self {
        match state {
            NoticeStateRecord::Idle => State::Idle,
            NoticeStateRecord::Working => State::Working,
            NoticeStateRecord::NeedsYou => State::NeedsYou,
            NoticeStateRecord::Done => State::Done,
            NoticeStateRecord::Failed => State::Failed,
            NoticeStateRecord::Exited => State::Exited,
        }
    }
}

/// What tells a terminal's state (`notify::Source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoticeSourceRecord {
    Plain,
    Agent,
    Program,
}

impl From<Source> for NoticeSourceRecord {
    fn from(source: Source) -> Self {
        match source {
            Source::Plain => Self::Plain,
            Source::Agent => Self::Agent,
            Source::Program => Self::Program,
        }
    }
}

impl From<NoticeSourceRecord> for Source {
    fn from(source: NoticeSourceRecord) -> Self {
        match source {
            NoticeSourceRecord::Plain => Source::Plain,
            NoticeSourceRecord::Agent => Source::Agent,
            NoticeSourceRecord::Program => Source::Program,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoticeKindRecord {
    NeedsYou,
    AgentDone,
    CommandDone,
    CommandFailed,
    AgentFailed,
    Bell,
    Notice,
}

impl From<Kind> for NoticeKindRecord {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::NeedsYou => Self::NeedsYou,
            Kind::AgentDone => Self::AgentDone,
            Kind::CommandDone => Self::CommandDone,
            Kind::CommandFailed => Self::CommandFailed,
            Kind::AgentFailed => Self::AgentFailed,
            Kind::Bell => Self::Bell,
            Kind::Notice => Self::Notice,
        }
    }
}

impl From<NoticeKindRecord> for Kind {
    fn from(kind: NoticeKindRecord) -> Self {
        match kind {
            NoticeKindRecord::NeedsYou => Kind::NeedsYou,
            NoticeKindRecord::AgentDone => Kind::AgentDone,
            NoticeKindRecord::CommandDone => Kind::CommandDone,
            NoticeKindRecord::CommandFailed => Kind::CommandFailed,
            NoticeKindRecord::AgentFailed => Kind::AgentFailed,
            NoticeKindRecord::Bell => Kind::Bell,
            NoticeKindRecord::Notice => Kind::Notice,
        }
    }
}

/// How hard a banner interrupts (`UNNotificationInterruptionLevel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoticeLevelRecord {
    Passive,
    Active,
    TimeSensitive,
}

impl From<Level> for NoticeLevelRecord {
    fn from(level: Level) -> Self {
        match level {
            Level::Passive => Self::Passive,
            Level::Active => Self::Active,
            Level::TimeSensitive => Self::TimeSensitive,
        }
    }
}

/// Which buttons a banner carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoticeCategoryRecord {
    NeedsYou,
    Finished,
    Plain,
}

/// One banner, ready for the notification centre.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NoticeRecord {
    pub id: String,
    pub thread: String,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub level: NoticeLevelRecord,
    pub sound: bool,
    pub category: NoticeCategoryRecord,
}

/// What the person's settings say about being told, per profile (the fields of `Settings` the rules read).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct NotifySettingsRecord {
    pub notifications: bool,
    pub needs_you: bool,
    pub agent_done: bool,
    pub command_done: bool,
    pub command_failed: bool,
    /// A command is told only past this many seconds.
    pub command_after_secs: u32,
    pub bell: bool,
    pub only_unseen: bool,
    /// A banner while Wormhole is in front, else nothing (the window carries the status).
    pub banner_in_front: bool,
    pub sound: SoundChoiceRecord,
    pub group_bursts: bool,
    pub dock_bounce: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SoundChoiceRecord {
    NeedsYou,
    All,
    None,
}

impl From<&Settings> for NotifySettingsRecord {
    fn from(settings: &Settings) -> Self {
        Self {
            notifications: settings.notifications,
            needs_you: settings.notify_needs_you,
            agent_done: settings.notify_agent_done,
            command_done: settings.notify_command_done,
            command_failed: settings.notify_command_failed,
            command_after_secs: settings.notify_command_after,
            bell: settings.notify_bell,
            only_unseen: settings.notify_only_unseen,
            banner_in_front: settings.notify_in_front == InFront::Banner,
            sound: match settings.notify_sound {
                NotifySound::NeedsYou => SoundChoiceRecord::NeedsYou,
                NotifySound::All => SoundChoiceRecord::All,
                NotifySound::None => SoundChoiceRecord::None,
            },
            group_bursts: settings.notify_group_bursts,
            dock_bounce: settings.dock_bounce,
        }
    }
}

impl NotifySettingsRecord {
    /// Core's `Settings` with these fields and every other field at its default: the rules read nothing else.
    pub(super) fn to_core(&self) -> Settings {
        Settings {
            notifications: self.notifications,
            notify_needs_you: self.needs_you,
            notify_agent_done: self.agent_done,
            notify_command_done: self.command_done,
            notify_command_failed: self.command_failed,
            notify_command_after: self.command_after_secs,
            notify_bell: self.bell,
            notify_only_unseen: self.only_unseen,
            notify_in_front: if self.banner_in_front { InFront::Banner } else { InFront::None },
            notify_sound: match self.sound {
                SoundChoiceRecord::NeedsYou => NotifySound::NeedsYou,
                SoundChoiceRecord::All => NotifySound::All,
                SoundChoiceRecord::None => NotifySound::None,
            },
            notify_group_bursts: self.group_bursts,
            dock_bounce: self.dock_bounce,
            ..Settings::default()
        }
    }
}

/// What the Dock icon's badge counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DockBadgeModeRecord {
    Off,
    NeedsYou,
    NeedsYouDone,
}

impl From<DockBadgeModeRecord> for DockBadge {
    fn from(mode: DockBadgeModeRecord) -> Self {
        match mode {
            DockBadgeModeRecord::Off => DockBadge::Off,
            DockBadgeModeRecord::NeedsYou => DockBadge::NeedsYou,
            DockBadgeModeRecord::NeedsYouDone => DockBadge::NeedsYouDone,
        }
    }
}

/// A program's own notification (OSC 9, 99, 777), numbered so a new one is told even without a state change.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProgramNoticeRecord {
    pub serial: u64,
    pub title: String,
    pub body: String,
}

/// A command's failing exit code, until the next command starts.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FailureRecord {
    pub serial: u64,
    pub code: i32,
    /// When it was told, on the monotonic clock.
    pub at_ms: u64,
    pub command: Option<String>,
    pub took_ms: Option<u64>,
    pub last_line: Option<String>,
}

/// The program in the terminal's foreground and when it started (Unix milliseconds).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ForegroundRecord {
    pub command: String,
    pub started_unix_ms: u64,
}

/// One terminal as read on this look (the app's `TabFacts`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TabFactsRecord {
    /// The terminal's id as its session has it, and what its hooks report.
    pub session: i64,
    pub tab: i64,
    pub name: String,
    pub state: NoticeStateRecord,
    /// When it started doing that, on the monotonic clock.
    pub since_ms: u64,
    /// The coding agent's name, or the name of the program that reports its own status.
    pub who: Option<String>,
    pub source: NoticeSourceRecord,
    pub message: Option<String>,
    /// The state is what a program reported about itself (OSC 7501), which anyone may send.
    pub reported: bool,
    pub failure: Option<FailureRecord>,
    pub notice: Option<ProgramNoticeRecord>,
    pub foreground: Option<ForegroundRecord>,
    pub folder: Option<String>,
    pub branch: Option<String>,
    pub changed: Option<u32>,
    /// On screen in its window, and its window is the key window.
    pub selected: bool,
    pub key: bool,
    /// The settings of the profile the terminal is in; `None`: the look's own.
    pub settings: Option<NotifySettingsRecord>,
}

/// What the person's attention is owed, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum NotifyActionRecord {
    /// Post (or replace, by id) a banner.
    Post { notice: NoticeRecord },
    /// Take a banner down, pending or delivered.
    Retract { id: String },
    /// One informational Dock bounce.
    Bounce,
    /// Something for VoiceOver to say.
    Say { text: String, urgent: bool },
}

/// What a click on a banner, or one of its buttons, asks of the app.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ResponseKindRecord {
    /// Not one of ours, or nothing to do.
    Nothing,
    /// Bring Wormhole forward (the summary, the test banner).
    Activate,
    /// Bring the terminal forward.
    Open { tab: i64 },
    /// Bring the terminal forward and show its Changes.
    ShowChanges { tab: i64 },
    /// Type `text` into the terminal and press Return, without bringing anything forward.
    Reply { tab: i64, text: String },
}

/// A click on one of our banners or its buttons.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResponseRecord {
    pub id: String,
    /// `open`, `reply`, `show-changes`; nil for a click on the banner itself.
    pub action: Option<String>,
    /// What was typed into Reply.
    pub reply: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResponseOutcomeRecord {
    /// The banner to take down when the click did not already (nil: it was not up).
    pub retract: Option<String>,
    pub kind: ResponseKindRecord,
}
