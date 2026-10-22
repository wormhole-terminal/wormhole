//! Feature flags: a switch for each in-progress feature (see core's `features`). This section,
//! and the whole Preview nav group it sits in, is drawn only in a development build or on the
//! Unstable update channel; any other release neither lists it nor reads the switches. Turning a
//! flag on makes its feature's own section appear (Project folders) and the feature work in every
//! window at once.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;

use crate::settings::settings;
use crate::settings::update_settings;
use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, SettingParts};

pub(super) const SETTINGS: &[Setting] = &[Setting::block("feature-flags", |w, cx| w.draw_feature_flags(cx))];

impl SettingsWindow {
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
                    chip: None,
                    key: "",
                    state,
                    found: None,
                    found_description: None,
                    control,
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
