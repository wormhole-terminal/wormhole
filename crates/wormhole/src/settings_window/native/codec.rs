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

use super::controls::{self, Control, Current, Label, MenuOption};
use super::{accent, themes};

/// One option of a menu: the stable id the window stores and sends, and the words it shows.
pub(super) type Choice = (String, String);

/// The id of a registry option: its message's id for a message, `n<number>` for a counted one. Ids are
/// snake_case and never change with the language or the wording.
fn option_id(option: &MenuOption) -> String {
    match &option.label {
        Label::Message(key) => key.id().replace(['-', '.'], "_"),
        Label::Counted(_, n) => format!("n{n}"),
    }
}

/// The id of a value written by hand into `config.json` that is none of a menu's options.
pub(super) const OTHER: &str = "other";

fn density_id(density: Density) -> String {
    density.label().to_lowercase()
}

/// The two pairing choices' ids.
pub(super) const PAIRING_BOTH: &str = "pairing_both";
pub(super) const PAIRING_SEPARATE: &str = "pairing_separate";

/// The id of the font the interface uses when none is named, and of "same as the terminal's".
pub(super) const SAME_AS_TERMINAL: &str = "same_as_terminal";

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
    if accent::is_colour_row(id) {
        return accent::read_colour(id, s);
    }
    match id {
        "default-shell" => vec![("", super::system::shell(s).0)],
        "title-row" => accent::read_title(s),
        "icon-theme" => accent::read_icon_theme(s),
        "density" => vec![("", density_id(s.density))],
        "theme-pairing" => {
            vec![("", if ctx.separate(s) { PAIRING_SEPARATE } else { PAIRING_BOTH }.to_string())]
        }
        "theme" => vec![
            ("", themes::pair_name(ctx.themes, s)),
            ("#dark", themes::pick(ctx.themes, s, Appearance::Dark)),
            ("#light", themes::pick(ctx.themes, s, Appearance::Light)),
        ],
        "translucent" => {
            vec![("Settings", on(s.settings_translucent)), ("Theme Studio", on(s.studio_translucent))] // text: not shown (values of the native window's own catalog)
        }
        _ => match controls::control(id).map(|setting| &setting.control) {
            Some(Control::Toggle { get, .. }) => vec![("", on(get(s)))],
            Some(Control::Menu(menu)) => vec![(
                "",
                match menu.current(s) {
                    Current::At(at) => option_id(&menu.options[at]),
                    Current::Other(_) => OTHER.to_string(),
                },
            )],
            Some(Control::Recorder(row)) => vec![("", super::recorder::glyphs((row.get)(s)))],
            _ => Vec::new(),
        },
    }
}

/// The options a row offers, when it is a menu.
pub(super) fn options(id: &str, s: &Settings) -> Option<Vec<Choice>> {
    if id == "icon-theme" {
        return Some(accent::icon_theme_options(s).into_iter().map(|name| (name.clone(), name)).collect());
    }
    if id == "default-shell" {
        let mut shells = super::system::shell(s).1;
        shells.push(("-".into(), "-".into()));
        shells.push(("edit_shells".into(), super::words::word("Edit shells…"))); // text: a native word, translated by its id
        return Some(shells);
    }
    if id == "density" {
        return Some(Density::ALL.iter().map(|d| (density_id(*d), d.label().to_string())).collect());
    }
    if id == "theme-pairing" {
        return Some(vec![
            (PAIRING_BOTH.into(), text(SETTINGS_APPEARANCE_PAIRING_BOTH).into()),
            (PAIRING_SEPARATE.into(), text(SETTINGS_APPEARANCE_PAIRING_SEPARATE).into()),
        ]);
    }
    let Some(Control::Menu(menu)) = controls::control(id).map(|setting| &setting.control) else {
        return None;
    };
    let mut choices: Vec<Choice> =
        menu.options.iter().map(|option| (option_id(option), option.label.words().to_string())).collect();
    if let Current::Other(other) = menu.current(s) {
        choices.push((OTHER.into(), other));
    }
    Some(choices)
}

/// Write `value` into the row `id` (`suffix` names a sub-value). Whether it was understood.
/// The two checkboxes of the translucent row, by the names the window gives them.
const SETTINGS_WINDOW: &str = "Settings"; // text: not shown (a part of the native window's own row)
const THEME_STUDIO: &str = "Theme Studio"; // text: not shown (a part of the native window's own row)

pub(super) fn write(id: &str, suffix: &str, s: &mut Settings, value: &str, ctx: &Ctx) -> bool {
    if let Some(flag) = flag(id) {
        s.feature_flags.insert(flag.key().to_string(), value == "on");
        return true;
    }
    if let Some(slot) = font_slot(id) {
        return write_font(slot, suffix, s, value);
    }
    if accent::is_colour_row(id) {
        return accent::write_colour(id, suffix, s, value);
    }
    match id {
        super::open_in::ROW => super::open_in::write(s, value),
        "default-shell" => super::system::set_shell(s, value),
        "title-row" => accent::write_title(suffix, s, value),
        "icon-theme" => accent::write_icon_theme(s, value),
        "density" => match Density::ALL.iter().find(|d| density_id(**d) == value) {
            Some(density) => {
                s.density = *density;
                true
            }
            None => false,
        },
        "theme-pairing" => {
            if value == PAIRING_BOTH {
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
            SETTINGS_WINDOW => {
                // text: not shown (values of the native window's own catalog)
                s.settings_translucent = value == "on";
                true
            }
            THEME_STUDIO => {
                // text: not shown (values of the native window's own catalog)
                s.studio_translucent = value == "on";
                true
            }
            _ => false,
        },
        _ => match controls::control(id).map(|setting| &setting.control) {
            Some(Control::Toggle { set, .. }) => {
                set(s, value == "on");
                true
            }
            Some(Control::Menu(menu)) => match menu.options.iter().position(|o| option_id(o) == value) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(row: &str) -> Vec<String> {
        options(row, &Settings::default()).unwrap_or_default().into_iter().map(|(id, _)| id).collect()
    }

    /// The ids the window's own code compares (previews, the colour scheme): `macos/settings/.../Pages.swift` names
    /// them in its catalog as `"id|English"`.
    #[test]
    fn the_ids_the_window_depends_on_exist() {
        for (row, wanted) in [
            (
                "theme-mode",
                &[
                    "settings_appearance_match_system",
                    "settings_appearance_dark",
                    "settings_appearance_light",
                ][..],
            ),
            ("changes-view", &["settings_git_list", "settings_git_tree"]),
            ("diff-layout", &["settings_git_split", "settings_git_unified"]),
            (
                "menu-bar-icon",
                &[
                    "settings_menu_bar_status_and_count",
                    "settings_menu_bar_status",
                    "settings_menu_bar_icon_only",
                ],
            ),
            ("notify-command-done", &["settings_notifications_off"]),
            ("density", &["regular", "compact", "dense"]),
            ("theme-pairing", &[PAIRING_BOTH, PAIRING_SEPARATE]),
            ("opacity", &["n100", "n95", "n90", "n80", "n70"]),
            ("tab-width", &["n4", "n2", "n8"]),
        ] {
            let have = ids(row);
            for id in wanted {
                assert!(have.iter().any(|h| h == id), "{row}: no option {id} among {have:?}");
            }
        }
    }

    #[test]
    fn an_option_is_read_and_written_by_its_id_in_any_language() {
        let mut s = Settings::default();
        let ctx = Ctx { themes: &[], separate: None };
        assert!(write("density", "", &mut s, "dense", &ctx));
        assert_eq!(read("density", &s, &ctx)[0].1, "dense");
        assert!(!write("density", "", &mut s, "Dense", &ctx), "the words are not the id");
        assert!(write("theme-mode", "", &mut s, "settings_appearance_dark", &ctx));
        assert_eq!(read("theme-mode", &s, &ctx)[0].1, "settings_appearance_dark");
    }

    #[test]
    fn option_ids_are_snake_case_and_distinct_within_a_row() {
        for (_, id) in crate::settings_window::native::rows::all() {
            let have = ids(id);
            let mut sorted = have.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), have.len(), "{id}: an id twice in {have:?}");
        }
    }
}
