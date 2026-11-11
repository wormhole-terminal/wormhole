//! What the status machine is told, and what it asks back.

use wormhole_term::program_status::Summary;
use wormhole_term::{Ended, Mark};

use crate::agents::AgentId;

use super::super::facts::Millis;

/// What a look at the terminal's process found: the system's answer about who holds it. The shell
/// asks the system (a few cheap calls, `Pty::program_in_foreground` and friends) and hands the answer
/// over whole, so the machine never touches a process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForegroundLook {
    /// A program other than the shell holds the terminal; `None` where the platform can't tell.
    pub in_front: Option<bool>,
    /// The program in front as a row spells it, when one is (`agents::foreground_command`).
    pub command: Option<Front>,
    /// The coding agent among the foreground processes (`agents::foreground_agent`).
    pub agent: Option<AgentId>,
    /// The shell itself holds the terminal (`agents::shell_in_foreground`).
    pub shell_in_front: bool,
}

/// The program in front and since when, on the workspace's clock. The moment is a stable one (the
/// process's start), so the same program looked at twice is the same.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Front {
    pub command: String,
    pub since: Millis,
}

/// One thing that happened to a terminal that may change its status or its row. Each says what it is,
/// not what to do about it: the rules are the machine's.
#[derive(Clone, Debug, PartialEq)]
pub enum StatusEvent {
    /// The program printed (the PTY's coalesced `Output`).
    Output,
    /// The program set its title (OSC 0/2), raw.
    Title(String),
    ResetTitle,
    Bell,
    /// The program sent a notification of its own (OSC 9/99/777): the bell with words.
    Notice {
        title: String,
        body: String,
    },
    /// A notification about the terminal sent through the control line (`wh notify`): told in its
    /// words like a program's own, without touching the status (the caller said something, the
    /// terminal did not ring to ask for you).
    ControlNotice {
        title: String,
        body: String,
    },
    /// The shell marked a command's life (OSC 133). `ended` is what a finished command was (its
    /// line, how long it ran, its last line), as far as the shell's blocks can say.
    Mark {
        mark: Mark,
        ended: Option<Ended>,
    },
    /// What programs report about themselves (OSC 7501), read again. `by_report`: the records changed
    /// because a program said something (rather than a prompt, the program's exit or the person
    /// looking dropping some).
    Reported {
        summary: Option<Summary>,
        by_report: bool,
    },
    /// An agent's hook: its name and the event as `integrations::describe` spelled it
    /// (`Stop`, `Notification:permission_prompt`, `PostToolUse:wakes=b7`), and the words it carries.
    Hook {
        agent: AgentId,
        event: String,
        message: Option<String>,
    },
    /// The process was looked at (every few seconds, and after what may have changed it).
    Foreground(ForegroundLook),
    /// The shell ended.
    Exited,
    /// The person typed (or pasted), which makes the program echo.
    Typed,
    /// A key went to the program while its agent's question may be up: Return or a choice answers it.
    Key {
        key: String,
        control: bool,
    },
    Resized,
    /// The program was told of a focus change.
    FocusReported,
    /// The person looked: a result is spent, a bell heard.
    Seen,
    /// The terminal's program finished something nobody had looked at before the app last quit, and
    /// it is the same program now (its session host was attached again): its "Done" comes back.
    DoneUnseen,
    /// Time passed to a deadline the machine asked for (`next_due`).
    Tick,
}

/// What a terminal's place in the window adds to what the machine decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    /// The terminal is on screen: the selected one, or shown beside it in its split.
    pub on_screen: bool,
    /// The window is the one in front.
    pub window_active: bool,
}

impl Context {
    /// On screen in the window in front: what the person looks at.
    pub fn looking(self) -> bool {
        self.on_screen && self.window_active
    }
}

/// What the machine needs done that only a shell can do (it holds the PTY), after which the answer
/// comes back as a [`StatusEvent::Reported`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The program that held the terminal left: drop what only it could say
    /// (`ProgramStatus::on_process_exit`) and read the records again.
    ProgramLeft,
    /// The person saw a finished or failed report: drop it (`ProgramStatus::acknowledge`) and read
    /// the records again.
    AcknowledgeReports,
}
