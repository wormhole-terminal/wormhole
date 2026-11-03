//! How a setting becomes the text the native window shows and back. A switch is `on` or `off`; a menu is the words
//! of its current option (the options themselves go with it, in the app's language); a font is its family.
//! Anything else a row keeps is a sub-value with a suffix (`#size`, the name of a checkbox).
//!
//! Most rows are read from the GPUI window's own controls (`registry`), so there is one definition of what a
//! setting is and how it is changed; the few the registry draws by hand are here.

use wormhole_core::config::Settings;
use wormhole_core::config::features::{self, FeatureFlag, FlagMode};
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::fonts::{FontSlot, SYSTEM_FONT};
use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::sidebar::Density;
use wormhole_text::keys::{SETTINGS_APPEARANCE_PAIRING_BOTH, SETTINGS_APPEARANCE_PAIRING_SEPARATE};
use wormhole_text::text;

use super::themes;
use crate::settings_window::registry::{self, Control, Current};

/// What the codecs may need besides the settings: the themes.
pub(super) struct Ctx<'a> {
    pub themes: &'a [ThemeEntry],
    /// Whether the person asked for the two picks separately (else it follows whether they are a pair).
    pub separate: Option<bool>,
}

impl Ctx<'_> {
    fn separate(&self, s: &Settings) -> bool {
        self.separate.unwrap_or_else(|| !themes::picks_are_a_pair(self.themes, s))
    }
}

const SAME_AS_TERMINAL: &str = "Same as terminal"; // text: not shown (values of the native window's own catalog)
const SYSTEM_NAME: &str = "SF Pro"; // text: not shown (values of the native window's own catalog)

fn font_slot(id: &str) -> Option<FontSlot> {
    match id {
        "font-terminal" => Some(FontSlot::Terminal),
        "font-code" => Some(FontSlot::Code),
        "font-interface" => Some(FontSlot::Interface),
        _ => None,
    }
}

fn family_words(slot: FontSlot, s: &Settings) -> String {
    let family = s.slot_family(slot).trim();
    if family.is_empty() {
        match slot {
            FontSlot::Code => SAME_AS_TERMINAL.into(),
            FontSlot::Interface => SYSTEM_NAME.into(),
            FontSlot::Terminal => String::new(),
        }
    } else if family == SYSTEM_FONT {
        SYSTEM_NAME.into()
    } else {
        family.into()
    }
}

/// `flag-<n>`: the n-th feature flag, in `FeatureFlag::ALL`'s order (the Feature flags page's).
fn flag(id: &str) -> Option<FeatureFlag> {
    FeatureFlag::ALL.get(id.strip_prefix("flag-")?.parse::<usize>().ok()?).copied()
}

fn flag_mode(s: &Settings) -> FlagMode {
    FlagMode::of(features::dev_build(), s.update_channel, features::unlocked(&s.feature_flags))
}

fn on(value: bool) -> String {
    if value { "on" } else { "off" }.into()
}

/// The values of the row `id`: (suffix, value) pairs, the row's own first.
pub(super) fn read(id: &str, s: &Settings, ctx: &Ctx) -> Vec<(&'static str, String)> {
    if let Some(flag) = flag(id) {
        return vec![("", on(features::enabled_in(flag, &s.feature_flags, flag_mode(s))))];
    }
    if let Some(slot) = font_slot(id) {
        return vec![("", family_words(slot, s)), ("#size", format!("{}", s.slot_size(slot).round()))];
    }
    match id {
        "theme-pairing" => {
            let words = if ctx.separate(s) {
                SETTINGS_APPEARANCE_PAIRING_SEPARATE
            } else {
                SETTINGS_APPEARANCE_PAIRING_BOTH
            };
            vec![("", text(words).to_string())]
        }
        "theme" => vec![
            ("", themes::pair_name(ctx.themes, s)),
            ("#dark", themes::pick(ctx.themes, s, Appearance::Dark)),
            ("#light", themes::pick(ctx.themes, s, Appearance::Light)),
        ],
        "translucent" => {
            vec![("Settings", on(s.settings_translucent)), ("Theme Studio", on(s.studio_translucent))] // text: not shown (values of the native window's own catalog)
        }
        _ => match registry::control(id).map(|setting| &setting.control) {
            Some(Control::Toggle { get, .. }) => vec![("", on(get(s)))],
            Some(Control::Menu(menu)) => vec![(
                "",
                match menu.current(s) {
                    Current::At(at) => menu.options[at].label.words().to_string(),
                    Current::Other(other) => other,
                },
            )],
            _ => Vec::new(),
        },
    }
}

/// The options a row offers, when it is a menu.
pub(super) fn options(id: &str, s: &Settings) -> Option<Vec<String>> {
    if id == "density" {
        return Some(Density::ALL.iter().map(|d| d.label().to_string()).collect());
    }
    if id == "theme-pairing" {
        return Some(vec![
            text(SETTINGS_APPEARANCE_PAIRING_BOTH).into(),
            text(SETTINGS_APPEARANCE_PAIRING_SEPARATE).into(),
        ]);
    }
    let Some(Control::Menu(menu)) = registry::control(id).map(|setting| &setting.control) else {
        return None;
    };
    let mut words: Vec<String> = menu.options.iter().map(|option| option.label.words().to_string()).collect();
    if let Current::Other(other) = menu.current(s) {
        words.push(other);
    }
    Some(words)
}

/// Write `value` into the row `id` (`suffix` names a sub-value). Whether it was understood.
pub(super) fn write(id: &str, suffix: &str, s: &mut Settings, value: &str, ctx: &Ctx) -> bool {
    if let Some(flag) = flag(id) {
        s.feature_flags.insert(flag.key().to_string(), value == "on");
        return true;
    }
    if let Some(slot) = font_slot(id) {
        return write_font(slot, suffix, s, value);
    }
    match id {
        "density" => match Density::ALL.iter().find(|d| d.label() == value) {
            Some(density) => {
                s.density = *density;
                true
            }
            None => false,
        },
        "theme-pairing" => {
            if value == text(SETTINGS_APPEARANCE_PAIRING_BOTH) {
                themes::pair_with_dark(ctx.themes, s);
            }
            true
        }
        "theme" => {
            match suffix {
                "" => themes::choose_pair(ctx.themes, s, value),
                "#dark" => s.theme = value.to_string(),
                "#light" => s.theme_light = value.to_string(),
                _ => return false,
            }
            true
        }
        "translucent" => match suffix {
            "Settings" => {
                // text: not shown (values of the native window's own catalog)
                s.settings_translucent = value == "on";
                true
            }
            "Theme Studio" => {
                // text: not shown (values of the native window's own catalog)
                s.studio_translucent = value == "on";
                true
            }
            _ => false,
        },
        _ => match registry::control(id).map(|setting| &setting.control) {
            Some(Control::Toggle { set, .. }) => {
                set(s, value == "on");
                true
            }
            Some(Control::Menu(menu)) => match menu.options.iter().position(|o| o.label.words() == value) {
                Some(at) => {
                    (menu.set)(s, at);
                    true
                }
                // The value a hand-edited file holds: it is already what is set.
                None => false,
            },
            _ => false,
        },
    }
}

fn write_font(slot: FontSlot, suffix: &str, s: &mut Settings, value: &str) -> bool {
    match suffix {
        "#size" => {
            let Ok(size) = value.parse::<f32>() else { return false };
            let (min, max) = slot.size_range();
            let delta = size.clamp(min, max) - s.slot_size(slot);
            s.step_slot_size(slot, delta);
            true
        }
        "" => {
            let family = match value {
                SAME_AS_TERMINAL => String::new(),
                SYSTEM_NAME if slot == FontSlot::Interface => SYSTEM_FONT.into(),
                other => other.into(),
            };
            s.set_slot_family(slot, family);
            true
        }
        _ => false,
    }
}
