//! The settings schema: every key of `config.json`, its default and its
//! valid range.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::features::{self, FeatureFlag};
use crate::folder_color::FolderColor;
use crate::fonts::{self, FontSlot};
use crate::project::ProjectColorChoice;
use crate::sidebar::{Density, SidebarNumbers, TerminalsIn};
use crate::tokens::Color;

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
    /// Terminals that need you and agents that finished while you were away.
    NeedsYouDone,
}

/// What the menu bar item shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuBarIcon {
    /// The status dot and the number of terminals that need you.
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// A built-in theme name, or a path to a Ghostty / iTerm2 / Alacritty theme file.
    pub theme: String,
    /// Replaces the theme's primary (the primary color: buttons, focus, selection,
    /// links) with this `#rrggbb`; empty keeps the theme's own.
    pub primary_color: String,
    pub font_family: String,
    pub font_size: f32,
    /// Combine sequences like `=>` and `!=` into single glyphs (in fonts that
    /// have them, like JetBrains Mono) in the terminal and code text.
    pub font_ligatures: bool,
    pub line_height: f32,
    /// Background opacity, 0.3..=1.0.
    pub opacity: f32,
    pub blur: bool,
    /// How much room each terminal takes in the sidebar and the tab bar.
    pub density: Density,
    /// Groups are cards in the sidebar; off, a group is its head and rows, with no card round them.
    pub show_groups_as_cards: bool,
    /// What `density` replaced: one-line rows. Read from older files (see [`Settings::sanitized`]),
    /// never written.
    #[serde(skip_serializing)]
    #[doc(hidden)]
    pub compact_sidebar: bool,
    /// Which Option keys send Esc+key (a side set to Meta can't type @ { [ on some layouts).
    pub option_as_meta: OptionAsMeta,
    pub scrollback_lines: usize,
    /// macOS notifications when a background session finishes or needs you.
    pub notifications: bool,
    /// Changes panel: show files as a folder tree instead of a flat list.
    pub changes_tree: bool,
    /// Changes panel: list untracked files in their own group.
    pub group_untracked: bool,
    /// Diff view: wrap long lines instead of scrolling horizontally.
    pub diff_wrap: bool,
    /// Diff view: old and new side by side (split) instead of one column (unified).
    pub diff_split: bool,
    /// Diff view: columns per tab stop.
    pub tab_width: usize,
    /// Diff view: file sizes in Finder's decimal units (MB) or git's binary ones (MiB).
    pub size_units: SizeUnits,
    /// File icon theme for the Changes / History panels.
    pub icon_theme: String,
    /// Show the menu bar (status bar) item.
    pub menu_bar: bool,
    /// System-wide shortcut to show/hide Wormhole, e.g. "ctrl-`" (empty = off).
    pub hotkey: String,
    /// Check for and download updates automatically.
    pub auto_update: bool,

    // Windows
    /// Settings window: translucent (window opacity + blur) like the main window.
    pub settings_translucent: bool,
    /// Theme Studio window: translucent like the main window.
    pub studio_translucent: bool,

    // Fonts outside the terminal (the terminal uses `font_family` / `font_size`).
    /// Interface font (sidebar, panels, settings); empty = the system font.
    pub ui_font_family: String,
    pub ui_font_size: f32,
    /// Code font (diff view, notes, Theme Studio samples); empty = the terminal font.
    pub code_font_family: String,
    pub code_font_size: f32,

    // Sidebar session rows
    /// Agent status dot.
    pub sidebar_status_dot: bool,
    /// Number of changed files.
    pub sidebar_changes: bool,
    /// Note icon with the number of open notes and tasks.
    pub sidebar_notes: bool,
    /// What ⌘1–9 count along: the terminal (a number stays with it) or the list (the first nine
    /// rows you can see, up to 0.4).
    pub sidebar_numbers: SidebarNumbers,
    /// Where every window lists its terminals: the sidebar, or a tab bar instead of it.
    pub terminals_in: TerminalsIn,
    /// The colour of a folder's terminals' title row: the folder's path (absolute) and the colour
    /// (a name: `red`, `teal` … see [`FolderColor`]; or a hex colour the picker made, `#rrggbb`).
    /// One list for every window; the Project tab sets it, there is no row for it in Settings. A
    /// name this build doesn't know stays in the file and colours nothing.
    pub folder_colors: BTreeMap<String, String>,
    /// Holding ⌘ shows the ⌘1–9 keycaps on the sidebar's terminals.
    pub sidebar_cmd_hints: bool,

    // Look
    /// Dark, light, or following the system's appearance.
    pub theme_mode: ThemeMode,
    /// The theme for light mode (`theme` is the dark one).
    pub theme_light: String,

    // Window and terminal
    /// The status bar along the bottom of every window (View → Show Status Bar).
    pub status_bar: bool,
    /// The minimap along every terminal's right edge (View → Show Minimap).
    pub minimap: bool,
    /// The line under the terminal's title row: always, only while content is under the bar, never.
    pub title_border: TitleBorder,
    /// The app the title row's Open in… button opens a terminal's folder in, by its bundle
    /// identifier (`dev.zed.Zed`); empty picks the first editor found on this Mac, else Finder.
    /// An app that isn't installed any more falls back the same way.
    pub open_in: String,
    /// The title row's extras (`title_extras`), each on its own switch: a dot and the ahead and
    /// behind counts beside the branch, an agent's model and how full its context window is, the
    /// ports a terminal's programs listen on, and the branch's pull request (which also puts its
    /// mark on the folder's band and tells when its checks finish).
    pub title_branch_status: bool,
    pub title_agent_gauge: bool,
    pub title_ports: bool,
    pub title_pull_request: bool,
    /// A system beep when a terminal rings its bell.
    pub bell_sound: bool,
    /// A brief glow along the terminal's edge when it rings its bell.
    pub bell_visual: bool,
    /// What a new terminal or tab runs unless chosen otherwise: `""` the login shell (`$SHELL`),
    /// a profile's name from `shells`, or a program's path (started as a login shell).
    pub default_shell: String,
    /// Shells to offer besides the login shell and the system's (`/etc/shells`).
    pub shells: Vec<ShellProfile>,

    // Notifications, Dock and menu bar (`notifications` is the master switch)
    /// When an agent needs you.
    pub notify_needs_you: bool,
    /// When an agent finishes.
    pub notify_agent_done: bool,
    /// When a command that ran for at least `notify_command_after` finishes.
    pub notify_command_done: bool,
    /// When a command that ran for at least `notify_command_after` ends with a non-zero exit code
    /// (needs a shell with integration).
    pub notify_command_failed: bool,
    /// Seconds a command must run before its end is worth a notification, 5..=300.
    pub notify_command_after: u32,
    /// When a terminal without an agent rings its bell.
    pub notify_bell: bool,
    /// Only for terminals you aren't looking at.
    pub notify_only_unseen: bool,
    /// What a notification looks like while Wormhole is in front.
    pub notify_in_front: InFront,
    pub notify_sound: NotifySound,
    /// Several notifications in a few seconds become one summary.
    pub notify_group_bursts: bool,
    pub dock_badge: DockBadge,
    /// One bounce of the Dock icon when a terminal needs you.
    pub dock_bounce: bool,
    pub menu_bar_icon: MenuBarIcon,
    /// Animate the menu bar item while an agent works.
    pub menu_bar_animate: bool,
    pub hotkey_action: HotkeyAction,
    /// Jump to the terminal that has waited longest, e.g. "alt-cmd-j".
    pub jump_waiting_key: String,

    // The control command line
    /// Let `Wormhole send-text` type into terminals and `Wormhole new-terminal` open them (and
    /// start programs). Off, the command line can only list, report and notify.
    pub control_write: bool,
    /// Answer a program's request for the checksum of a rectangle of the screen (DECRQCRA). Off:
    /// it lets any program, also one over ssh, read the screen back cell by cell. Terminals started
    /// after the change use it.
    pub allow_screen_checksum: bool,
    /// Let programs act on the window (move, resize, minimise, raise, full screen: XTWINOPS) and
    /// read its title back. Off: a program can only ask for sizes and the window's place. Terminals
    /// started after the change use it.
    pub allow_window_ops: bool,
    /// Where New Terminal in New Worktree puts a worktree: under this folder (in a folder named
    /// after the repository), or, empty, beside the repository in `<name>-worktrees`.
    pub worktree_base: String,

    /// Features still behind a flag (see [`crate::features`]). The key is a [`FeatureFlag`]'s
    /// `key`, the value whether it is on. Only a development build reads this; a release ignores
    /// it and uses each flag's release default. A key this build doesn't know stays in the file.
    pub feature_flags: BTreeMap<String, bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "Tokyo Night".into(),
            primary_color: String::new(),
            // Only fresh configs: config.json is written out in full on first
            // launch, so existing ones keep the font they have.
            font_family: fonts::bundled::FAMILY.into(),
            font_size: 14.0,
            font_ligatures: true,
            line_height: 1.3,
            opacity: 0.8,
            blur: true,
            density: Density::default(),
            show_groups_as_cards: true,
            compact_sidebar: false,
            option_as_meta: OptionAsMeta::Off,
            scrollback_lines: 10_000,
            notifications: true,
            changes_tree: true,
            group_untracked: true,
            diff_wrap: false,
            diff_split: true,
            tab_width: 4,
            size_units: SizeUnits::Decimal,
            icon_theme: "Wormhole Icons".into(),
            menu_bar: true,
            hotkey: String::new(),
            auto_update: true,
            settings_translucent: false,
            studio_translucent: false,
            ui_font_family: String::new(),
            ui_font_size: 13.0,
            code_font_family: String::new(),
            code_font_size: 13.0,
            sidebar_status_dot: true,
            sidebar_changes: true,
            sidebar_notes: false,
            sidebar_numbers: SidebarNumbers::default(),
            terminals_in: TerminalsIn::default(),
            folder_colors: BTreeMap::new(),
            sidebar_cmd_hints: true,
            theme_mode: ThemeMode::default(),
            theme_light: "Catppuccin Latte".into(),
            status_bar: false,
            minimap: false,
            title_border: TitleBorder::default(),
            open_in: String::new(),
            title_branch_status: true,
            title_agent_gauge: true,
            title_ports: true,
            title_pull_request: true,
            bell_sound: true,
            bell_visual: true,
            default_shell: String::new(),
            shells: Vec::new(),
            notify_needs_you: true,
            notify_agent_done: true,
            notify_command_done: true,
            notify_command_failed: true,
            notify_command_after: 10,
            notify_bell: false,
            notify_only_unseen: true,
            notify_in_front: InFront::default(),
            notify_sound: NotifySound::default(),
            notify_group_bursts: true,
            dock_badge: DockBadge::default(),
            dock_bounce: true,
            menu_bar_icon: MenuBarIcon::default(),
            menu_bar_animate: false,
            hotkey_action: HotkeyAction::default(),
            jump_waiting_key: "alt-cmd-j".into(),
            control_write: false,
            allow_screen_checksum: false,
            allow_window_ops: false,
            worktree_base: String::new(),
            feature_flags: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Whether the feature behind `flag` is on in this build: a development build reads
    /// `feature_flags` (a flag with no entry defaults on), a release build uses the flag's
    /// release default (see [`crate::features`]).
    pub fn feature_enabled(&self, flag: FeatureFlag) -> bool {
        features::enabled_in(flag, &self.feature_flags, features::dev_build())
    }
}

/// Settings a project's `.wormhole/config.json` may change (how its sessions
/// look and behave while one is in front). The app applies only these keys of
/// a project overlay; every other setting is in [`APP_SETTINGS`]. The theme is not
/// among them: a project brings its own as `.wormhole/theme.json`.
pub const PROJECT_SETTINGS: &[&str] = &[
    "primary_color",
    "font_family",
    "font_size",
    "font_ligatures",
    "line_height",
    "opacity",
    "blur",
    "density",
    "show_groups_as_cards",
    "option_as_meta",
    "scrollback_lines",
    "changes_tree",
    "group_untracked",
    "diff_wrap",
    "diff_split",
    "tab_width",
    "icon_theme",
    "ui_font_family",
    "ui_font_size",
    "code_font_family",
    "code_font_size",
    "sidebar_status_dot",
    "sidebar_changes",
    "sidebar_notes",
    "sidebar_cmd_hints",
    "bell_sound",
    "bell_visual",
];

/// Settings that are about the app as a whole, which only the user's own
/// `config.json` sets: bringing another project's session to the front must not
/// rebind the global hotkey, hide the menu bar item, turn off updates,
/// restyle the settings and Studio windows, silence notifications about
/// other sessions, or show the status bar in one window and not the next. The
/// theme picks are here too: a project's theme is its `theme.json`, one way to
/// theme a project instead of two that could disagree.
pub const APP_SETTINGS: &[&str] = &[
    "theme",
    "theme_mode",
    "theme_light",
    "hotkey",
    "menu_bar",
    "auto_update",
    "notifications",
    "settings_translucent",
    "studio_translucent",
    "status_bar",
    "minimap",
    // The person's own habit: what ⌘1–9 reach, whatever project is in front.
    "sidebar_numbers",
    // The window's chrome, one look for every window (where the terminals are listed too); and the
    // programs the person opens folders in.
    "terminals_in",
    "title_border",
    "open_in",
    // What the title row shows is the person's own choice. The pull request also runs `gh`, which
    // a repository must not be able to switch on for itself.
    "title_branch_status",
    "title_agent_gauge",
    "title_ports",
    "title_pull_request",
    "notify_needs_you",
    "notify_agent_done",
    "notify_command_done",
    "notify_command_failed",
    "notify_command_after",
    "notify_bell",
    "notify_only_unseen",
    "notify_in_front",
    "notify_sound",
    "notify_group_bursts",
    "dock_badge",
    "dock_bounce",
    "menu_bar_icon",
    "menu_bar_animate",
    "hotkey_action",
    "jump_waiting_key",
    // How the person reads sizes, not something a project should change.
    "size_units",
    // Programs to run are the person's own choice; a project names its shell with `shell`.
    "default_shell",
    "shells",
    // Whether scripts may type into terminals and open them: a cloned repository must not be
    // able to turn that on for itself.
    "control_write",
    // Whether programs may read the screen back with DECRQCRA: a cloned repository must not be
    // able to turn that on for itself.
    "allow_screen_checksum",
    // Whether programs may move and resize the window and read its title: a cloned repository must
    // not be able to turn that on for itself.
    "allow_window_ops",
    // Where worktrees go is the person's own folder layout; a repository must not pick it.
    "worktree_base",
    // Which colour each folder wears is the person's too: a repository can tint its own title row
    // (`project_color`), not recolour the folders in everyone's windows.
    "folder_colors",
    // Which in-progress features are on is the person's own, dev-build choice, never a project's.
    "feature_flags",
];

/// Every key a settings file can have, sorted.
pub(crate) fn keys() -> Vec<String> {
    // Sorted here: whether a `Map` keeps insertion order depends on serde_json's
    // features, which other crates in a build can turn on.
    let mut keys: Vec<String> = match serde_json::to_value(Settings::default()) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        _ => Vec::new(),
    };
    keys.sort();
    keys
}

impl Settings {
    /// The primary chosen in `primary_color`, if it is set to a valid color.
    pub fn primary(&self) -> Option<Color> {
        Color::from_hex(&self.primary_color).ok().map(|c| c.with_alpha(1.0))
    }

    /// A problem with `primary_color` to show the user.
    pub(super) fn primary_error(&self) -> Option<String> {
        let value = self.primary_color.trim();
        if value.is_empty() {
            return None;
        }
        Color::from_hex(value).err().map(|e| format!("primary_color: {e}"))
    }

    /// These settings with a partial JSON object (e.g. a project's
    /// `.wormhole/config.json`) layered on top. Unknown or mistyped keys are
    /// ignored rather than failing the whole overlay.
    pub fn with_overrides(&self, overrides: &serde_json::Map<String, serde_json::Value>) -> Settings {
        self.with_overrides_reported(overrides).0
    }

    /// [`with_overrides`](Self::with_overrides), and the keys the overlay really set (known
    /// keys whose values read), sorted: what the settings window marks as set by a project.
    pub fn with_overrides_reported(
        &self,
        overrides: &serde_json::Map<String, serde_json::Value>,
    ) -> (Settings, Vec<String>) {
        let Ok(serde_json::Value::Object(mut merged)) = serde_json::to_value(self) else {
            return (self.clone(), Vec::new());
        };
        let mut applied = Vec::new();
        for (key, value) in overrides {
            if let Some(slot) = merged.get_mut(key) {
                let previous = std::mem::replace(slot, value.clone());
                if serde_json::from_value::<Settings>(serde_json::Value::Object(merged.clone())).is_err() {
                    merged.insert(key.clone(), previous);
                } else {
                    applied.push(key.clone());
                }
            }
        }
        applied.sort();
        match serde_json::from_value::<Settings>(serde_json::Value::Object(merged)) {
            Ok(settings) => (settings.sanitized(), applied),
            Err(_) => (self.clone(), Vec::new()),
        }
    }

    /// A notification kind's switch (`notify_needs_you`, …) as it is in effect: with the master switch
    /// `notifications` off nothing is on, whatever the kinds say.
    pub fn notify_kind_on(&self, switch: bool) -> bool {
        self.notifications && switch
    }

    /// Change a notification kind's switch with `change`, and keep the master switch on while any kind
    /// is on (turning every kind off turns it off). Settings → Notifications and the welcome both do
    /// this, so a kind turned on there is never held back by a master switch left off.
    pub fn change_notify_kind(&mut self, change: impl FnOnce(&mut Settings)) {
        change(self);
        self.notifications = self.notify_needs_you
            || self.notify_agent_done
            || self.notify_command_done
            || self.notify_command_failed
            || self.notify_bell;
    }

    /// The colour `folder` wears, if it has one this build knows: a name, or a hex colour the
    /// person picked.
    pub fn folder_color_choice(&self, folder: &Path) -> Option<ProjectColorChoice> {
        ProjectColorChoice::parse(self.folder_colors.get(&folder_color_key(folder))?).ok()
    }

    /// [`Self::folder_color_choice`], for a name only.
    pub fn folder_color(&self, folder: &Path) -> Option<FolderColor> {
        self.folder_color_choice(folder)?.named()
    }

    /// Colour `folder`, or (`None`) take its colour away. Returns whether anything changed.
    pub fn set_folder_color_choice(&mut self, folder: &Path, color: Option<ProjectColorChoice>) -> bool {
        let key = folder_color_key(folder);
        match color {
            Some(color) => {
                let value = color.to_value();
                self.folder_colors.insert(key, value.clone()).as_deref() != Some(value.as_str())
            }
            None => self.folder_colors.remove(&key).is_some(),
        }
    }

    /// [`Self::set_folder_color_choice`], for a name.
    pub fn set_folder_color(&mut self, folder: &Path, color: Option<FolderColor>) -> bool {
        self.set_folder_color_choice(folder, color.map(ProjectColorChoice::Named))
    }

    pub(super) fn sanitized(mut self) -> Self {
        for slot in FontSlot::ALL {
            slot.step_size(&mut self, 0.0);
        }
        self.line_height = self.line_height.clamp(1.0, 2.0);
        self.opacity = self.opacity.clamp(0.3, 1.0);
        self.scrollback_lines = self.scrollback_lines.clamp(100, 200_000);
        self.tab_width = self.tab_width.clamp(1, 16);
        self.notify_command_after = self.notify_command_after.clamp(5, 300);
        if self.font_family.trim().is_empty() {
            self.font_family = Settings::default().font_family;
        }
        // A profile without a name or a program can't be offered or run; of two with one name
        // the first is the one a tab's key means.
        let mut names: Vec<String> = Vec::new();
        self.shells.retain(|profile| {
            let keep = profile.is_usable() && !names.contains(&profile.name);
            if keep {
                names.push(profile.name.clone());
            }
            keep
        });
        // Files from before the three densities say it with one switch.
        if std::mem::take(&mut self.compact_sidebar) && self.density == Density::Regular {
            self.density = Density::Compact;
        }
        // A colour for a path that isn't absolute can never match a folder; the same folder
        // spelt with a trailing slash is one entry.
        let colors = std::mem::take(&mut self.folder_colors);
        for (folder, color) in colors {
            if Path::new(&folder).is_absolute() {
                self.folder_colors.insert(folder_color_key(Path::new(&folder)), color);
            }
        }
        self
    }
}

/// How a folder is named in `folder_colors`: its path without a trailing slash or `.` parts, so
/// the terminal's folder and one typed by hand find the same entry.
fn folder_color_key(folder: &Path) -> String {
    folder.components().collect::<PathBuf>().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use crate::project::ProjectColorChoice;

    use super::{
        APP_SETTINGS, DockBadge, HotkeyAction, InFront, MenuBarIcon, NotifySound, OptionAsMeta,
        PROJECT_SETTINGS, Settings, ShellProfile, SizeUnits, ThemeMode, TitleBorder, keys,
    };
    use crate::folder_color::FolderColor;
    use crate::fonts;
    use crate::jsonc;
    use crate::sidebar::{Density, SidebarNumbers, TerminalsIn};
    use crate::tokens::Color;
    use std::path::Path;

    #[test]
    fn the_master_notification_switch_is_on_while_any_kind_is() {
        let mut s = Settings::default();
        s.change_notify_kind(|s| s.notify_needs_you = false);
        s.change_notify_kind(|s| s.notify_agent_done = false);
        assert!(s.notifications, "commands still notify");
        s.change_notify_kind(|s| s.notify_command_done = false);
        assert!(s.notifications, "a failing command still notifies");
        s.change_notify_kind(|s| s.notify_command_failed = false);
        assert!(!s.notifications, "all off = false (the bell is off by default)");
        assert!(!s.notify_kind_on(true), "with the master off nothing is on, whatever a kind says");
        s.change_notify_kind(|s| s.notify_bell = true);
        assert!(s.notifications && s.notify_kind_on(s.notify_bell));
        assert!(!s.notify_kind_on(false));
    }

    #[test]
    fn every_setting_is_either_for_projects_or_app_wide() {
        let mut classified: Vec<&str> = PROJECT_SETTINGS.iter().chain(APP_SETTINGS).copied().collect();
        classified.sort();
        let keys = keys();
        assert_eq!(classified, keys.iter().map(String::as_str).collect::<Vec<_>>(), "classify new settings");
        let app_wide = [
            "hotkey",
            "menu_bar",
            "auto_update",
            "notifications",
            "settings_translucent",
            "studio_translucent",
            // One switch for every window, so View → Show Status Bar says the same everywhere.
            "status_bar",
            "minimap",
            "sidebar_numbers",
            "dock_badge",
            "hotkey_action",
            "jump_waiting_key",
        ];
        for app_wide in app_wide {
            assert!(!PROJECT_SETTINGS.contains(&app_wide), "{app_wide}");
        }
        for key in keys.iter().filter(|k| k.starts_with("notify_") || k.starts_with("menu_bar")) {
            assert!(APP_SETTINGS.contains(&key.as_str()), "{key} is about the whole app");
        }
    }

    /// A project themes itself with `.wormhole/theme.json`, its one way to: the picks of the
    /// user's own config.json are not a project's to change.
    #[test]
    fn a_project_themes_itself_with_its_theme_file_only() {
        for key in ["theme", "theme_mode", "theme_light"] {
            assert!(!PROJECT_SETTINGS.contains(&key), "{key}");
            assert!(APP_SETTINGS.contains(&key), "{key}");
        }
        assert!(PROJECT_SETTINGS.contains(&"primary_color"), "the primary stays a project's to set");
    }

    #[test]
    fn the_final_design_keys_have_their_defaults() {
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert_eq!(unset, Settings::default());
        assert!(!unset.status_bar, "the status bar is hidden until asked for");
        assert!(!unset.minimap, "so is the minimap");
        assert!(unset.bell_sound && unset.bell_visual);
        assert_eq!(unset.sidebar_numbers, SidebarNumbers::Terminal);
        assert!(unset.sidebar_cmd_hints);
        assert_eq!(unset.terminals_in, TerminalsIn::Sidebar);
        assert!(!unset.sidebar_notes, "the rows are drawn without the notes and tasks count");
        assert!(unset.sidebar_changes);
        assert_eq!(unset.theme_mode, ThemeMode::System);
        assert_eq!(unset.theme_light, "Catppuccin Latte");
        assert!(unset.notify_needs_you && unset.notify_agent_done && unset.notify_command_done);
        assert!(unset.notify_command_failed, "a failing command is told unless switched off");
        assert_eq!(unset.notify_command_after, 10);
        assert!(!unset.notify_bell && unset.notify_only_unseen && unset.notify_group_bursts);
        assert_eq!(unset.notify_in_front, InFront::None, "in front, the window itself says nothing");
        assert_eq!(unset.notify_sound, NotifySound::NeedsYou);
        assert_eq!(unset.dock_badge, DockBadge::NeedsYou);
        assert!(unset.dock_bounce && !unset.menu_bar_animate);
        assert_eq!(unset.menu_bar_icon, MenuBarIcon::StatusCount);
        assert_eq!(unset.hotkey_action, HotkeyAction::QuickPanel);
        assert_eq!(unset.jump_waiting_key, "alt-cmd-j");
        assert_eq!(unset.size_units, SizeUnits::Decimal, "sizes read like Finder's unless asked");
        assert_eq!(unset.title_border, TitleBorder::Scrolled, "the line shows once output is under the bar");
        assert!(unset.open_in.is_empty(), "Open in… picks an editor until one is chosen");
        assert!(!unset.control_write, "the command line can't type into terminals until asked");
        assert!(!unset.allow_screen_checksum, "programs can't read the screen back until asked");
        assert!(!unset.allow_window_ops, "programs can't move the window or read its title until asked");
        assert!(unset.worktree_base.is_empty(), "worktrees go beside the repository until asked");
    }

    #[test]
    fn the_title_border_reads_by_name_and_shows_by_rule() {
        let settings: Settings =
            jsonc::parse(r#"{ "title_border": "never", "open_in": "dev.zed.Zed" }"#).expect("valid jsonc");
        assert_eq!(settings.title_border, TitleBorder::Never);
        assert_eq!(settings.open_in, "dev.zed.Zed");
        for (border, under, shown) in [
            (TitleBorder::Always, false, true),
            (TitleBorder::Always, true, true),
            (TitleBorder::Scrolled, false, false),
            (TitleBorder::Scrolled, true, true),
            (TitleBorder::Never, true, false),
        ] {
            assert_eq!(border.shows(under), shown, "{border:?} with content under: {under}");
        }
        // The window's chrome and the programs a person opens: not a repository's to set.
        for key in ["title_border", "open_in"] {
            assert!(APP_SETTINGS.contains(&key) && !PROJECT_SETTINGS.contains(&key), "{key}");
        }
    }

    /// Every extra of the title row is on until the person turns it off, and none of them is a
    /// repository's to switch on (the pull request runs `gh`).
    #[test]
    fn the_title_rows_extras_are_on_by_default_and_the_persons_own() {
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert!(unset.title_branch_status && unset.title_agent_gauge);
        assert!(unset.title_ports && unset.title_pull_request);
        let off: Settings =
            jsonc::parse(r#"{ "title_ports": false, "title_pull_request": false }"#).expect("valid jsonc");
        assert!(!off.title_ports && !off.title_pull_request && off.title_agent_gauge);
        for key in ["title_branch_status", "title_agent_gauge", "title_ports", "title_pull_request"] {
            assert!(APP_SETTINGS.contains(&key) && !PROJECT_SETTINGS.contains(&key), "{key}");
        }
    }

    /// `control_write` lets scripts type into terminals and start programs: only the user's own
    /// config.json may turn it on, never a repository's `.wormhole/config.json`.
    #[test]
    fn a_project_cannot_let_scripts_type_into_terminals() {
        assert!(APP_SETTINGS.contains(&"control_write") && !PROJECT_SETTINGS.contains(&"control_write"));
        assert!(
            APP_SETTINGS.contains(&"allow_screen_checksum")
                && !PROJECT_SETTINGS.contains(&"allow_screen_checksum")
        );
        assert!(
            APP_SETTINGS.contains(&"allow_window_ops") && !PROJECT_SETTINGS.contains(&"allow_window_ops")
        );
        let overrides = serde_json::json!({ "control_write": true });
        let serde_json::Value::Object(map) = overrides else { unreachable!() };
        let (merged, applied) = Settings::default().with_overrides_reported(&map);
        // The merge itself is blind to the lists (the app applies only `PROJECT_SETTINGS`), so
        // what matters is the classification above; the overlay reads like any other key here.
        assert!(applied == ["control_write"] && merged.control_write);
    }

    /// Files from before the three densities have one switch for one-line rows: it becomes Compact,
    /// is never written again, and a density the file does say wins over it.
    #[test]
    fn the_old_compact_switch_becomes_a_density() {
        let read = |text: &str| jsonc::parse::<Settings>(text).expect("valid jsonc").sanitized();
        assert_eq!(read(r#"{ "compact_sidebar": true }"#).density, Density::Compact);
        assert_eq!(read(r#"{ "compact_sidebar": false }"#).density, Density::Regular);
        assert_eq!(read(r#"{ "compact_sidebar": true, "density": "dense" }"#).density, Density::Dense);
        assert_eq!(read("{}").density, Density::Regular);
        assert!(read("{}").show_groups_as_cards, "cards are on until asked");
        let json = serde_json::to_value(read(r#"{ "compact_sidebar": true }"#)).expect("serializes");
        assert!(json.get("compact_sidebar").is_none(), "the old key is not written back");
        assert_eq!(json["density"], "compact");
        assert!(PROJECT_SETTINGS.contains(&"density") && PROJECT_SETTINGS.contains(&"show_groups_as_cards"));
    }

    /// A config.json from before the final sidebar still reads: its grouping, order, tab layout and
    /// pinned folders mean nothing now, and the rest applies as before.
    #[test]
    fn a_config_with_the_old_sidebar_keys_still_reads() {
        let text = r#"{
            "sidebar_group": "folder", "sidebar_order": "classic", "tab_layout": "stacks",
            "pinned_folders": ["/w/a"], "sidebar_numbers": "list", "font_size": 15.0
        }"#;
        let settings: Settings = jsonc::parse(text).expect("old keys are ignored");
        assert_eq!(settings.font_size, 15.0);
        assert_eq!(settings.sidebar_numbers, SidebarNumbers::List);
        let json = serde_json::to_value(&settings).expect("serializes");
        for gone in ["sidebar_group", "sidebar_order", "tab_layout", "pinned_folders"] {
            assert!(json.get(gone).is_none(), "{gone} is no setting any more");
        }
    }

    /// A folder's colour is the person's own, for every window: a repository can't recolour the
    /// folders in everyone's sidebar. It is the name that is stored, by an absolute path.
    #[test]
    fn folder_colours_are_the_apps_own_and_are_kept_by_name_and_absolute_path() {
        assert!(APP_SETTINGS.contains(&"folder_colors") && !PROJECT_SETTINGS.contains(&"folder_colors"));
        assert!(Settings::default().folder_colors.is_empty(), "no folder has a colour until asked");
        let read: Settings = jsonc::parse(
            r#"{ "folder_colors": { "/w/a": "teal", "/w/b/": "red", "~/c": "blue", "rel": "pink", "/w/d": "brown" } }"#,
        )
        .expect("valid jsonc");
        assert_eq!(read.folder_colors.len(), 5, "reading keeps what the file says");
        let kept = read.sanitized();
        assert_eq!(
            kept.folder_colors.keys().map(String::as_str).collect::<Vec<_>>(),
            ["/w/a", "/w/b", "/w/d"],
            "only absolute paths, a trailing slash dropped"
        );
        assert_eq!(kept.folder_color(Path::new("/w/a")), Some(FolderColor::Teal));
        assert_eq!(kept.folder_color(Path::new("/w/b")), Some(FolderColor::Red));
        assert_eq!(
            kept.folder_color(Path::new("/w/d")),
            None,
            "a name this build doesn't know colours nothing"
        );
        assert!(kept.folder_colors.contains_key("/w/d"), "but stays for the build that does");
        assert_eq!(kept.folder_color(Path::new("/w/other")), None);
    }

    #[test]
    fn a_picked_hex_colour_is_kept_for_the_folder_like_a_name() {
        let mut settings = Settings::default();
        let picked = ProjectColorChoice::parse("#2bb3a3").expect("a colour");
        assert!(settings.set_folder_color_choice(Path::new("/w/a"), Some(picked)));
        assert!(!settings.set_folder_color_choice(Path::new("/w/a"), Some(picked)), "nothing changed");
        assert_eq!(settings.folder_colors.get("/w/a").map(String::as_str), Some("#2bb3a3"));
        assert_eq!(settings.folder_color_choice(Path::new("/w/a")), Some(picked));
        assert_eq!(settings.folder_color(Path::new("/w/a")), None, "a hex colour is none of the eight");
    }

    #[test]
    fn a_folder_is_coloured_and_uncoloured_by_its_path_however_it_is_spelt() {
        let mut settings = Settings::default();
        assert!(settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal)));
        assert!(!settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal)), "nothing changed");
        assert!(
            settings.set_folder_color(Path::new("/w/a/"), Some(FolderColor::Pink)),
            "one entry per folder"
        );
        assert_eq!(settings.folder_colors.len(), 1);
        assert_eq!(settings.folder_color(Path::new("/w/a")), Some(FolderColor::Pink));
        assert_eq!(settings.folder_color(Path::new("/w/./a/")), Some(FolderColor::Pink));
        assert_eq!(settings.folder_color(Path::new("/w")), None, "a parent is another folder");
        assert!(!settings.set_folder_color(Path::new("/w/other"), None), "nothing to take away");
        assert!(settings.set_folder_color(Path::new("/w/a"), None));
        assert!(settings.folder_colors.is_empty());
    }

    #[test]
    fn folder_colours_are_saved_in_the_file_as_names() {
        let mut settings = Settings::default();
        settings.set_folder_color(Path::new("/w/a"), Some(FolderColor::Teal));
        let text = serde_json::to_string(&settings).expect("serializes");
        assert!(text.contains(r#""folder_colors":{"/w/a":"teal"}"#), "{text}");
        let back: Settings = jsonc::parse(&text).expect("reads back");
        assert_eq!(back, settings);
    }

    #[test]
    fn the_final_design_keys_round_trip_by_their_documented_names() {
        let text = r#"{
            "status_bar": true, "bell_sound": false, "bell_visual": false,
            "sidebar_cmd_hints": false, "sidebar_numbers": "list",
            "theme_mode": "light", "theme_light": "GitHub Light",
            "notify_command_after": 30, "notify_in_front": "banner", "notify_sound": "all",
            "dock_badge": "needs_you_done", "menu_bar_icon": "status_count",
            "hotkey_action": "toggle_app", "jump_waiting_key": "ctrl-cmd-j"
        }"#;
        let settings: Settings = jsonc::parse(text).expect("valid jsonc");
        assert!(settings.status_bar && !settings.bell_sound && !settings.bell_visual);
        assert_eq!(settings.sidebar_numbers, SidebarNumbers::List);
        assert_eq!(settings.theme_mode, ThemeMode::Light);
        assert_eq!(settings.notify_in_front, InFront::Banner);
        assert_eq!(settings.notify_sound, NotifySound::All);
        assert_eq!(settings.dock_badge, DockBadge::NeedsYouDone);
        assert_eq!(settings.hotkey_action, HotkeyAction::ToggleApp);
        let json = serde_json::to_value(&settings).expect("serializes");
        for (key, value) in [
            ("sidebar_numbers", "list"),
            ("theme_mode", "light"),
            ("notify_in_front", "banner"),
            ("notify_sound", "all"),
            ("dock_badge", "needs_you_done"),
            ("menu_bar_icon", "status_count"),
            ("hotkey_action", "toggle_app"),
        ] {
            assert_eq!(json[key], value, "{key}");
        }
        assert_eq!(serde_json::from_value::<Settings>(json).expect("reads back"), settings);
    }

    /// Older builds offered an in-window toast while Wormhole is in front, and a config.json
    /// written then may still say so. It must read (an unknown value would mark the whole file
    /// unreadable and stop saving) and mean nothing in front, which is what replaced it.
    #[test]
    fn an_older_configs_toast_reads_as_nothing_in_front() {
        let text = r#"{ "notify_in_front": "toast", "font_size": 15.0 }"#;
        let settings: Settings = jsonc::parse(text).expect("the old value still reads");
        assert_eq!(settings.notify_in_front, InFront::None);
        assert_eq!(settings.font_size, 15.0, "the rest of the file applies as before");
        let json = serde_json::to_value(&settings).expect("serializes");
        assert_eq!(json["notify_in_front"], "none", "saved by its current name");
    }

    #[test]
    fn the_terminal_font_size_stays_within_the_one_range() {
        let with =
            |size| Settings { font_size: size, code_font_size: size, ..Settings::default() }.sanitized();
        let (min, max) = (*fonts::TERMINAL_FONT_SIZES.start(), *fonts::TERMINAL_FONT_SIZES.end());
        assert_eq!((with(1.0).font_size, with(99.0).font_size), (min, max));
        assert_eq!((with(1.0).code_font_size, with(99.0).code_font_size), (min, max));
        assert_eq!(with(14.0).font_size, 14.0);
        assert_eq!(fonts::FontSlot::Terminal.size_range(), (min, max));
        assert_eq!(fonts::FontSlot::Code.size_range(), (min, max));
    }

    #[test]
    fn a_command_notification_waits_between_5_seconds_and_5_minutes() {
        let with = |after| Settings { notify_command_after: after, ..Settings::default() }.sanitized();
        assert_eq!(with(0).notify_command_after, 5);
        assert_eq!(with(60).notify_command_after, 60);
        assert_eq!(with(3600).notify_command_after, 300);
    }

    #[test]
    fn overrides_apply_and_bad_values_are_ignored() {
        let overrides = serde_json::json!({ "font_size": 18.0, "tab_width": "wide", "nope": 1 });
        let serde_json::Value::Object(map) = overrides else { unreachable!() };
        let merged = Settings::default().with_overrides(&map);
        assert_eq!(merged.font_size, 18.0);
        assert_eq!(merged.tab_width, Settings::default().tab_width);
    }

    #[test]
    fn sanitizes_out_of_range_values() {
        let settings = Settings { font_size: 200.0, opacity: 0.0, tab_width: 0, ..Settings::default() };
        let settings = settings.sanitized();
        assert_eq!(settings.font_size, 36.0);
        assert_eq!(settings.opacity, 0.3);
        assert_eq!(settings.tab_width, 1);
    }

    #[test]
    fn primary_color_is_a_valid_hex_or_nothing() {
        let with = |value: &str| Settings { primary_color: value.into(), ..Settings::default() };
        assert_eq!(Settings::default().primary(), None);
        assert_eq!(with("#ff6600").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
        assert_eq!(with(" ff6600 ").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
        assert_eq!(with("#ff660080").primary().map(Color::to_hex).as_deref(), Some("#ff6600"));
        assert_eq!(with("orange").primary(), None);
        assert!(with("orange").primary_error().is_some_and(|e| e.starts_with("primary_color:")));
        assert!(with("").primary_error().is_none());
        assert!(with("#ff6600").primary_error().is_none());
    }

    #[test]
    fn projects_can_override_the_primary_color() {
        let serde_json::Value::Object(map) = serde_json::json!({ "primary_color": "#00aa88" }) else {
            unreachable!()
        };
        let merged = Settings::default().with_overrides(&map);
        assert_eq!(merged.primary().map(Color::to_hex).as_deref(), Some("#00aa88"));
    }

    #[test]
    fn partial_json_uses_defaults() {
        let settings: Settings = jsonc::parse("{ // comment\n \"font_size\": 16.0, }").expect("valid jsonc");
        assert_eq!(settings.font_size, 16.0);
        assert_eq!(settings.theme, Settings::default().theme);
    }

    #[test]
    fn a_chosen_terminal_font_survives_the_new_default() {
        let chosen: Settings = jsonc::parse(r#"{ "font_family": "Menlo" }"#).expect("valid jsonc");
        assert_eq!(chosen.sanitized().font_family, "Menlo");
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert_eq!(unset.font_family, fonts::bundled::FAMILY);
        let blank = Settings { font_family: " ".into(), ..Settings::default() };
        assert_eq!(blank.sanitized().font_family, fonts::bundled::FAMILY);
    }

    #[test]
    fn ligatures_default_on_and_round_trip() {
        assert!(Settings::default().font_ligatures);
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert!(unset.font_ligatures);
        let off: Settings = jsonc::parse(r#"{ "font_ligatures": false }"#).expect("valid jsonc");
        assert!(!off.font_ligatures);
        let json = serde_json::to_value(Settings { font_ligatures: false, ..Settings::default() })
            .expect("serializes");
        assert_eq!(json["font_ligatures"], false);
    }

    #[test]
    fn the_status_dot_follows_the_setting_and_keys_of_older_files_still_load() {
        // Keys of older files that mean nothing now: the agent's logo is always on its tile.
        let settings: Settings = jsonc::parse(
            r#"{ "sidebar_status_position": "right", "sidebar_branch": false, "sidebar_agent_icon": false,
                "sidebar_changes": false }"#,
        )
        .expect("valid jsonc");
        assert!(!settings.sidebar_changes, "the rest of the file is read");
        let json = serde_json::to_value(&settings).expect("serializes");
        for gone in ["sidebar_status_position", "sidebar_branch", "sidebar_agent_icon"] {
            assert!(json.get(gone).is_none(), "{gone} is not written back");
        }
    }

    #[test]
    fn option_as_meta_reads_its_sides_and_the_old_switch() {
        let read = |json: &str| -> OptionAsMeta {
            let settings: Settings =
                jsonc::parse(&format!("{{ \"option_as_meta\": {json} }}")).expect("reads");
            settings.option_as_meta
        };
        assert_eq!(Settings::default().option_as_meta, OptionAsMeta::Off);
        assert_eq!(read("\"left\""), OptionAsMeta::Left);
        assert_eq!(read("\"right\""), OptionAsMeta::Right);
        assert_eq!(read("\"both\""), OptionAsMeta::Both);
        assert_eq!(read("\"off\""), OptionAsMeta::Off);
        // Every config.json written before the sides existed has the boolean.
        assert_eq!(read("true"), OptionAsMeta::Both);
        assert_eq!(read("false"), OptionAsMeta::Off);
        assert!(jsonc::parse::<Settings>(r#"{ "option_as_meta": "middle" }"#).is_err());
        let json =
            serde_json::to_value(Settings { option_as_meta: OptionAsMeta::Left, ..Settings::default() })
                .expect("serializes");
        assert_eq!(json["option_as_meta"], "left");
    }

    #[test]
    fn option_as_meta_follows_the_side_that_is_held() {
        use OptionAsMeta::{Both, Left, Off, Right};
        // (left held, right held) → Meta?
        for (setting, left, right, both, unknown) in [
            (Off, false, false, false, false),
            (Left, true, false, true, true),
            (Right, false, true, true, true),
            (Both, true, true, true, true),
        ] {
            assert_eq!(setting.is_meta(true, false), left, "{setting:?} left");
            assert_eq!(setting.is_meta(false, true), right, "{setting:?} right");
            assert_eq!(setting.is_meta(true, true), both, "{setting:?} both held");
            assert_eq!(setting.is_meta(false, false), unknown, "{setting:?} side unknown");
        }
    }

    #[test]
    fn overrides_report_the_keys_they_set() {
        let overrides =
            serde_json::json!({ "tab_width": 8, "font_size": "big", "nope": 1, "diff_wrap": true });
        let serde_json::Value::Object(map) = overrides else { unreachable!() };
        let (merged, applied) = Settings::default().with_overrides_reported(&map);
        assert_eq!(applied, ["diff_wrap", "tab_width"], "a mistyped or unknown key is not reported");
        assert_eq!(merged.tab_width, 8);
        assert!(merged.diff_wrap);
    }

    #[test]
    fn shell_profiles_read_with_or_without_arguments() {
        let settings: Settings = jsonc::parse(
            r#"{
                "default_shell": "fish",
                "shells": [
                    { "name": "fish", "command": "/opt/homebrew/bin/fish" },
                    { "name": "Claude", "command": "claude", "args": [] },
                    { "name": "Dev", "command": "npm", "args": ["run", "dev"] },
                ],
            }"#,
        )
        .expect("valid jsonc");
        assert_eq!(settings.default_shell, "fish");
        let args: Vec<Option<&[String]>> = settings.shells.iter().map(|p| p.args.as_deref()).collect();
        assert_eq!(args[0], None, "no args: a login shell");
        assert_eq!(args[1], Some(&[][..]), "[]: the program bare");
        assert_eq!(args[2].map(|a| a.len()), Some(2));
        // Missing args stay missing when written back, so the login-shell meaning is kept.
        let json = serde_json::to_value(&settings).expect("serializes");
        assert_eq!(
            json["shells"][0],
            serde_json::json!({ "name": "fish", "command": "/opt/homebrew/bin/fish" })
        );
        assert_eq!(json["shells"][1]["args"], serde_json::json!([]));
        let unset: Settings = jsonc::parse("{}").expect("valid jsonc");
        assert!(unset.shells.is_empty() && unset.default_shell.is_empty(), "the login shell by default");
        // An unknown default is kept as written: the resolver falls back and says so.
        let odd: Settings = jsonc::parse(r#"{ "default_shell": "nope" }"#).expect("valid jsonc");
        assert_eq!(odd.sanitized().default_shell, "nope");
    }

    #[test]
    fn unusable_and_duplicate_shell_profiles_are_dropped() {
        let profile = |name: &str, command: &str| ShellProfile {
            name: name.into(),
            command: command.into(),
            args: None,
        };
        let settings = Settings {
            shells: vec![
                profile("fish", "/opt/homebrew/bin/fish"),
                profile("", "/bin/bash"),
                profile("nameless", " "),
                profile("fish", "/usr/local/bin/fish"),
                profile("nu", "nu"),
            ],
            ..Settings::default()
        }
        .sanitized();
        let kept: Vec<(&str, &str)> =
            settings.shells.iter().map(|p| (p.name.as_str(), p.command.as_str())).collect();
        assert_eq!(kept, [("fish", "/opt/homebrew/bin/fish"), ("nu", "nu")]);
    }
}
