//! Feature flags: a switch for each in-progress feature (see core's `features`). This section,
//! and the whole Preview nav group it sits in, is drawn only in a development build, on the
//! Unstable update channel, or in a Stable release whose flags were unlocked with an unlisted key
//! (⌃⌥⌘F in this window); any other release neither lists it nor reads the switches. Turning a
//! flag on makes its feature's own section appear (Project folders) and the feature work in every
//! window at once.

use gpui_kit::*;
use wormhole_core::config::features::{self, FeatureFlag};

use crate::settings::settings;
use crate::settings::update_settings;
use crate::settings_window::registry::Setting;
use crate::settings_window::{Section, SettingsWindow, ui};
use crate::style::{self, Edges, SettingParts};

pub(super) const SETTINGS: &[Setting] = &[Setting::block("feature-flags", |w, cx| w.draw_feature_flags(cx))];

/// The unlisted key that unlocks the flags in a Stable release, and locks them again: ⌃⌥⌘F. It is in
/// no menu, keymap or shortcut sheet on purpose; the flags are for testers who were told about it.
pub(in crate::settings_window) fn is_unlock_key(keys: &Keystroke) -> bool {
    let m = &keys.modifiers;
    m.control && m.alt && m.platform && !m.shift && keys.key == "f"
}

impl SettingsWindow {
    /// Unlocks the feature flags (the page shows, and the flags written in `config.json` apply), or
    /// locks them again, which puts every flag back to what the release ships while keeping the
    /// switches' entries for the next unlock.
    pub(in crate::settings_window) fn toggle_flags_unlocked(&mut self, cx: &mut Context<Self>) {
        let unlock = !features::unlocked(&settings(cx).feature_flags);
        update_settings(cx, |s| {
            if unlock {
                s.feature_flags.insert(features::UNLOCKED_KEY.to_string(), true);
            } else {
                s.feature_flags.remove(features::UNLOCKED_KEY);
            }
        });
        tracing::info!("feature flags {}", if unlock { "unlocked" } else { "locked" });
        if unlock {
            self.show_section(Section::FeatureFlags, cx);
        }
    }

    fn draw_feature_flags(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let mut rows = div().flex().flex_col().rule_bottom(ui);
        for flag in FeatureFlag::ALL {
            let on = settings(cx).feature_enabled(flag);
            let key = flag.key();
            // A flag is on by default in a dev build and as released on Unstable, so "changed" is
            // whatever differs from that. Going back to the default takes the key out again rather
            // than writing a redundant entry, so the dot clears.
            let default = settings(cx).flag_mode().default_for(flag);
            let state =
                if on == default { style::SettingState::Default } else { style::SettingState::Changed };
            let control = style::toggle(key, on, move |on, _, cx| {
                update_settings(cx, |s| {
                    if on == default {
                        s.feature_flags.remove(key);
                    } else {
                        s.feature_flags.insert(key.to_string(), on);
                    }
                });
            })
            .into_any_element();
            rows = rows.child(style::setting(
                SharedString::from(format!("feature-flag-{key}")),
                ui,
                SettingParts {
                    label: flag.title().into(),
                    description: Some(flag.description().into()),
                    visible_hint: false,
                    chip: None,
                    key: "",
                    state,
                    found: None,
                    found_description: None,
                    control,
                    tall: false,
                    extra: None,
                    on_reset: Some(Box::new(move |_, _, cx| {
                        update_settings(cx, |s| {
                            s.feature_flags.remove(key);
                        });
                    })),
                },
                cx,
            ));
        }
        rows.into_any_element()
    }
}
