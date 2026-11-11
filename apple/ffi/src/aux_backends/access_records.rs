//! The agent-access question as Swift reads it (`AuxModel/Access/AccessRequest.swift`): the program's facts as
//! data, never sentences (those are the catalog's, put together by `AccessDisplay`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AccessCapability {
    Read,
    Type,
    Run,
    Open,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AccessKindRecord {
    Scripts,
    Script,
    Unsigned,
    AdHoc,
    DeveloperId,
    Apple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AccessSignerKind {
    DeveloperId,
    AppleSystem,
    AdHoc,
    Other,
    Unsigned,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessProgramRecord {
    /// With the home folder as `~`.
    pub path: String,
    pub pid: u32,
    pub signer: AccessSignerKind,
    pub subject: Option<String>,
    pub team: Option<String>,
    /// For a script: the interpreter it runs on.
    pub interpreter: Option<String>,
    pub started_by: Option<String>,
}

/// One question waiting for the person.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessQuestionRecord {
    pub id: u64,
    pub label: String,
    pub capability: AccessCapability,
    pub kind: AccessKindRecord,
    pub program: Option<AccessProgramRecord>,
    /// The terminal (its id in the workspace) when the question is about one terminal the program did not open.
    pub about_terminal: Option<i64>,
    pub unusual_signer: bool,
    pub changed_since_allowed: bool,
    pub elevated: bool,
    /// Whether "Always" is offered.
    pub can_be_remembered: bool,
}

/// The question about a mode widened outside Wormhole: the words of the mode that stays in force and of the one asked for.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessModeRecord {
    /// Tells this question from an earlier one when it is answered.
    pub token: u64,
    pub previous: String,
    pub requested: String,
}

/// The buttons that settle a question (`sheet::Button` of the GPUI app, without the mode-change pair).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum AccessAnswer {
    /// Not now, and not asked again for two minutes.
    Deny,
    /// While the program's process runs.
    Session,
    /// Until Wormhole quits.
    UntilQuit,
    /// For good, in the profile's database.
    Always,
}

/// An approval in force, for the list in Settings.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessHeldRecord {
    pub id: u64,
    pub label: String,
    pub until_quit: bool,
}
