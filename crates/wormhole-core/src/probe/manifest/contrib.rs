//! What a Probe contributes (`[[contributes.*]]`) and what it asks for (`[permissions]`), as the
//! manifest spells them. Closed sets of values are enums, so a typo is a parse error naming the
//! choices; everything with a rule that needs more than the spelling (ids, caps, patterns, paths)
//! stays text here and is judged by `validate`, which can say where and why.

use std::collections::BTreeMap;

use serde::Deserialize;

/// Keys the structs below do not know. They are kept, not dropped, so the loader can warn about a
/// misspelt key and `validate` can refuse it.
pub(crate) type Unknown = BTreeMap<String, toml::Value>;

/// How a rule draws the text it matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Look {
    /// Keep part of the text (`keep`) and show the rest on `reveal`.
    Shorten,
    /// A solid block.
    Redact,
    /// Draw nothing of its own: only the tone and the action.
    #[default]
    None,
}

/// When a shortened or redacted text shows itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reveal {
    Hover,
    /// An option-click.
    Deliberate,
    Never,
}

/// What a rule does on ⌘-click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    OpenUrl,
    Copy,
    /// Runs one of the Probe's own commands with the captures as arguments.
    Command,
    None,
}

/// A rule's action as one table: the fields each type reads are told apart by `validate`, which
/// can then name a field a type does not use.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Action {
    #[serde(rename = "type")]
    pub kind: ActionKind,
    /// `open-url`: the link, with `{n}` for a capture.
    pub url: Option<String>,
    /// `command`: the local id of one of this Probe's commands.
    pub command: Option<String>,
    /// `copy`: what is copied, with `{n}` for a capture; the whole match when absent.
    pub text: Option<String>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// `[[contributes.rules]]`: text the terminal shows, found by a regular expression.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rule {
    pub id: String,
    pub pattern: String,
    #[serde(default)]
    pub look: Look,
    /// `start:N`, `end:N` and `domain`, comma-separated: see [`KeepSpec`].
    pub keep: Option<String>,
    pub reveal: Option<Reveal>,
    /// A design token's name, never a colour.
    pub tone: Option<String>,
    pub hover: Option<String>,
    pub action: Option<Action>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// Where a command's terminal opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunIn {
    /// The user's shell, with its output visible.
    #[default]
    NewTerminal,
    /// Spawned by Wormhole itself: no shell, no terminal.
    Background,
}

/// The folder a command or status item starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cwd {
    /// The focused terminal's shell-confirmed folder.
    #[default]
    Terminal,
    /// The Probe's own folder.
    Probe,
}

/// `[[contributes.commands]]`: a palette entry.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    /// The program and its arguments. Absent: handled by code (E2).
    pub run: Option<Vec<String>>,
    #[serde(rename = "in", default)]
    pub run_in: RunIn,
    #[serde(default)]
    pub cwd: Cwd,
    /// A shortcut; checked once Probe keys have a layer in the keymap.
    pub key: Option<String>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// Which side of the status bar an item sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Left,
    #[default]
    Right,
}

/// `[[contributes.status]]`: an item fed by a command's first line.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StatusItem {
    pub id: String,
    /// Absent: fed by code (E2).
    pub run: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Cwd,
    /// Seconds between runs.
    pub interval: Option<u64>,
    /// Seconds before the process is killed.
    pub timeout: Option<u64>,
    #[serde(default)]
    pub side: Side,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// How a coding agent reports what it is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// The program reports itself in OSC 7501; no file is written anywhere.
    #[default]
    Osc7501,
    /// Hook entries in the agent's own configuration.
    Hooks,
}

/// `[[contributes.agents]]`: a coding agent Wormhole should recognise.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AgentDef {
    pub id: String,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub executables: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    /// Two capitals, drawn where there is no icon.
    pub monogram: String,
    /// An SVG inside the Probe's folder.
    pub icon: Option<String>,
    #[serde(default)]
    pub status: AgentStatus,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// `[[contributes.themes]]` and `[[contributes.icon_themes]]`: a file the existing loaders read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FileContribution {
    pub id: String,
    /// The file, relative to the Probe's folder.
    pub path: String,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// `[[contributes.panels]]` and `[[contributes.hovers]]`: slots that only code (E2) fills.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CodeSlot {
    pub id: String,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// What `[permissions] terminal` may do (E2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalAccess {
    Read,
    Write,
}

/// `[permissions]`: what the Probe asks for. A Probe can use nothing it did not declare.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Permissions {
    /// Programs it may run: a bare name, or `./path` inside its folder.
    #[serde(default)]
    pub process: Vec<String>,
    /// Variables a background command may see.
    #[serde(default)]
    pub env: Vec<String>,
    /// Hosts a rule's link may open.
    #[serde(default)]
    pub open: Vec<String>,
    #[serde(default)]
    pub notify: bool,
    /// Hooks it may write for its own agents: `install-hooks:<agent id>`.
    #[serde(default)]
    pub agents: Vec<String>,
    /// E2.
    #[serde(default)]
    pub terminal: Vec<TerminalAccess>,
    /// E2: hosts it may reach over `https`.
    #[serde(default)]
    pub net: Vec<String>,
    /// E2: `read:<path>` and `write:<path>`.
    #[serde(default)]
    pub fs: Vec<String>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// `[[contributes.*]]`.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct Contributes {
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub status: Vec<StatusItem>,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
    #[serde(default)]
    pub themes: Vec<FileContribution>,
    #[serde(default)]
    pub icon_themes: Vec<FileContribution>,
    #[serde(default)]
    pub panels: Vec<CodeSlot>,
    #[serde(default)]
    pub hovers: Vec<CodeSlot>,
    #[serde(flatten)]
    pub(crate) unknown: Unknown,
}

/// The pieces of a rule's `keep`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeepSpec {
    /// Keep the part after the last `@`.
    pub domain: bool,
    /// Keep this many characters from the start (1 to 8).
    pub start: Option<u8>,
    /// Keep this many characters from the end (1 to 8).
    pub end: Option<u8>,
}

impl KeepSpec {
    /// Parse `start:1,domain`: the pieces in any order, each at most once, N from 1 to 8.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut spec = Self::default();
        for piece in text.split(',').map(str::trim) {
            match piece.split_once(':') {
                None if piece == "domain" => {
                    if std::mem::replace(&mut spec.domain, true) {
                        return Err("`domain` twice".into());
                    }
                }
                Some((side @ ("start" | "end"), n)) => {
                    let n: u8 = n
                        .parse()
                        .ok()
                        .filter(|n| (1..=8).contains(n))
                        .ok_or_else(|| format!("`{side}:{n}`: N is a number from 1 to 8"))?;
                    let slot = if side == "start" { &mut spec.start } else { &mut spec.end };
                    if slot.replace(n).is_some() {
                        return Err(format!("`{side}` twice"));
                    }
                }
                _ => return Err(format!("`{piece}` is not `domain`, `start:N` or `end:N`")),
            }
        }
        Ok(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_takes_the_three_pieces_in_any_order() {
        assert_eq!(
            KeepSpec::parse("start:1,domain"),
            Ok(KeepSpec { domain: true, start: Some(1), end: None })
        );
        assert_eq!(
            KeepSpec::parse(" end:8 , start:2 "),
            Ok(KeepSpec { domain: false, start: Some(2), end: Some(8) })
        );
    }

    #[test]
    fn keep_refuses_what_it_does_not_know() {
        for bad in ["", "start", "start:0", "start:9", "start:x", "start:1,start:2", "domain,domain", "mid:1"]
        {
            assert!(KeepSpec::parse(bad).is_err(), "{bad:?}");
        }
    }
}
