//! Feature flags: a switch for each in-progress feature (see core's `features`). This section,
//! and the whole Preview nav group it sits in, is drawn only in a development build; a release
//! build neither lists it nor reads the switches. Turning a flag on makes its feature's own
//! section appear (Project folders) and the feature work in every window at once.

use wormhole_core::features::FeatureFlag;

use super::*;

pub(super) const SETTINGS: &[Setting] = &[Setting::block("feature-flags", |w, cx| w.draw_feature_flags(cx))];

impl SettingsWindow {
    fn draw_feature_flags(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let mut rows = div().flex().flex_col().rule_bottom(ui);
        for flag in FeatureFlag::ALL {
            let on = settings(cx).feature_enabled(flag);
            let key = flag.key();
            // A flag is on by default in a dev build, so "changed" means off. Turning it back on
            // takes the key out again rather than writing a redundant `true`, so the dot clears.
            let state = if on { style::SettingState::Default } else { style::SettingState::Changed };
            let control = style::toggle(key, on, move |on, _, cx| {
                update_settings(cx, |s| {
                    if on {
                        s.feature_flags.remove(key);
                    } else {
                        s.feature_flags.insert(key.to_string(), false);
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
