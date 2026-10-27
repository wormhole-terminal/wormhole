//! The welcome's part of the control protocol: what the program `Wormhole welcome` asks of the app
//! that runs it (`Action::Welcome`), and the facts the app answers with.
//!
//! The welcome is a program in a terminal of the app, and the app is the one that can do things: read
//! the machine's agents and the theme, write a theme or an agent's hooks, change what reaches you,
//! remember that the first run is over. So every choice goes over the control socket.
//!
//! ## Who may ask
//!
//! These requests are not behind `control_write`: that switch is for a script the user did not start,
//! and this program is started by the app itself for the first run. What stands in for the switch is a
//! token. The app makes a random one for each welcome terminal and puts it in that terminal's
//! environment only (`WORMHOLE_WELCOME_TOKEN`). A request is honoured when it comes from that
//! terminal (`from`, its `WORMHOLE_TAB_ID`), carries that token, and only while that welcome runs
//! (`Gate`): once it has finished, or the terminal is gone, the token is worth nothing. A hostile
//! program in another terminal has neither the token nor the tab; one in this terminal after the
//! welcome has ended finds the gate shut. The program removes the token from the environment before
//! it hands the terminal to the shell.
//!
//! Nothing here installs or changes what reaches you behind the person's back: hooks are written for
//! exactly the agent an `InstallHooks` names, which the program sends only from that agent's own Install
//! press, and a `SetNotify` goes out only from a press on a notification row (the only presses that make
//! macOS ask for permission are the ones that turn a banner on, and the page's own Allow button). None
//! is ever part of moving through the steps (`welcome::model` and its tests drive every path to show it).
//!
//! `SetShown` changes how much the window around the terminal shows (the title row's parts, the status
//! bar, the minimap, the lines between the columns, the right panel and its tools), as the intro
//! window's Look step does. It carries what the walk showed before the answer (`from`) and what it shows
//! with it (`shown`), and the app writes only the parts where the two differ, so a part the person
//! changed or arranged themselves meanwhile is never overwritten by a value they did not give. Every
//! setting but the right panel is the app's (all windows); the panel is the one window's. It is safe to
//! take from the program for the same reason as the others: only a press on an answer (or Start over)
//! sends it, it names no file, command or path, every value is a switch or one of a handful of names
//! read strictly (a name this build doesn't know refuses the request), and the app refuses a title row
//! or a set of tools of the person's own unless it is the one `from` already had, so the request can
//! never write an arrangement the walk doesn't offer.
//!
//! `Demo` is the one request that only shows something: while the sidebar step is up the app draws a
//! small example band in the window's sidebar (never a terminal: not listed, not saved, not numbered)
//! and takes it away when the step is left or the welcome ends.
//!
//! The mutating requests have budgets like the other actions' (`limits`), and every string is capped
//! here, so a program that goes wrong cannot flood the app.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{Error, ErrorCode};
use crate::agents::AgentId;
use crate::config::NotifySound;
use crate::config::presets::{Shown, ToolsShown};

/// The longest theme name or colour a request may carry.
pub const FIELD_MAX_BYTES: usize = 4096;

/// The longest terminal key an [`Import`](WelcomeOp::Import) may name (the longest real one is
/// `terminal-app`).
const SOURCE_MAX_BYTES: usize = 32;

/// What the program asks of the app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WelcomeOp {
    /// Everything the pages are made of ([`WelcomeState`]).
    State,
    /// Change the look. Only what is named changes: a theme (by name), the primary colour (by hex,
    /// from the ones [`LookFacts::primaries`] offers; empty is the theme's own), the ligatures, and the
    /// terminal font's size by one step either way.
    SetLook {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        theme: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        primary: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ligatures: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font_step: Option<i8>,
    },
    /// Change what reaches you, as Settings → Notifications does. Only what is named changes: whether
    /// a banner comes when an agent needs you, when one finishes, and which notifications play a
    /// sound. Turning a banner on while macOS has not been asked yet makes the app ask; so does
    /// `allow`, the page's own button for it. Nothing else ever does.
    SetNotify {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        needs_you: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent_done: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sound: Option<NotifySound>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        allow: bool,
    },
    /// Change how much the window around the terminal shows from `from` (what the walk showed before
    /// the answer) to `shown` (with it). The app writes only the parts where the two differ: the title
    /// row, the status bar, the minimap and the lines between the columns as settings, the tools as the
    /// features' switches, and the right panel as the window's own layout. Only a press on an answer (or
    /// on Start over) sends it. Answered with a [`ShownDone`]. A program only sends it to an app that
    /// reports what its window shows ([`WelcomeState::shown`]); an app from before this op does not.
    SetShown { from: Shown, shown: Shown },
    /// Write the status hooks of exactly this agent. Answered at once; the result is in the next
    /// [`WelcomeState`] (the agent's hooks, or the `problem`).
    InstallHooks { agent: AgentId },
    /// Show (`show`) or take away the example band in the window's sidebar, which the sidebar step
    /// points at. The program sends it when the step is entered and left; the app also drops it when
    /// the welcome finishes or its terminal goes.
    Demo { show: bool },
    /// Bring over the settings of the terminal the person used before (`source`: an `import::Source`'s
    /// key, `ghostty`, `iterm2`, …), which the look page found and offered. Only a press on that
    /// offer sends it. The app reads that one terminal's settings itself (it never takes values from
    /// the request), applies what Settings → Import ticks by default in one settings update, and
    /// answers with an [`ImportDone`]. An app from before this op cannot read it and answers
    /// `bad_request`; the page shows that as a note and goes on.
    Import { source: String },
    /// The welcome is over. On the first run the app remembers it; `skipped` is whether Esc ended it.
    Finish {
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skipped: bool,
    },
}

impl WelcomeOp {
    /// What the op is called in the log.
    pub fn name(&self) -> &'static str {
        match self {
            WelcomeOp::State => "state",
            WelcomeOp::SetLook { .. } => "set_look",
            WelcomeOp::SetNotify { .. } => "set_notify",
            WelcomeOp::SetShown { .. } => "set_shown",
            WelcomeOp::InstallHooks { .. } => "install_hooks",
            WelcomeOp::Demo { .. } => "demo",
            WelcomeOp::Import { .. } => "import",
            WelcomeOp::Finish { .. } => "finish",
        }
    }

    /// The strings in the op are within [`FIELD_MAX_BYTES`], the font step is one step, and a title row
    /// or a set of tools of the person's own has no more parts than there are, none twice.
    pub fn check_caps(&self) -> Result<(), Error> {
        let too_long = |what: &str| {
            Error::new(ErrorCode::TooLarge, format!("{what} is longer than {FIELD_MAX_BYTES} bytes"))
        };
        let long = |text: &Option<String>| text.as_ref().is_some_and(|text| text.len() > FIELD_MAX_BYTES);
        match self {
            WelcomeOp::SetLook { theme, .. } if long(theme) => Err(too_long("the theme's name")),
            WelcomeOp::SetLook { primary, .. } if long(primary) => Err(too_long("the primary colour")),
            WelcomeOp::SetLook { font_step: Some(step), .. } if !(-1..=1).contains(step) => {
                Err(Error::new(ErrorCode::BadRequest, "the font moves one step at a time"))
            }
            WelcomeOp::Import { source } if source.len() > SOURCE_MAX_BYTES => {
                Err(too_long("the terminal's name"))
            }
            WelcomeOp::SetShown { from, shown } => check_shown(from).and_then(|()| check_shown(shown)),
            _ => Ok(()),
        }
    }
}

/// What a [`Shown`] may carry: the tools of the person's own, each at most once. (Their number, and a
/// title row's, are bounded as they are read, and the title row is whole by construction.)
fn check_shown(shown: &Shown) -> Result<(), Error> {
    if let ToolsShown::Own(on) = &shown.tools {
        let mut seen = HashSet::new();
        if !on.iter().all(|id| seen.insert(*id)) {
            return Err(Error::new(ErrorCode::BadRequest, "a tool is named twice"));
        }
    }
    Ok(())
}

/// The app's answer to [`WelcomeOp::SetShown`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShownDone {
    /// Whether the window's right panel is on screen now. A window too narrow for it keeps it hidden
    /// whatever was asked, and the walk shows what is true.
    pub panel_open: bool,
}

/// The app's answer to [`WelcomeOp::Import`]: what the import did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDone {
    /// Settings taken.
    pub brought: usize,
    /// Settings read that were not taken: unticked, or with no Wormhole setting to go in.
    pub left: usize,
}

/// The app's answer to [`WelcomeOp::State`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WelcomeState {
    /// How long until the text should start, from the animation's clock: nothing when no animation
    /// plays (a replay without one, a window that was not shown) or under Reduce Motion.
    pub wait_ms: u64,
    pub reduce_motion: bool,
    /// The hand-over the animation plays, by its key (`intro::Handover::key`: "cursor", "portal",
    /// "logo"), which is also how the enum is written in JSON; the program reads it back with
    /// `Handover::parse`.
    pub handover: String,
    /// The first run, which finishing is remembered of, or a replay.
    pub first_run: bool,
    pub look: LookFacts,
    /// What reaches you now; `None` for a tour, which has no app to ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<NotifyFacts>,
    /// `None` while the agents are still being looked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentsFacts>,
    /// How much the welcome terminal's window shows now; `None` for a tour, from an app that does not
    /// know it, and when the window's settings are not the ones being edited (another profile's): then
    /// the page is read-only, as in a tour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shown: Option<Shown>,
    /// What went wrong with the last Install.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// What the look page offers and what is chosen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookFacts {
    pub themes: Vec<ThemeFact>,
    /// The theme in use, by name.
    pub theme: String,
    pub primaries: Vec<PrimaryFact>,
    /// The primary colour in use: its hex, or empty for the theme's own.
    pub primary: String,
    /// The terminal font's family and size.
    pub font: String,
    pub font_size: f32,
    pub ligatures: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeFact {
    pub name: String,
    /// A light theme (the list marks it).
    pub light: bool,
}

/// A primary colour on offer: the theme's own, or one of its terminal colours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryFact {
    /// "Theme", "Green", …
    pub name: String,
    /// What `SetLook` sends for it: empty for the theme's own.
    pub hex: String,
    /// Its place in the terminal's 16 colours, which the program draws it with (so it follows the
    /// theme); `None` for the theme's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ansi: Option<u8>,
}

/// What the notifications page offers and what is chosen now (Settings → Notifications' own switches,
/// read through its master switch: with notifications off nothing is on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyFacts {
    /// A banner when an agent needs you.
    pub needs_you: bool,
    /// A banner when an agent finishes.
    pub agent_done: bool,
    pub sound: NotifySound,
    pub permission: PermissionFact,
}

/// Whether macOS lets Wormhole notify, as the page words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionFact {
    /// Still being read.
    Checking,
    /// macOS has not been asked: the page offers the button.
    NotAsked,
    Allowed,
    /// Off in System Settings, where only the person can turn it on.
    Denied,
    /// A build that is not the app bundle (`cargo run`, a test instance): banners go another way.
    Outside,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsFacts {
    /// The agents on PATH.
    pub found: Vec<AgentFact>,
    /// How many more the app recognises that are not installed.
    pub others: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFact {
    pub id: AgentId,
    pub name: String,
    pub hooks: HooksFact,
    /// The file the hooks go in (`~/.claude/settings.json`), when the agent has hooks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// An agent's hooks as the page shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HooksFact {
    Installed,
    /// Some of ours are missing: Repair.
    Partial,
    /// Supported, not installed: Install.
    Missing,
    /// The agent has no hooks; its status is read from its output.
    Unsupported,
}

impl HooksFact {
    /// Whether the page offers a button for it.
    pub fn installable(self) -> bool {
        matches!(self, HooksFact::Missing | HooksFact::Partial)
    }

    /// What the row says of them.
    pub fn word(self) -> &'static str {
        match self {
            HooksFact::Installed => "Hooks in",
            HooksFact::Partial => "Needs repair",
            HooksFact::Missing => "Not installed",
            HooksFact::Unsupported => "Status from its output",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::presets::{Preset, TitleShown};
    use crate::view::layout::title_parts::TitlePart;

    #[test]
    fn the_caps_refuse_what_is_too_long_or_too_big() {
        let long = "x".repeat(FIELD_MAX_BYTES + 1);
        let ops = [
            WelcomeOp::SetLook { theme: Some(long.clone()), primary: None, ligatures: None, font_step: None },
            WelcomeOp::SetLook { theme: None, primary: Some(long), ligatures: None, font_step: None },
        ];
        for op in ops {
            assert_eq!(op.check_caps().map_err(|e| e.code), Err(ErrorCode::TooLarge), "{op:?}");
        }
        let long_source = WelcomeOp::Import { source: "x".repeat(SOURCE_MAX_BYTES + 1) };
        assert_eq!(long_source.check_caps().map_err(|e| e.code), Err(ErrorCode::TooLarge));
        assert!(WelcomeOp::Import { source: "terminal-app".into() }.check_caps().is_ok());
        let jump = WelcomeOp::SetLook { theme: None, primary: None, ligatures: None, font_step: Some(40) };
        assert_eq!(jump.check_caps().map_err(|e| e.code), Err(ErrorCode::BadRequest));
        let fine = WelcomeOp::SetLook {
            theme: Some("Nord".into()),
            primary: Some(String::new()),
            ligatures: Some(true),
            font_step: Some(-1),
        };
        assert!(fine.check_caps().is_ok());
        assert!(WelcomeOp::State.check_caps().is_ok());
    }

    #[test]
    fn a_shown_of_ones_own_is_bounded_and_read_strictly() {
        use crate::config::feature_table::FeatureId;
        use crate::config::presets::Preset;
        let set = |shown: Shown| WelcomeOp::SetShown { from: shown.clone(), shown };
        for preset in Preset::ALL {
            assert!(set(Shown::of(preset)).check_caps().is_ok(), "{preset:?}");
        }
        let own =
            |tools: Vec<FeatureId>| set(Shown { tools: ToolsShown::Own(tools), ..Shown::of(Preset::Zen) });
        assert!(own(vec![FeatureId::History, FeatureId::Notes]).check_caps().is_ok());
        assert!(own(Vec::new()).check_caps().is_ok());
        let twice = own(vec![FeatureId::History, FeatureId::History]);
        assert_eq!(twice.check_caps().map_err(|e| e.code), Err(ErrorCode::BadRequest));
        // Off the wire a name that is no part is refused, not left out, and so is a list longer than
        // there are parts; a part named twice within that is the part once.
        let wire = |parts: &str| {
            let shown = r#""status_bar":false,"minimap":false,"column_dividers":false,"panel_open":false,"tools":"few""#;
            let shown = format!(r#"{{"title":{{"own":[{parts}]}},{shown}}}"#);
            serde_json::from_str::<WelcomeOp>(&format!(
                r#"{{"kind":"set_shown","from":{shown},"shown":{shown}}}"#
            ))
        };
        assert!(wire(r#""nope""#).is_err());
        assert!(wire(&[r#""folder""#; 12].join(",")).is_err());
        let op = wire(r#""folder","folder""#).expect("parses");
        let WelcomeOp::SetShown { shown: Shown { title: TitleShown::Own(row), .. }, .. } = &op else {
            panic!("{op:?}")
        };
        assert_eq!(row.parts(), [TitlePart::Folder, TitlePart::Space]);
        assert!(op.check_caps().is_ok());
    }

    #[test]
    fn an_ops_wire_form_is_stable() {
        let lines = [
            (WelcomeOp::State, r#"{"kind":"state"}"#),
            (
                WelcomeOp::SetLook {
                    theme: Some("Nord".into()),
                    primary: None,
                    ligatures: Some(false),
                    font_step: None,
                },
                r#"{"kind":"set_look","theme":"Nord","ligatures":false}"#,
            ),
            (
                WelcomeOp::SetNotify {
                    needs_you: Some(true),
                    agent_done: None,
                    sound: Some(NotifySound::NeedsYou),
                    allow: false,
                },
                r#"{"kind":"set_notify","needs_you":true,"sound":"needs_you"}"#,
            ),
            (
                WelcomeOp::SetNotify { needs_you: None, agent_done: None, sound: None, allow: true },
                r#"{"kind":"set_notify","allow":true}"#,
            ),
            (
                WelcomeOp::InstallHooks { agent: AgentId::Claude },
                r#"{"kind":"install_hooks","agent":"claude"}"#,
            ),
            (
                WelcomeOp::SetShown { from: Shown::of(Preset::Balanced), shown: Shown::of(Preset::Zen) },
                r#"{"kind":"set_shown","from":{"title":"everything","status_bar":false,"minimap":false,"column_dividers":true,"panel_open":false,"tools":"all"},"shown":{"title":"name","status_bar":false,"minimap":false,"column_dividers":false,"panel_open":false,"tools":"all"}}"#,
            ),
            (WelcomeOp::Demo { show: true }, r#"{"kind":"demo","show":true}"#),
            (WelcomeOp::Import { source: "ghostty".into() }, r#"{"kind":"import","source":"ghostty"}"#),
            (WelcomeOp::Finish { skipped: true }, r#"{"kind":"finish","skipped":true}"#),
            (WelcomeOp::Finish { skipped: false }, r#"{"kind":"finish"}"#),
        ];
        for (op, line) in lines {
            assert_eq!(serde_json::to_string(&op).expect("serializes"), line);
            assert_eq!(serde_json::from_str::<WelcomeOp>(line).expect("parses"), op);
        }
    }

    #[test]
    fn the_answer_to_a_shown_is_where_the_panel_ended_up() {
        let line = serde_json::to_string(&ShownDone { panel_open: true }).expect("serializes");
        assert_eq!(line, r#"{"panel_open":true}"#);
    }

    #[test]
    fn the_answer_to_an_import_is_two_counts() {
        let done = ImportDone { brought: 11, left: 7 };
        let line = serde_json::to_string(&done).expect("serializes");
        assert_eq!(line, r#"{"brought":11,"left":7}"#);
        assert_eq!(serde_json::from_str::<ImportDone>(&line).expect("parses"), done);
    }

    #[test]
    fn only_a_missing_or_partial_hook_has_a_button() {
        assert!(HooksFact::Missing.installable() && HooksFact::Partial.installable());
        assert!(!HooksFact::Installed.installable() && !HooksFact::Unsupported.installable());
    }
}
