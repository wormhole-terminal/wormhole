//! The controls of each section, one file each, and what pages share: the select, the
//! page's header action, opening a file in the editor.

use std::path::Path;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;

use crate::select::{Choice, SelectTrigger};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore};
use crate::settings_window::registry::Setting;
use crate::settings_window::{SettingsWindow, ui};

mod advanced;
mod agent_access;
mod agent_skills;
mod agents;
mod appearance;
mod cli_tool;
mod extensions;
mod extensions_sheet;
mod extensions_text;
mod feature_flags;
mod fonts;
mod git;
mod import;
mod menu_bar;
mod menu_bar_preview;
mod motion;
mod motion_preview;
mod notifications;
mod notifications_preview;
mod preview;
mod profile;
mod profile_form;
mod profile_sheet;
mod projects;
mod samples;
mod sessions;
mod stage;
mod terminal;
mod theme_picker;
mod title_row;
mod updates;

pub(super) use agents::AgentsSnapshot;
pub(super) use cli_tool::CliToolPanel;
pub(super) use extensions::Extensions;
pub(super) use feature_flags::is_unlock_key;
pub(super) use fonts::draw_specimen;
pub(crate) use import::ImportPage;
pub(super) use menu_bar_preview::draw_menu_bar_preview;
#[cfg(test)]
pub(super) use motion_preview::MotionPreview;
pub(super) use motion_preview::{MotionPreviews, Scene as MotionScene};
pub(super) use notifications_preview::draw_notifications_preview;
pub(super) use preview::{draw_agents_preview, draw_sidebar_preview};
pub(super) use profile_form::ProfileForm;
pub(super) use profile_sheet::NewProfileSheet;
pub(crate) use profile_sheet::open_new_profile_sheet;
pub(super) use projects::ProjectPicker;
pub(super) use stage::draw_stage;
pub(super) use theme_picker::ThemePicker;

/// The controls of a section's rows (their names and words are in core's settings index).
pub(super) fn controls(section: Section) -> &'static [Setting] {
    match section {
        Section::Profile => profile::SETTINGS,
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

/// The selects of the pages, by their row's id (devctl opens them by name).
#[cfg(feature = "devctl")]
pub(super) fn select_id(id: &str) -> Option<&'static str> {
    [
        "scrollback",
        "icon-theme",
        "default-shell",
        "open-in",
        "motion-insert",
        "motion-fold",
        "motion-status",
        "motion-column",
    ]
    .into_iter()
    .find(|known| *known == id)
}

impl SettingsWindow {
    pub(super) fn render_section(&self, section: Section, cx: &mut Context<Self>) -> AnyElement {
        self.render_page(section, cx)
    }

    /// What sits at the right of a page's title (Coding agents: Rescan PATH).
    pub(super) fn header_action(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        match self.section {
            Section::Agents => {
                let this = cx.weak_entity();
                let native = self.native_button(
                    "agents-rescan",
                    text(SETTINGS_WINDOW_RESCAN_PATH),
                    self.agents_task.is_none(),
                    cx,
                    move |_, cx| {
                        this.update(cx, |this, cx| {
                            this.load_agents(cx);
                            cx.notify();
                        })
                        .ok();
                    },
                );
                Some(native.unwrap_or_else(|| {
                    Button::new("agents-rescan")
                        .outline()
                        .small()
                        .icon(IconName::RefreshCw)
                        .label(text(SETTINGS_WINDOW_RESCAN_PATH))
                        .loading(self.agents_task.is_some())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.load_agents(cx);
                            cx.notify();
                        }))
                        .into_any_element()
                }))
            }
            Section::Import => {
                let this = cx.weak_entity();
                let native = self.native_button(
                    "import-rescan",
                    text(SETTINGS_IMPORT_SCAN_AGAIN),
                    !self.import.scanning(),
                    cx,
                    move |_, cx| {
                        this.update(cx, |this, cx| this.scan_import(cx)).ok();
                    },
                );
                Some(native.unwrap_or_else(|| {
                    Button::new("import-rescan")
                        .outline()
                        .small()
                        .icon(IconName::RefreshCw)
                        .label(text(SETTINGS_IMPORT_SCAN_AGAIN))
                        .loading(self.import.scanning())
                        .on_click(cx.listener(|this, _, _, cx| this.scan_import(cx)))
                        .into_any_element()
                }))
            }
            Section::Extensions => Some(extensions::header_action(cx)),
            Section::Profile => {
                let this = cx.weak_entity();
                let native = self.native_button(
                    "profile-new",
                    text(SETTINGS_WINDOW_NEW_PROFILE),
                    true,
                    cx,
                    move |window, cx| {
                        this.update(cx, |this, cx| this.open_new_profile(window, cx)).ok();
                    },
                );
                Some(native.unwrap_or_else(|| {
                    Button::new("profile-new")
                        .outline()
                        .small()
                        .icon(IconName::Plus)
                        .label(text(SETTINGS_WINDOW_NEW_PROFILE))
                        .on_click(cx.listener(|this, _, window, cx| this.open_new_profile(window, cx)))
                        .into_any_element()
                }))
            }
            _ => None,
        }
    }

    /// A select in a row: its value in the face, a menu of `choices` under it.
    /// `id` names the row; only one select is open at a time.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn select(
        &self,
        id: &'static str,
        face: AnyElement,
        width: f32,
        choices: Vec<Choice<String>>,
        current: String,
        pick: fn(&mut Settings, &str),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        use crate::platform;
        use gpui_kit::base::Button as BaseButton;

        let anchor = platform::Anchor::default();
        let menu_anchor = anchor.clone();
        let label = choices
            .iter()
            .find(|choice| choice.value == current)
            .map(|choice| choice.label.clone())
            .unwrap_or_default();
        let menu = settings_choice_menu(choices, &current, pick);

        // devctl asks to open this same menu by row id. The platform records it in
        // background mode, just as it does menus opened by a real click.
        #[cfg(feature = "devctl")]
        if self.open_select == Some(id) {
            if let Some((handle, _)) = super::current_window(cx) {
                let this = cx.weak_entity();
                let menu = menu.clone();
                let anchor = anchor.clone();
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, cx| {
                        let should_open = this
                            .update(cx, |page, _| {
                                if page.open_select == Some(id) {
                                    page.open_select = None;
                                    true
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false);
                        if should_open {
                            platform::show_menu(menu, anchor.below(), window, cx);
                        }
                    });
                });
            }
        }

        div()
            .relative()
            .debug_selector(move || format!("select-{id}-trigger"))
            .child(anchor.probe())
            .child(
                BaseButton::new(SharedString::from(format!("select-{id}-trigger")))
                    .accessibility_label(label)
                    .on_click(move |_, window, cx| {
                        platform::show_menu(menu.clone(), menu_anchor.below(), window, cx);
                    })
                    .child(
                        SelectTrigger::new(SharedString::from(format!("select-{id}-face")), ui, face)
                            .width(px(width)),
                    ),
            )
            .into_any_element()
    }
}

/// Build the same menu model for AppKit and the platform's drawn fallback.
fn settings_choice_menu(
    choices: Vec<Choice<String>>,
    current: &str,
    pick: fn(&mut Settings, &str),
) -> crate::platform::Menu {
    use crate::platform::{Item, Menu};
    choices.into_iter().fold(Menu::new(), |menu, choice| {
        let selected = choice.value == current;
        let value = choice.value;
        menu.item(Item::new(choice.label).subtitle(choice.detail).checked(selected).on_select(
            move |_, cx| {
                update_settings(cx, |settings| pick(settings, &value));
            },
        ))
    })
}

#[cfg(test)]
mod choice_menu_tests {
    use super::settings_choice_menu;
    use crate::platform::menu::{Check, Entry};
    use crate::select::Choice;

    #[test]
    fn menu_preserves_choices_details_and_selection() {
        let choices = vec![
            Choice::new("a".to_string(), "First"),
            Choice::new("b".to_string(), "Second").detail("Unavailable"),
        ];
        let menu = settings_choice_menu(choices, "b", |_, _| {});
        let items: Vec<_> = menu
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Item(item) => Some(item),
                _ => None,
            })
            .collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title.as_ref(), "First");
        assert_eq!(items[0].check, Check::Off);
        assert_eq!(items[1].title.as_ref(), "Second");
        assert_eq!(items[1].subtitle.as_deref(), Some("Unavailable"));
        assert_eq!(items[1].check, Check::On);
        assert!(items.iter().all(|item| item.on_select.is_some()));
        assert_eq!(menu.outline(), ["- First", "- Second — Unavailable [x]"]);
    }
}

/// A row's button that opens something (a folder, a file).
fn open_button(
    window: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
    id: &'static str,
    icon: IconName,
    label: &'static str,
    open: fn(&App),
) -> AnyElement {
    window.native_button(id, label, true, cx, move |_, cx| open(cx)).unwrap_or_else(|| {
        Button::new(id)
            .outline()
            .small()
            .icon(icon)
            .label(label)
            .on_click(move |_, _, cx| open(cx))
            .into_any_element()
    })
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
