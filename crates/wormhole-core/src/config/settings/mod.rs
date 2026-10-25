//! The settings schema: every key of `config.json`, its default and its
//! valid range. The struct and its defaults are here; the values some settings take are in `choices`,
//! which keys a project may set in `keys`, the project overlay in `overlay`, the primary colour's switch
//! in `primary` and the colours of folders in `folder_colors`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::clipboard::{ClipboardRead, ClipboardWrite};
use crate::config::features::{self, FeatureFlag, FlagMode};
use crate::theme::fonts::{self, FontSlot};
use crate::theme::registry::{DEFAULT_THEME, DEFAULT_THEME_LIGHT};
use crate::update::UpdateChannel;
use crate::view::sidebar::{Density, SidebarNumbers, TerminalsIn};

mod choices;
mod folder_colors;
mod keys;
mod overlay;
mod primary;
#[cfg(test)]
mod tests;

use self::folder_colors::folder_color_key;

pub use self::choices::{
    DockBadge, HotkeyAction, InFront, MenuBarIcon, NotifySound, OptionAsMeta, ShellProfile, SizeUnits,
    ThemeMode, TitleBorder,
};
pub(crate) use self::keys::keys;
pub use self::keys::{APP_SETTINGS, PROJECT_SETTINGS};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// A built-in theme name, or a path to a Ghostty / iTerm2 / Alacritty theme file.
    pub theme: String,
    /// Replaces the theme's primary (the primary color: buttons, focus, selection,
    /// links) with this `#rrggbb` while `theme_primary` is off; empty keeps the theme's own.
    pub primary_color: String,
    /// Use the theme's own primary. Off, `primary_color` replaces it, in every theme; the color
    /// stays chosen while this is on. A config from before the switch has no value: one that
    /// chose a color keeps it (see [`Settings::uses_theme_primary`]).
    #[serde(default)]
    pub theme_primary: Option<bool>,
    pub font_family: String,
    pub font_size: f32,
    /// Combine sequences like `=>` and `!=` into single glyphs (in fonts that
    /// have them, like JetBrains Mono) in the terminal and code text.
    pub font_ligatures: bool,
    pub line_height: f32,
    /// Background opacity, 0.3..=1.0.
    pub opacity: f32,
    pub blur: bool,
    /// Liquid Glass under the sidebar's and the panel's cards (macOS 26 and later). Off, every
    /// surface is painted; see [`Settings::glass_on`].
    pub liquid_glass: bool,
    /// How much room each terminal takes in the sidebar and the tab bar.
    pub density: Density,
    /// Groups are cards in the sidebar; off, a group is its head and rows, with no card round them.
    pub show_groups_as_cards: bool,
    /// A group's colour washes its card from the left and fades out across it; off, the colour fills
    /// the card evenly.
    pub fade_group_colors: bool,
    /// A group with a folder shows it under its name on its head.
    pub show_group_folders: bool,
    /// What `density` replaced: one-line rows. Read from older files (see [`Settings::sanitized`]),
    /// never written.
    #[serde(skip_serializing)]
    #[doc(hidden)]
    pub compact_sidebar: bool,
    /// Which Option keys send Esc+key (a side set to Meta can't type @ { [ on some layouts).
    pub option_as_meta: OptionAsMeta,
    pub scrollback_lines: usize,
    /// Save each terminal's last lines (encrypted, in the profile's `scrollback.db`) and show them
    /// again after a restart. Off: nothing is saved, and what was is erased.
    pub restore_terminal_text: bool,
    /// macOS notifications when a background session finishes or needs input.
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
    /// Which builds to update to: releases, or also every build of main (see [`UpdateChannel`]).
    /// Unstable also shows the feature-flag switches (see [`crate::config::features`]).
    pub update_channel: UpdateChannel,

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
    /// Themes the picker leaves out, each as `theme` would name it; Settings brings them back one
    /// by one.
    pub hidden_themes: Vec<String>,

    // Window and terminal
    /// The status bar along the bottom of every window (View → Show Status Bar).
    pub status_bar: bool,
    /// The minimap along every terminal's right edge (View → Show Minimap).
    pub minimap: bool,
    /// A small gap between the terminals of a split, each its own rounded surface; off joins them with
    /// a hairline.
    pub split_gaps: bool,
    /// The lines between the sidebar, the terminal and the right panel, as the theme draws them
    /// (`divider`, `metrics.divider_width`). Off hides them whatever the theme says; the handles
    /// still resize. Increase Contrast keeps them.
    pub column_dividers: bool,
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
    /// When an agent needs input.
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
    /// One bounce of the Dock icon when a terminal needs input.
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
    /// Let programs act on the window (move, resize, minimise, raise, full screen: XTWINOPS). Off:
    /// a program can only ask for sizes and the window's place. Terminals started after the change
    /// use it.
    pub allow_window_ops: bool,
    /// Answer a program's request for the window title and icon name (XTWINOPS 20 and 21). Off:
    /// the title is text a program chose, and the answer is typed into the shell's input, so it
    /// lets any program, also one over ssh, type a command line. Terminals started after the
    /// change use it.
    pub allow_title_report: bool,
    /// What a program that asks to read the clipboard (OSC 52) gets: a question over its terminal
    /// (the default), the text, or nothing. Applies at once, to every terminal.
    pub clipboard_read: ClipboardRead,
    /// Whether a program may set the clipboard (OSC 52), which the terminal then says in a notice.
    pub clipboard_write: ClipboardWrite,
    /// Where New Terminal in New Worktree puts a worktree: under this folder (in a folder named
    /// after the repository), or, empty, beside the repository in `<name>-worktrees`.
    pub worktree_base: String,

    /// Features still behind a flag (see [`crate::config::features`]). The key is a [`FeatureFlag`]'s
    /// `key`, the value whether it is on. Only a development build or the Unstable channel reads
    /// this; otherwise a release uses each flag's release default. A key this build doesn't know
    /// stays in the file.
    pub feature_flags: BTreeMap<String, bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.into(),
            primary_color: String::new(),
            theme_primary: Some(true),
            // Only fresh configs: config.json is written out in full on first
            // launch, so existing ones keep the font they have.
            font_family: fonts::bundled::FAMILY.into(),
            font_size: 14.0,
            font_ligatures: true,
            line_height: 1.3,
            opacity: 0.8,
            blur: true,
            liquid_glass: false,
            density: Density::default(),
            show_groups_as_cards: true,
            fade_group_colors: false,
            show_group_folders: true,
            compact_sidebar: false,
            option_as_meta: OptionAsMeta::Off,
            scrollback_lines: 10_000,
            restore_terminal_text: true,
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
            update_channel: UpdateChannel::Stable,
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
            theme_light: DEFAULT_THEME_LIGHT.into(),
            hidden_themes: Vec::new(),
            status_bar: false,
            minimap: false,
            split_gaps: true,
            column_dividers: true,
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
            allow_title_report: false,
            clipboard_read: ClipboardRead::default(),
            clipboard_write: ClipboardWrite::default(),
            worktree_base: String::new(),
            feature_flags: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Whether the feature behind `flag` is on in this build: a development build reads
    /// `feature_flags` (a flag with no entry defaults on), a release on the Unstable channel reads
    /// it too (no entry: the release default), any other release uses the flag's release default
    /// (see [`crate::config::features`]).
    pub fn feature_enabled(&self, flag: FeatureFlag) -> bool {
        features::enabled_in(flag, &self.feature_flags, self.flag_mode())
    }

    /// Who decides the feature flags in this build with these settings.
    pub fn flag_mode(&self) -> FlagMode {
        FlagMode::of(features::dev_build(), self.update_channel)
    }

    /// Whether the sidebar's and the panel's cards sit on Liquid Glass: asked for, the Mac has it
    /// (macOS 26 and later), and the system doesn't want windows opaque (Reduce Transparency,
    /// Increase Contrast). Otherwise every surface paints itself.
    pub fn glass_on(&self, mac_has_glass: bool, system_opaque: bool) -> bool {
        self.liquid_glass && mac_has_glass && !system_opaque
    }
}

impl Settings {
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

    pub(super) fn sanitized(mut self) -> Self {
        // Written out from now on, so the switch says what is in use.
        self.theme_primary = Some(self.uses_theme_primary());
        for slot in FontSlot::ALL {
            self.step_slot_size(slot, 0.0);
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
