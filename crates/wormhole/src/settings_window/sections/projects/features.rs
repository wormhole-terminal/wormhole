//! What a feature row says and how it is drawn, for both views.

use gpui_kit::*;
use wormhole_core::project::{Feature, FeatureStanding};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::settings::UiColors;
use crate::style::{self, RowKeys, SettingParts, SettingState};

/// The line under a feature's name: core's words for a file that is missing or sets nothing, what
/// the feature does otherwise, with the wait for trust when something is held back.
pub(super) fn feature_description(feature: Feature, standing: FeatureStanding) -> String {
    match (feature, standing) {
        // The switch is how the file gets made.
        (Feature::Notes, FeatureStanding::Missing) => {
            text(SETTINGS_PROJECTS_NO_NOTES_MD_YET_ON_CREATES).to_string()
        }
        (_, FeatureStanding::NeedsTrust) => settings_projects_feature_needs_trust(feature.description()),
        (_, standing) => standing.hint(feature).unwrap_or_else(|| feature.description().to_string()),
    }
}

/// A feature's row: what it uses and its control (a switch for the defaults, Default | On |
/// Off for a project), faint while the project has no such file.
pub(super) fn feature_row(
    ui: UiColors,
    feature: Feature,
    description: String,
    state: SettingState,
    control: AnyElement,
    cx: &App,
) -> AnyElement {
    style::setting(
        SharedString::from(format!("feature-row-{}", feature.key())),
        ui,
        SettingParts {
            label: feature.label().into(),
            description: Some(description.into()),
            visible_hint: state == SettingState::Disabled,
            chip: None,
            keys: RowKeys::NONE,
            state,
            found: None,
            found_description: None,
            control,
            tall: false,
            extra: None,
            on_reset: None,
        },
        cx,
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use wormhole_core::project::{Feature, FeatureStanding};

    use super::feature_description;

    /// The page words a feature the way the Project tab does: a file that sets nothing says so
    /// rather than waiting for trust, and what is held back says it waits.
    #[test]
    fn a_feature_row_says_what_the_project_tab_says() {
        assert_eq!(
            feature_description(Feature::Settings, FeatureStanding::Empty),
            "Nothing set yet in config.json"
        );
        assert_eq!(
            feature_description(Feature::Actions, FeatureStanding::Missing),
            "No actions.json in this project"
        );
        assert_eq!(
            feature_description(Feature::Settings, FeatureStanding::NeedsTrust),
            format!("{}. Only after you trust the project", Feature::Settings.description())
        );
        assert_eq!(feature_description(Feature::Theme, FeatureStanding::Ready), Feature::Theme.description());
        // Notes can be switched on without the file: that makes it.
        assert!(feature_description(Feature::Notes, FeatureStanding::Missing).contains("On creates it"));
    }
}
