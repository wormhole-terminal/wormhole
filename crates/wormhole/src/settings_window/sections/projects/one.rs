//! One project: its trust, its own feature switches, its problems and what to do with its folder.

use std::path::Path;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;
use wormhole_core::project::{Feature, FeatureStanding, TrustStatus, trust_applies};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::super::reveal;
use super::features::{feature_description, feature_row};
use crate::features::project;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, SettingState, TextRole};
use crate::window::window_lifecycle::with_front_workspace;

impl SettingsWindow {
    pub(super) fn render_project(&self, root: &Path, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let Some(project) = self.projects.info(root, cx) else {
            return style::notice(
                ui,
                style::NoticeKind::Warning,
                settings_projects_missing_folder(&root.display().to_string()),
            )
            .into_any_element();
        };
        let (approving, hub_error) = project::api::approval(cx);
        let prefs: Vec<(Feature, FeatureStanding, Option<bool>, bool)> =
            project::api::feature_rows(root, &project, cx);
        let needing = project.snapshot.needing_trust();
        let has_commands = project.request.as_ref().is_some_and(|r| !r.commands.is_empty());

        // Trust: a badge for where it stands, the one action that fits.
        let (status, color) = match project.status {
            TrustStatus::Trusted => (text(SETTINGS_PROJECTS_TRUSTED), ui.success_text),
            TrustStatus::Changed => (text(SETTINGS_PROJECTS_CHANGED_SINCE_YOU_TRUSTED_IT), ui.warning_text),
            TrustStatus::Untrusted => (text(SETTINGS_PROJECTS_NOT_TRUSTED), ui.muted),
        };
        let detail = trust_detail(project.status, has_commands, &needing);
        let trusted = project.status == TrustStatus::Trusted;
        let trust_root = root.to_path_buf();
        let trust_button = if trusted {
            style::form_button("project-revoke")
                .outline()
                .label(text(SETTINGS_PROJECTS_REVOKE_TRUST))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let root = trust_root.clone();
                    project::revoke_project(&project::shown_profile(cx).clone(), &root, cx);
                    this.projects.forget_info();
                }))
        } else {
            style::form_button("project-trust")
                .primary()
                .icon(IconName::FingerprintPattern)
                .label(if approving {
                    text(SETTINGS_PROJECTS_WAITING_FOR_APPROVAL)
                } else {
                    text(SETTINGS_PROJECTS_TRUST_WITH_TOUCH_ID)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    let root = trust_root.clone();
                    project::approve_project(&project::shown_profile(cx).clone(), &root, cx);
                    this.projects.forget_info();
                }))
        };
        let mut trust = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_TRUST), None))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .min_h(px(style::SETTING_ROW_MIN_H))
                    .py_2()
                    .rule_top(ui)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_body()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(text(SETTINGS_PROJECTS_THIS_PROJECT)),
                                    )
                                    .child(style::status_badge(ui, color, status)),
                            )
                            .child(div().text_caption().text_color(ui.muted).child(detail)),
                    )
                    .child(trust_button),
            );
        if let Some(request) = project.request.as_ref().filter(|r| !trusted && !r.commands.is_empty()) {
            trust = trust.child(
                div().pb_2().child(
                    style::card(ui, ui.border)
                        .flex()
                        .flex_col()
                        .gap_1()
                        .text_caption()
                        .font_family(crate::fonts::code_font(cx).0)
                        .children(request.commands.iter().map(|c| div().child(c.to_string()))),
                ),
            );
        }
        if let Some(error) = hub_error {
            trust = trust.child(div().pb_2().child(style::notice(ui, style::NoticeKind::Danger, error)));
        }

        // Features: default / on / off per project, as the panel's small segmented control.
        let mut features = div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_USE_FROM_THIS_PROJECT), None))
            .child(
                div()
                    .pb_2()
                    .text_body()
                    .text_color(ui.muted)
                    .child(text(SETTINGS_PROJECTS_ONLY_FOR_YOU_NOTHING_IS_WRITTEN)),
            );
        for (feature, standing, current, default) in prefs {
            let description = feature_description(feature, standing);
            let key = feature.key();
            let options = [
                (
                    None,
                    if default {
                        text(SETTINGS_PROJECTS_DEFAULT_ON)
                    } else {
                        text(SETTINGS_PROJECTS_DEFAULT_OFF)
                    },
                ),
                (Some(true), text(SETTINGS_PROJECTS_ON)),
                (Some(false), text(SETTINGS_PROJECTS_OFF)),
            ];
            let control = style::segmented(ui)
                .children(options.into_iter().enumerate().map(|(ix, (value, label))| {
                    let root = root.to_path_buf();
                    style::segment(
                        SharedString::from(format!("feature-{key}-{ix}")),
                        ui,
                        label,
                        current == value,
                    )
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let root = root.clone();
                        project::set_feature_for(
                            &project::shown_profile(cx).clone(),
                            &root,
                            feature,
                            value,
                            cx,
                        )
                    }))
                }))
                .into_any_element();
            // Notes without a file are still for the person to switch on: that makes the file.
            let state = if standing == FeatureStanding::Missing && feature != Feature::Notes {
                SettingState::Disabled
            } else {
                SettingState::Default
            };
            features = features.child(feature_row(ui, feature, description, state, control, cx));
        }
        let features = features.rule_bottom(ui);

        let errors = (!project.errors.is_empty()).then(|| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_PROBLEMS), None))
                .children(
                    project.errors.iter().map(|e| style::notice(ui, style::NoticeKind::Warning, e.clone())),
                )
        });

        let open_root = root.to_path_buf();
        let reveal_root = project.folder.dir.clone();
        let actions = div()
            .flex()
            .gap_2()
            .child(
                style::form_button("project-open-session")
                    .icon(IconName::SquareTerminal)
                    .label(text(SETTINGS_PROJECTS_OPEN_IN_A_NEW_TERMINAL))
                    .on_click(cx.listener(move |_, _, _, cx| {
                        let root = open_root.clone();
                        with_front_workspace(cx, |ws, window, cx| {
                            crate::background::activate_window(window, cx);
                            ws.spawn_session(None, Some(root), window, cx);
                        });
                    })),
            )
            .child(
                style::form_button("project-reveal")
                    .ghost()
                    .icon(IconName::ExternalLink)
                    .label(settings_projects_show_in_finder(&project.folder.dir_name()))
                    .on_click(move |_, _, cx| reveal(&reveal_root, cx)),
            );

        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(trust.rule_bottom(ui))
            .child(features)
            .children(errors)
            .child(actions)
            .into_any_element()
    }
}

/// What the Trust row says under the badge. Trusting applies more than commands: a `config.json`
/// with just `font_size` is held back too, so a project with no command still says what waits.
pub(super) fn trust_detail(status: TrustStatus, has_commands: bool, needing: &[Feature]) -> String {
    let applies = trust_applies(needing);
    match (status, has_commands, needing.is_empty()) {
        (TrustStatus::Trusted, true, _) => {
            text(SETTINGS_PROJECTS_ACTIONS_THE_TERMINAL_LAYOUT_AND_ENV).to_string()
        }
        (TrustStatus::Trusted, false, false) => settings_projects_trust_applies(&applies),
        (TrustStatus::Trusted, false, true) => text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string(),
        (TrustStatus::Changed, true, _) => {
            text(SETTINGS_PROJECTS_REVIEW_THE_COMMANDS_BELOW_AND_TRUST).to_string()
        }
        (TrustStatus::Changed, false, false) => settings_projects_trust_again(&applies),
        (TrustStatus::Changed, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST_NOW).to_string()
        }
        (TrustStatus::Untrusted, true, _) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_CAN_RUN_COMMANDS).to_string()
        }
        (TrustStatus::Untrusted, false, false) => settings_projects_trust_would_apply(&applies),
        (TrustStatus::Untrusted, false, true) => {
            text(SETTINGS_PROJECTS_NOTHING_IN_IT_NEEDS_TRUST).to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::project::{Feature, TrustStatus};

    use super::trust_detail;

    #[test]
    fn the_trust_row_names_what_waits_when_nothing_is_a_command() {
        let settings = [Feature::Settings];
        assert_eq!(
            trust_detail(TrustStatus::Untrusted, false, &settings),
            "Trusting it applies its settings in config.json."
        );
        assert_eq!(
            trust_detail(TrustStatus::Changed, false, &settings),
            "Trust it again to apply its settings in config.json."
        );
        assert_eq!(
            trust_detail(TrustStatus::Trusted, false, &settings),
            "Applies its settings in config.json."
        );
        // Commands keep the words they had; a project with nothing to approve says so.
        assert_eq!(
            trust_detail(TrustStatus::Untrusted, true, &settings),
            "Nothing in it can run commands until you trust it."
        );
        assert_eq!(trust_detail(TrustStatus::Untrusted, false, &[]), "Nothing in it needs trust.");
    }
}
