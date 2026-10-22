//! The controls of each section, one file each, and what pages share: the select, the
//! page's header action, opening a file in the editor.

use wormhole_core::project::Feature;
use wormhole_core::store::data_dir;

use super::*;
use crate::select::{Choice, SearchSelect, SelectTrigger};

mod advanced;
mod agents;
mod appearance;
mod cli_tool;
mod feature_flags;
mod fonts;
mod git;
mod menu_bar;
mod notifications;
mod preview;
mod profile;
mod profile_form;
mod profile_sheet;
mod projects;
mod sessions;
mod stage;
mod terminal;
mod theme_picker;
mod updates;

pub(super) use agents::AgentsSnapshot;
pub(super) use cli_tool::CliToolPanel;
pub(super) use fonts::draw_specimen;
pub(super) use preview::draw_sidebar_preview;
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
        Section::Terminal => terminal::SETTINGS,
        Section::Sessions => sessions::SETTINGS,
        Section::Git => git::SETTINGS,
        Section::Agents => agents::SETTINGS,
        Section::Projects => projects::SETTINGS,
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
    ["scrollback", "icon-theme", "default-shell", "open-in"].into_iter().find(|known| *known == id)
}

impl SettingsWindow {
    pub(super) fn render_section(&self, section: Section, cx: &mut Context<Self>) -> AnyElement {
        self.render_page(section, cx)
    }

    /// What sits at the right of a page's title (Coding agents: Rescan PATH).
    pub(super) fn header_action(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        match self.section {
            Section::Agents => Some(
                Button::new("agents-rescan")
                    .outline()
                    .small()
                    .icon(IconName::RefreshCw)
                    .label("Rescan PATH")
                    .loading(self.agents_task.is_some())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.load_agents(cx);
                        cx.notify();
                    }))
                    .into_any_element(),
            ),
            Section::Profile => Some(
                Button::new("profile-new")
                    .outline()
                    .small()
                    .icon(IconName::Plus)
                    .label("New profile…")
                    .on_click(cx.listener(|this, _, window, cx| this.open_new_profile(window, cx)))
                    .into_any_element(),
            ),
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
        let this = cx.weak_entity();
        let on_open = this.clone();
        SearchSelect::new(
            SharedString::from(format!("select-{id}")),
            ui,
            SelectTrigger::new(SharedString::from(format!("select-{id}-trigger")), ui, face).width(px(width)),
            &self.menu_search,
            cx,
        )
        .options(choices)
        .current(current)
        .open(self.open_select == Some(id))
        .without_search()
        .anchor(Anchor::TopRight)
        .menu_width(px(width))
        .on_open_change(move |open, _, cx| {
            on_open
                .update(cx, |this, cx| {
                    this.open_select = open.then_some(id);
                    cx.notify();
                })
                .ok();
        })
        .on_pick(move |value, _, cx| {
            let value = value.clone();
            this.update(cx, |this, cx| {
                this.open_select = None;
                cx.notify();
            })
            .ok();
            update_settings(cx, |s| pick(s, &value));
        })
        .into_popover()
        .into_any_element()
    }
}

/// A row's button that opens something (a folder, a file).
fn open_button(id: &'static str, icon: IconName, label: &'static str, open: fn()) -> AnyElement {
    Button::new(id)
        .outline()
        .small()
        .icon(icon)
        .label(label)
        .on_click(move |_, _, _| open())
        .into_any_element()
}

/// Open a file in the text editor (config.json).
pub(super) fn open_in_editor(path: &Path) {
    if let Err(err) = crate::platform::apps::open_in_text_editor(path) {
        tracing::warn!("opening {}: {err}", path.display());
    }
}

/// Show a folder in the Finder.
fn reveal(path: &Path) {
    let mut cmd = std::process::Command::new("open");
    cmd.arg(path);
    if let Err(err) = crate::platform::spawn::spawn_detached(cmd) {
        tracing::warn!("opening {}: {err}", path.display());
    }
}

/// Import a theme file the person picks; a failure is said on the Appearance page.
fn import_theme(this: WeakEntity<SettingsWindow>, cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Import theme".into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(path) = paths.into_iter().next() else { return };
        cx.update(|cx| {
            if let Err(err) = SettingsStore::import_theme(cx, &path) {
                this.update(cx, |this, cx| {
                    this.notice = Some((Section::Appearance, format!("Import failed: {err}").into()));
                    cx.notify();
                })
                .ok();
            }
            cx.refresh_windows();
        });
    })
    .detach();
}
