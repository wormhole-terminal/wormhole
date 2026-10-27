//! The values some settings take, as the file spells them: the theme mode, size units, which windows
//! count as in front, the sound, badge and icon choices, Option as Meta, the title border, the branch
//! color and a shell profile. `Settings` holds them; each reads and writes by its documented name.

use serde::{Deserialize, Serialize};

/// Which theme a window shows: always the dark pick (`theme`), always the light
/// pick (`theme_light`), or whichever matches the system's appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
    #[default]
    System,
}

/// How file sizes are written: like Finder and Get Info (12.4 MB, 48.2 KB), or like git
/// and `du -h` (11.8 MiB, 47.1 KiB).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeUnits {
    #[default]
    Decimal,
    Binary,
}

/// What a notification looks like while Wormhole is the front app: a system banner, or nothing
/// (the sidebar, the Dock badge, the menu bar item and ⌥⌘J carry the status). Older builds had
/// an in-window toast as well: a config.json that still says `"toast"` reads as nothing rather
/// than failing to load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InFront {
    Banner,
    #[default]
    #[serde(alias = "toast")]
    None,
}

/// Which notifications play a sound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotifySound {
    #[default]
    NeedsYou,
    All,
    None,
}

/// What the Dock icon's badge counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockBadge {
    Off,
    #[default]
    NeedsYou,
    /// Terminals that need input and agents that finished while you were away.
    NeedsYouDone,
}

/// What the menu bar item shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuBarIcon {
    /// The status dot and the number of terminals that need input.
    #[default]
    StatusCount,
    /// The status dot only.
    Status,
    /// The terminal symbol alone.
    Plain,
}

/// What the global hotkey (`hotkey`) does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyAction {
    /// The quick panel over the current app.
    #[default]
    QuickPanel,
    /// Bring Wormhole to the front, or hide it.
    ToggleApp,
}

/// Which Option keys type Meta (Esc + key) instead of the character the layout composes.
/// One side as Meta keeps the other for `@ { [` on layouts that need Option for them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OptionAsMeta {
    #[default]
    Off,
    Left,
    Right,
    Both,
}

impl OptionAsMeta {
    pub const ALL: [OptionAsMeta; 4] =
        [OptionAsMeta::Off, OptionAsMeta::Left, OptionAsMeta::Right, OptionAsMeta::Both];

    /// Whether a key typed with these Option keys held is Meta. When the side can't be told
    /// (`left` and `right` both false: a synthetic event, a platform without the flags), any
    /// setting but Off counts.
    pub fn is_meta(self, left: bool, right: bool) -> bool {
        match self {
            OptionAsMeta::Off => false,
            OptionAsMeta::Both => true,
            _ if !left && !right => true,
            OptionAsMeta::Left => left,
            OptionAsMeta::Right => right,
        }
    }
}

/// Reads the names, and also `true` / `false` from before the setting had sides (the token
/// list's rule: `true` is both): every `config.json` written so far has the boolean, and a
/// mistyped key would leave the whole file unreadable.
impl<'de> Deserialize<'de> for OptionAsMeta {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Bool(bool),
            Name(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Bool(true) => Ok(OptionAsMeta::Both),
            Raw::Bool(false) => Ok(OptionAsMeta::Off),
            Raw::Name(name) => match name.as_str() {
                "off" => Ok(OptionAsMeta::Off),
                "left" => Ok(OptionAsMeta::Left),
                "right" => Ok(OptionAsMeta::Right),
                "both" => Ok(OptionAsMeta::Both),
                other => Err(serde::de::Error::unknown_variant(other, &["off", "left", "right", "both"])),
            },
        }
    }
}

/// When the line under the terminal's title row is drawn: always, only while the terminal
/// has output scrolled under the bar (the bar then blurs it, and the line says where the bar
/// ends), or never.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleBorder {
    Always,
    #[default]
    Scrolled,
    Never,
}

impl TitleBorder {
    pub const ALL: [TitleBorder; 3] = [TitleBorder::Always, TitleBorder::Scrolled, TitleBorder::Never];

    /// Whether the line is drawn, given whether content is scrolled under the bar.
    pub fn shows(self, content_under: bool) -> bool {
        match self {
            TitleBorder::Always => true,
            TitleBorder::Scrolled => content_under,
            TitleBorder::Never => false,
        }
    }
}

/// Where the git branch's color comes from: the theme's own `git.branch`, the primary color
/// in effect, or `branch_color_custom`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchColor {
    #[default]
    Theme,
    Primary,
    Custom,
}

impl BranchColor {
    pub const ALL: [BranchColor; 3] = [BranchColor::Theme, BranchColor::Primary, BranchColor::Custom];
}

/// A shell profile as `config.json` lists it under `shells`: a program to offer for new terminals
/// and tabs (see `shells`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellProfile {
    /// What menus call it, and the key a tab remembers it by.
    pub name: String,
    /// The program: a path, or a bare name found on the login shell's PATH.
    pub command: String,
    /// Its arguments. Missing, the program starts as a login shell (`-l`); `[]` starts it bare.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
}

impl ShellProfile {
    /// Whether it names a program at all.
    pub fn is_usable(&self) -> bool {
        !self.name.trim().is_empty() && !self.command.trim().is_empty()
    }
}

/// Who may control Wormhole through the automation API (`wh`, `wh mcp`, a script on the socket), once
/// the Automation feature is on: nobody beyond `list` and `status`, each program once it has been
/// asked and the user said yes, or every program while Wormhole is open. The one setting that replaces
/// `control_write` (and the retired `control_read`) for the feature (`agent-permissions.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentAccess {
    /// Nothing is let in beyond what needs no approval (`list`, `status`, `notify`, `doctor`).
    Off,
    /// A program asks the first time it needs more; the answer is remembered as the user chose. The
    /// default, also for everybody who never turned the old switches on.
    #[default]
    Ask,
    /// Every program is let in without asking while Wormhole runs. Hidden terminals and command
    /// rules still apply.
    Open,
}

impl AgentAccess {
    /// The mode as the file spells it.
    pub fn word(self) -> &'static str {
        match self {
            AgentAccess::Off => "off",
            AgentAccess::Ask => "ask",
            AgentAccess::Open => "open",
        }
    }
}
