//! What a source's settings say, as data: the Wormhole settings they would set and the ones that
//! cannot be taken, each with the reason. No sentence is written here; the Settings window words
//! every variant.

use crate::config::{OptionAsMeta, ShellProfile};
use crate::theme::{Builder, Theme};

/// A Wormhole setting an import can set. Named as `config.json` names them (see [`Setting::key`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Setting {
    FontFamily,
    FontSize,
    LineHeight,
    Ligatures,
    /// The terminal palette (`theme`; the dark pick).
    Theme,
    /// The palette for light mode (`theme_light`), when the source has a light and a dark one.
    ThemeLight,
    Opacity,
    Blur,
    OptionAsMeta,
    ScrollbackLines,
    BellSound,
    BellVisual,
    /// What a new terminal runs: a shell the system lists (`default_shell`).
    DefaultShell,
    /// A command with its arguments, added to the shells Wormhole offers (`shells`).
    ShellProfile,
}

impl Setting {
    /// The `config.json` key it sets.
    pub fn key(self) -> &'static str {
        match self {
            Setting::FontFamily => "font_family",
            Setting::FontSize => "font_size",
            Setting::LineHeight => "line_height",
            Setting::Ligatures => "font_ligatures",
            Setting::Theme => "theme",
            Setting::ThemeLight => "theme_light",
            Setting::Opacity => "opacity",
            Setting::Blur => "blur",
            Setting::OptionAsMeta => "option_as_meta",
            Setting::ScrollbackLines => "scrollback_lines",
            Setting::BellSound => "bell_sound",
            Setting::BellVisual => "bell_visual",
            Setting::DefaultShell => "default_shell",
            Setting::ShellProfile => "shells",
        }
    }
}

/// A font as a source names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontName {
    /// A family name (`JetBrains Mono`), or a PostScript name (`JetBrainsMono-Regular`) when
    /// `postscript` is set.
    pub name: String,
    /// iTerm2 and Terminal.app store the PostScript name of the face. Only the system can say which
    /// family it belongs to (the app asks it: [`Proposal::resolve_fonts`]); a name is never cut at its
    /// `-` to guess, and one still unresolved is not applied.
    pub postscript: bool,
}

impl FontName {
    /// A family name.
    pub(crate) fn family_name(name: impl Into<String>) -> Self {
        Self { name: name.into(), postscript: false }
    }

    /// A PostScript name.
    pub(crate) fn postscript_name(name: impl Into<String>) -> Self {
        Self { name: name.into(), postscript: true }
    }
}

/// The value a setting would take. Values outside Wormhole's ranges are already clamped to them;
/// `Proposed::from` keeps what the source wrote.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Font(FontName),
    /// Points, or a multiplier, or an opacity from 0 to 1.
    Number(f32),
    /// Lines of scrollback.
    Count(usize),
    Flag(bool),
    Meta(OptionAsMeta),
    /// A palette, named for the theme file it will be saved as (`<product> <theme>`).
    Palette(Theme),
    /// A shell's path, for `default_shell`.
    Shell(String),
    /// A command, for `shells`.
    Profile(ShellProfile),
}

/// One setting a source says, and what Wormhole would set for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposed {
    pub setting: Setting,
    /// What the source wrote, as it wrote it (`font-size = 13`), for the review list.
    pub from: String,
    pub value: SettingValue,
    /// Ticked in the review list from the start. Off for what is risky or unverified: a command
    /// Wormhole would run in every new terminal.
    pub default_on: bool,
}

/// Why a setting is not taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkippedKind {
    CursorShape,
    CursorBlink,
    Padding,
    CopyOnSelect,
    ConfirmClose,
    WorkingDirectory,
    Keybinding,
    Environment,
    /// Bold, italic and weight variants of the font, a second font for non-ASCII text.
    FontVariant,
    /// Letter spacing, character width.
    CellWidth,
    /// Anything else the source has and Wormhole does not.
    Other,
}

/// Why a key of the source did not become a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Wormhole has no such setting.
    NoSuchSetting(SkippedKind),
    /// A value Wormhole cannot show or run (an unknown name, another unit, a colour by name).
    UnsupportedValue,
    /// Lua that is computed, not written out; it is never run.
    DynamicLua,
    /// A theme the source ships inside the app, whose colours are not in any file.
    BuiltInTheme(String),
    /// It runs a program (a generated include, a bell command, a trigger). Never run, never taken.
    Executes,
    /// The file could not be read: gone, too big, not text, not valid.
    Unreadable,
    /// A file that includes itself, directly or through others.
    IncludeCycle,
    /// Includes nested too deep, or more files than are read.
    IncludeLimit,
    /// A settings file in a format that is no longer read (Alacritty's YAML).
    OldFormat,
    /// A file named by the settings (an include, a theme, an import) that is outside the home
    /// folder, or a link that leads out of it. Only the person's own folder is read.
    OutsideHome,
    /// Some colours, but not both the background and the foreground: a few colours on defaults, not
    /// a palette to save as a theme.
    PartialPalette,
    /// A font the source names by its PostScript name, which no font installed on this Mac has.
    FontNotInstalled(String),
}

/// A key of the source that was read and not taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skipped {
    /// The key (or the file) as the source names it.
    pub key: String,
    pub reason: Reason,
}

/// Everything one source's settings say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proposal {
    /// The settings that can be taken, one per setting, in the order of [`Setting`].
    pub values: Vec<Proposed>,
    /// What cannot.
    pub skipped: Vec<Skipped>,
}

impl Proposal {
    /// Add a setting, ticked by default.
    pub(crate) fn propose(&mut self, setting: Setting, from: impl Into<String>, value: SettingValue) {
        self.add(setting, from.into(), value, true);
    }

    /// Add a setting that is not ticked by default.
    pub(crate) fn propose_unticked(
        &mut self,
        setting: Setting,
        from: impl Into<String>,
        value: SettingValue,
    ) {
        self.add(setting, from.into(), value, false);
    }

    fn add(&mut self, setting: Setting, from: String, value: SettingValue, default_on: bool) {
        // One value per setting, the later reading wins, except the shell profiles, which add up.
        if setting != Setting::ShellProfile {
            self.values.retain(|proposed| proposed.setting != setting);
        }
        self.values.push(Proposed { setting, from, value, default_on });
    }

    /// Offer the palette `builder` holds, called `label`, when it is a whole one (both the background
    /// and the foreground: [`Builder::has_base`]); colours short of that are noted as skipped, under
    /// `from`. A palette with no colour at all is nothing to say.
    pub(crate) fn propose_palette(
        &mut self,
        setting: Setting,
        from: impl Into<String>,
        label: &str,
        builder: Builder,
    ) {
        let from = from.into();
        if builder.has_base() {
            self.propose(setting, from, SettingValue::Palette(builder.finish(label)));
        } else if builder.has_any() {
            self.skip(from, Reason::PartialPalette);
        }
    }

    /// Note a key that is not taken.
    pub(crate) fn skip(&mut self, key: impl Into<String>, reason: Reason) {
        self.skipped.push(Skipped { key: key.into(), reason });
    }

    /// Note a key Wormhole has no setting for.
    pub(crate) fn skip_kind(&mut self, key: impl Into<String>, kind: SkippedKind) {
        self.skip(key, Reason::NoSuchSetting(kind));
    }

    /// Put the values in the order of [`Setting`] (shell profiles keep the order they came in).
    pub(crate) fn sorted(mut self) -> Self {
        self.values.sort_by_key(|proposed| proposed.setting);
        self
    }

    /// Turn each PostScript font name into its family with `family_of`, the system's answer (core has
    /// none: the app asks CoreText). A name nothing answers to is not offered: the font is not
    /// installed, and any family made up from its name would not be either. It moves to the skipped
    /// list with [`Reason::FontNotInstalled`].
    pub fn resolve_fonts(&mut self, family_of: impl Fn(&str) -> Option<String>) {
        let values = std::mem::take(&mut self.values);
        for mut proposed in values {
            if let SettingValue::Font(font) = &proposed.value
                && font.postscript
            {
                match family_of(&font.name) {
                    Some(family) => proposed.value = SettingValue::Font(FontName::family_name(family)),
                    None => {
                        self.skipped.push(Skipped {
                            key: proposed.from,
                            reason: Reason::FontNotInstalled(font.name.clone()),
                        });
                        continue;
                    }
                }
            }
            self.values.push(proposed);
        }
    }

    /// The proposal for `setting`, if there is one.
    pub fn get(&self, setting: Setting) -> Option<&Proposed> {
        self.values.iter().find(|proposed| proposed.setting == setting)
    }

    /// What is ticked by default.
    pub fn selected(&self) -> impl Iterator<Item = &Proposed> {
        self.values.iter().filter(|proposed| proposed.default_on)
    }

    /// Whether there is nothing to take and nothing skipped.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.skipped.is_empty()
    }
}
