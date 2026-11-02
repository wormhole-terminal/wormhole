//! The view of all projects: the defaults every project starts from, and the trusted projects.

use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::project::Feature;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::features::feature_row;
use super::picker::{choice_label, short_home};
use crate::features::project;
use crate::settings_window::nav::page_tile;
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, SettingState};

impl SettingsWindow {
    /// The defaults every project starts from, one switch per feature, under the "For" row.
    pub(super) fn defaults_group(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let mut rows = vec![self.for_row(ui, cx)];
        for feature in Feature::ALL {
            let on = project::api::default_enabled(feature, cx);
            let profile = project::shown_profile(cx).clone();
            let switch = style::toggle(
                SharedString::from(format!("default-{}", feature.key())),
                on,
                move |on, _, cx| project::set_feature_default(&profile, feature, on, cx),
            );
            let description = if feature.requires_trust() {
                settings_projects_feature_needs_trust(feature.description())
            } else {
                feature.description().to_string()
            };
            rows.push(feature_row(
                ui,
                feature,
                description,
                SettingState::Default,
                switch.into_any_element(),
                cx,
            ));
        }
        div()
            .flex()
            .flex_col()
            .child(style::setting_group_head(ui, text(SETTINGS_PROJECTS_USE_FROM_PROJECTS)))
            .child(style::group_card(ui, rows))
            .into_any_element()
    }

    /// The trusted projects, a tile row each, with the ways into a project's page and out of its trust.
    pub(super) fn trusted_group(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let head = style::setting_group_head(ui, text(SETTINGS_PROJECTS_TRUSTED_PROJECTS));
        let mut trusted = div().flex().flex_col().child(head);
        let list: Vec<PathBuf> = project::api::trusted_roots(cx);
        let mut rows = Vec::new();
        if list.is_empty() {
            trusted = trusted.child(style::empty_list(
                ui,
                style::EmptyList {
                    icon: IconName::Folder,
                    title: text(SETTINGS_PROJECTS_NONE_YET).into(),
                    hint: text(SETTINGS_PROJECTS_NONE_HINT).into(),
                    actions: Vec::new(),
                },
            ));
        }
        for (ix, root) in list.into_iter().enumerate() {
            let tile =
                style::RowTile::Icon { fill: page_tile(Section::Projects, cx), icon: IconName::Folder };
            let (open, revoke) = (root.clone(), root.clone());
            let mut parts = style::TileRowParts::new(tile, choice_label(&root));
            parts.chip = Some((style::ChipKind::Good, text(SETTINGS_PROJECTS_TRUSTED).into()));
            parts.hint = Some(short_home(&root).into());
            parts.controls = vec![
                style::form_button(("trusted-open", ix))
                    .ghost()
                    .label(text(SETTINGS_PROJECTS_SETTINGS))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.projects.select(open.clone());
                        cx.notify();
                    }))
                    .into_any_element(),
                style::form_button(("revoke", ix))
                    .ghost()
                    .label(text(SETTINGS_PROJECTS_REVOKE))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let root = revoke.clone();
                        project::revoke_project(&project::shown_profile(cx).clone(), &root, cx);
                        this.projects.forget_info();
                    }))
                    .into_any_element(),
            ];
            rows.push(style::tile_row(("trusted-project", ix), ui, parts, cx).into_any_element());
        }
        if !rows.is_empty() {
            trusted = trusted.child(style::group_card(ui, rows));
        }
        if !*self.projects.can_approve.get_or_init(crate::platform::auth::available) {
            trusted = trusted.child(div().pb_2().child(style::notice(
                ui,
                style::NoticeKind::Warning,
                text(SETTINGS_PROJECTS_APPROVAL_ISNT_AVAILABLE_ON_THIS_MAC),
            )));
        }
        trusted.into_any_element()
    }
}
