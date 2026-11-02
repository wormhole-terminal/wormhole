//! What a feature row says and how it is drawn, for both views.

use std::path::Path;

use gpui_kit::*;
use wormhole_core::project::{Feature, FeatureStanding};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::features::project;
use crate::select::{MenuOptions, menu_select};
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

/// A row of the page: `label` with `description` behind its (i), or under it while `visible_hint` (a
/// warning), faint when `state` is disabled.
pub(super) fn row(
    id: impl Into<ElementId>,
    ui: UiColors,
    label: &str,
    description: String,
    state: SettingState,
    control: AnyElement,
    cx: &App,
) -> AnyElement {
    style::setting(
        id,
        ui,
        SettingParts {
            label: label.into(),
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

/// A feature's row: what it uses and its control (a switch for the defaults, a menu for a project),
/// faint while the project has no such file.
pub(super) fn feature_row(
    ui: UiColors,
    feature: Feature,
    description: String,
    state: SettingState,
    control: AnyElement,
    cx: &App,
) -> AnyElement {
    row(
        SharedString::from(format!("feature-row-{}", feature.key())),
        ui,
        feature.label(),
        description,
        state,
        control,
        cx,
    )
}

/// What a project's own choice for a feature can be, in the menu's order: the default, on, off.
const CHOICES: [Option<bool>; 3] = [None, Some(true), Some(false)];

/// A project's own choice for `feature`: Default (on or off, as the defaults say), On or Off, in a menu.
/// Nothing is written into the project folder.
pub(super) fn feature_menu(
    ui: UiColors,
    feature: Feature,
    root: &Path,
    current: Option<bool>,
    default: bool,
) -> AnyElement {
    let words = [
        SharedString::from(if default {
            text(SETTINGS_PROJECTS_DEFAULT_ON)
        } else {
            text(SETTINGS_PROJECTS_DEFAULT_OFF)
        }),
        text(SETTINGS_PROJECTS_ON).into(),
        text(SETTINGS_PROJECTS_OFF).into(),
    ];
    let at = CHOICES.iter().position(|choice| *choice == current);
    let face = words[at.unwrap_or(0)].clone();
    let root = root.to_path_buf();
    menu_select(
        SharedString::from(format!("feature-{}", feature.key())),
        ui,
        feature.label(),
        face,
        move |_| {
            MenuOptions::words(words, at, move |ix, _, cx| {
                if let Some(choice) = CHOICES.get(ix) {
                    project::set_feature_for(
                        &project::shown_profile(cx).clone(),
                        &root,
                        feature,
                        *choice,
                        cx,
                    );
                }
            })
        },
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
