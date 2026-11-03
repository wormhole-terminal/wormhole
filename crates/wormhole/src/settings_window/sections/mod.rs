//! The controls of each section, one file each, and what pages share: the select, the
//! page's header action, opening a file in the editor.

use std::path::Path;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;

use crate::settings::SettingsStore;
use crate::settings_window::SettingsWindow;
use crate::settings_window::registry::Setting;
use crate::style;

mod advanced;
mod agent_access;
mod agent_skills;
mod agents;
mod appearance;
mod card;
mod cli_tool;
mod color_rows;
mod diff_preview;
mod extensions;
mod extensions_sheet;
mod extensions_text;
mod feature_flags;
mod fonts;
mod git;
mod import;
mod list_preview;
mod menu_bar;
mod menu_bar_preview;
mod motion;
mod motion_preview;
mod notifications;
mod notifications_preview;
mod profile;
mod profile_form;
mod profile_sheet;
mod projects;
mod samples;
mod sessions;
mod split_preview;
mod stage;
mod terminal;
mod theme_picker;
mod title_row;
mod updates;

pub(super) use agents::AgentsSnapshot;
pub(super) use cli_tool::CliToolPanel;
pub(super) use color_rows::in_force as colors_in_force;
pub(super) use diff_preview::draw_diff_preview;
#[cfg(feature = "devctl")]
pub(super) use {
    appearance::draw_theme, color_rows::draw_primary, fonts::draw_specimen, stage::draw_stage,
    title_row::draw_parts,
};
pub(super) use extensions::Extensions;
pub(super) use feature_flags::is_unlock_key;
pub(crate) use import::ImportPage;
pub(super) use list_preview::{draw_agents_preview, draw_sidebar_preview};
pub(super) use menu_bar_preview::draw_menu_bar_preview;
#[cfg(test)]
pub(super) use motion_preview::MotionPreview;
pub(super) use motion_preview::{MotionPreviews, Scene as MotionScene};
pub(super) use notifications_preview::draw_notifications_preview;
pub(super) use profile::ProfileRows;
pub(super) use profile_form::ProfileForm;
pub(super) use profile_sheet::NewProfileSheet;
pub(crate) use profile_sheet::open_new_profile_sheet;
pub(super) use projects::ProjectPicker;
pub(super) use split_preview::draw_split_preview;
pub(super) use terminal::TerminalRows;
pub(super) use theme_picker::ThemePicker;

/// The controls of a section's rows (their names and words are in core's settings index).
pub(super) fn controls(section: Section) -> &'static [Setting] {
    match section {
        Section::Profile => profile::SETTINGS,
        // Other pages' rows, drawn by the page itself (`changed`).
        Section::Changed => &[],
        Section::Appearance => appearance::SETTINGS,
        Section::Fonts => fonts::SETTINGS,
        Section::Motion => motion::SETTINGS,
        Section::Terminal => terminal::SETTINGS,
        Section::Sessions => sessions::SETTINGS,
        Section::Git => git::SETTINGS,
        Section::Agents => agents::SETTINGS,
        Section::Import => import::SETTINGS,
        Section::Projects => projects::SETTINGS,
        Section::Extensions => extensions::SETTINGS,
        Section::FeatureFlags => feature_flags::SETTINGS,
        Section::Notifications => notifications::SETTINGS,
        Section::MenuBar => menu_bar::SETTINGS,
        Section::Updates => updates::SETTINGS,
        Section::Advanced => advanced::SETTINGS,
    }
}

/// The recorders of the pages, by their row's id (devctl starts them by name).
#[cfg(feature = "devctl")]
pub(super) fn recorder_id(id: &str) -> Option<&'static str> {
    ["hotkey", "jump-waiting-key"].into_iter().find(|known| *known == id)
}

/// What a page says above its rows that is of the moment: Notifications while macOS has not allowed banners.
pub(super) fn page_notice(section: Section, cx: &mut Context<SettingsWindow>) -> Option<AnyElement> {
    match section {
        Section::Notifications => notifications::notice(cx),
        Section::Updates => updates::dev_notice(cx),
        // The page says in one line what the design's notice says: the flags are for features that are
        // not finished.
        Section::FeatureFlags => Some(
            style::notice(crate::settings::ui(cx), style::NoticeKind::Info, section.info().intro())
                .into_any_element(),
        ),
        _ => None,
    }
}

impl SettingsWindow {
    pub(super) fn render_section(&self, section: Section, cx: &mut Context<Self>) -> AnyElement {
        if section == Section::Changed {
            return self.render_changed(cx);
        }
        self.render_page(section, cx)
    }

    /// What the design keeps off a page's title row and puts in its last line: rescanning the PATH
    /// (Coding agents), scanning for terminals again (Import) and a new profile (Profile).
    pub(super) fn page_footer(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let line = match self.section {
            Section::Agents if self.agents_task.is_none() => style::footer("").link(
                text(SETTINGS_WINDOW_RESCAN_PATH),
                cx.listener(|this, _, _, cx| {
                    this.load_agents(cx);
                    cx.notify();
                }),
            ),
            Section::Import if !self.import.scanning() => style::footer("")
                .link(text(SETTINGS_IMPORT_SCAN_AGAIN), cx.listener(|this, _, _, cx| this.scan_import(cx))),
            Section::Profile => style::footer("").link(
                text(SETTINGS_WINDOW_NEW_PROFILE),
                cx.listener(|this, _, window, cx| this.open_new_profile(window, cx)),
            ),
            _ => return None,
        };
        Some(div().child(line).into_any_element())
    }
}

/// A row's button that opens something (a folder, a file).
fn open_button(id: &'static str, label: &'static str, open: fn(&App)) -> AnyElement {
    style::form_button(id).label(label).on_click(move |_, _, cx| open(cx)).into_any_element()
}

/// Open a file in the text editor (config.json).
pub(super) fn open_in_editor(path: &Path) {
    if let Err(err) = crate::platform::apps::open_in_text_editor(path) {
        tracing::warn!("opening {}: {err}", path.display());
    }
}

/// Show a folder in the Finder.
// GPUI's test platform panics on `open_with_system` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a view
// test that clicks this needs a stand-in first.
fn reveal(path: &Path, cx: &App) {
    cx.open_with_system(path);
}

/// Import a theme file the person picks; a failure is said on the Appearance page.
fn import_theme(this: WeakEntity<SettingsWindow>, cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some(text(SETTINGS_WINDOW_IMPORT_THEME).into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(path) = paths.into_iter().next() else { return };
        cx.update(|cx| {
            if let Err(err) = SettingsStore::import_theme(cx, &path) {
                this.update(cx, |this, cx| {
                    this.notice =
                        Some((Section::Appearance, settings_window_import_failed(&err.to_string()).into()));
                    cx.notify();
                })
                .ok();
            }
            cx.refresh_windows();
        });
    })
    .detach();
}
