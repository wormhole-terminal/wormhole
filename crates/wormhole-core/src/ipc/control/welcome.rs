//! The welcome's part of the control protocol: what the program `Wormhole welcome` asks of the app
//! that runs it (`Action::Welcome`), and the facts the app answers with.
//!
//! The welcome is a program in a terminal of the app, and the app is the one that can do things: read
//! the machine's agents and the theme, write a theme or an agent's hooks, ask for Touch ID, remember
//! that the first run is over. So every choice goes over the control socket.
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
//! Nothing here installs or trusts behind the person's back: hooks are written for exactly the agent
//! an `InstallHooks` names, which the program sends only from that agent's own Install press, and a
//! folder is trusted only from a `Trust`, which shows Touch ID. Neither is ever part of moving
//! through the steps (`welcome::model` and its tests drive every path to show it).
//!
//! The mutating requests have budgets like the other actions' (`limits`), and every string is capped
//! here, so a program that goes wrong cannot flood the app.

use serde::{Deserialize, Serialize};

use super::{Error, ErrorCode};
use crate::agents::AgentId;
use crate::intro::Handover;

/// The longest token, folder or theme name a request may carry.
pub const FIELD_MAX_BYTES: usize = 4096;

/// What the program asks of the app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WelcomeOp {
    /// Everything the pages are made of ([`WelcomeState`]). `folder` is the folder chosen so far,
    /// whose trust the agents page shows: the program names it only on that page, and without one
    /// there are no trust facts (answering them reads the project's files).
    State {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
    },
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
    /// Write the status hooks of exactly this agent. Answered at once; the result is in the next
    /// [`WelcomeState`] (the agent's hooks, or the `problem`).
    InstallHooks { agent: AgentId },
    /// Ask to trust exactly this folder's project: the app shows Touch ID. Answered at once; the
    /// result is in the next [`WelcomeState`].
    Trust { folder: String },
    /// The welcome is over. On the first run the app remembers it; `folder` is where the shell is
    /// about to start, `skipped` whether Esc ended it.
    Finish {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        skipped: bool,
    },
}

impl WelcomeOp {
    /// What the op is called in the log.
    pub fn name(&self) -> &'static str {
        match self {
            WelcomeOp::State { .. } => "state",
            WelcomeOp::SetLook { .. } => "set_look",
            WelcomeOp::InstallHooks { .. } => "install_hooks",
            WelcomeOp::Trust { .. } => "trust",
            WelcomeOp::Finish { .. } => "finish",
        }
    }

    /// The strings in the op are within [`FIELD_MAX_BYTES`], the font step is one step.
    pub fn check_caps(&self) -> Result<(), Error> {
        let too_long = |what: &str| Error {
            code: ErrorCode::TooLarge,
            message: format!("{what} is longer than {FIELD_MAX_BYTES} bytes"),
        };
        let long = |text: &Option<String>| text.as_ref().is_some_and(|text| text.len() > FIELD_MAX_BYTES);
        match self {
            WelcomeOp::State { folder } | WelcomeOp::Finish { folder, .. } if long(folder) => {
                Err(too_long("the folder"))
            }
            WelcomeOp::Trust { folder } if folder.len() > FIELD_MAX_BYTES => Err(too_long("the folder")),
            WelcomeOp::SetLook { theme, .. } if long(theme) => Err(too_long("the theme's name")),
            WelcomeOp::SetLook { primary, .. } if long(primary) => Err(too_long("the primary colour")),
            WelcomeOp::SetLook { font_step: Some(step), .. } if !(-1..=1).contains(step) => Err(Error {
                code: ErrorCode::BadRequest,
                message: "the font moves one step at a time".to_string(),
            }),
            _ => Ok(()),
        }
    }
}

/// The app's answer to [`WelcomeOp::State`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WelcomeState {
    /// How long until the text should start, from the animation's clock: nothing when no animation
    /// plays (a replay without one, a window that was not shown) or under Reduce Motion.
    pub wait_ms: u64,
    pub reduce_motion: bool,
    pub handover: Handover,
    /// The first run, which finishing is remembered of, or a replay.
    pub first_run: bool,
    /// The folders the second step offers.
    pub places: Vec<PlaceFact>,
    pub look: LookFacts,
    /// `None` while the agents are still being looked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<AgentsFacts>,
    /// The chosen folder's project, when it has one that is not trusted yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<TrustFacts>,
    /// What went wrong with the last Install or Trust.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceFact {
    /// "Desktop", or a recent folder's name.
    pub name: String,
    pub path: String,
    /// Where it is, with the home folder as `~`.
    pub detail: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustFacts {
    /// The project's root.
    pub folder: String,
    /// Its name, and the folder it keeps its files in (`.wormhole`).
    pub name: String,
    pub dir: String,
    /// How many commands its actions run (they wait for trust).
    pub commands: usize,
    /// The Touch ID prompt is up.
    pub asking: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caps_refuse_what_is_too_long_or_too_big() {
        let long = "x".repeat(FIELD_MAX_BYTES + 1);
        let ops = [
            WelcomeOp::State { folder: Some(long.clone()) },
            WelcomeOp::Trust { folder: long.clone() },
            WelcomeOp::Finish { folder: Some(long.clone()), skipped: false },
            WelcomeOp::SetLook { theme: Some(long.clone()), primary: None, ligatures: None, font_step: None },
            WelcomeOp::SetLook { theme: None, primary: Some(long), ligatures: None, font_step: None },
        ];
        for op in ops {
            assert_eq!(op.check_caps().map_err(|e| e.code), Err(ErrorCode::TooLarge), "{op:?}");
        }
        let jump = WelcomeOp::SetLook { theme: None, primary: None, ligatures: None, font_step: Some(40) };
        assert_eq!(jump.check_caps().map_err(|e| e.code), Err(ErrorCode::BadRequest));
        let fine = WelcomeOp::SetLook {
            theme: Some("Nord".into()),
            primary: Some(String::new()),
            ligatures: Some(true),
            font_step: Some(-1),
        };
        assert!(fine.check_caps().is_ok());
        assert!(WelcomeOp::State { folder: None }.check_caps().is_ok());
    }

    #[test]
    fn an_ops_wire_form_is_stable() {
        let lines = [
            (WelcomeOp::State { folder: None }, r#"{"kind":"state"}"#),
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
                WelcomeOp::InstallHooks { agent: AgentId::Claude },
                r#"{"kind":"install_hooks","agent":"claude"}"#,
            ),
            (WelcomeOp::Trust { folder: "/work/app".into() }, r#"{"kind":"trust","folder":"/work/app"}"#),
            (
                WelcomeOp::Finish { folder: Some("/work/app".into()), skipped: true },
                r#"{"kind":"finish","folder":"/work/app","skipped":true}"#,
            ),
            (WelcomeOp::Finish { folder: None, skipped: false }, r#"{"kind":"finish"}"#),
        ];
        for (op, line) in lines {
            assert_eq!(serde_json::to_string(&op).expect("serializes"), line);
            assert_eq!(serde_json::from_str::<WelcomeOp>(line).expect("parses"), op);
        }
    }

    #[test]
    fn only_a_missing_or_partial_hook_has_a_button() {
        assert!(HooksFact::Missing.installable() && HooksFact::Partial.installable());
        assert!(!HooksFact::Installed.installable() && !HooksFact::Unsupported.installable());
    }
}
