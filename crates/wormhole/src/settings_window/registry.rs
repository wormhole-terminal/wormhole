//! The settings' controls. What every setting is called, what it says, the words that find it
//! and the `config.json` keys it writes are data in core (`wormhole_core::config::settings_index`,
//! one table for the pages, the search and the command palette); this module gives each of its
//! rows a [`Control`] (how it is drawn, how it reads and writes the [`Settings`]) and draws a
//! page from the two: one renderer for every row (`style::setting`), with its changed, project
//! and search states.
//!
//! A row that is more than a switch, a segmented control or a stepper (a font select, the
//! theme cards) is a [`Control::Custom`] or [`Control::Block`]: a function of the window.
//! A new setting is a row in the core table plus a [`Setting`] in its section's list here;
//! a test fails while one has no other.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::config::settings_index::{self, Entry, Hit, Query, Values};
use wormhole_text::{Key, keys::*, text};

use super::{FrontWindow, SettingsWindow, section_icon, sections, ui};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::style::{self, Edges, SettingParts, SettingState, TextRole};

/// Draws part of a page that is more than a switch, a segmented control or a stepper.
pub(super) type Draw = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> AnyElement;
/// Says something more current than the table's description (the font in use).
pub(super) type Describe = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// A tag after a row's label that says something of the moment ("Time Sensitive").
pub(super) type Chip = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// What a row shows under itself, across the column, when it has something to say (the reset's
/// question).
pub(super) type Below = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<AnyElement>;

/// One option of a [`Control::Segmented`].
pub(super) struct Segment {
    /// The option's name, a message of the catalogs: a table of options is a `const`, which cannot
    /// read the language in use, so it is looked up as the control is drawn.
    pub label: Key,
    pub icon: Option<IconName>,
}

impl Segment {
    pub(super) const fn text(label: Key) -> Self {
        Self { label, icon: None }
    }

    pub(super) const fn icon(icon: IconName, label: Key) -> Self {
        Self { label, icon: Some(icon) }
    }
}

pub(super) enum Control {
    /// A switch.
    Toggle { get: fn(&Settings) -> bool, set: fn(&mut Settings, bool) },
    /// A few options in one segmented control, one of them current.
    Segmented { options: &'static [Segment], get: fn(&Settings) -> usize, set: fn(&mut Settings, usize) },
    /// A small number with a minus and a plus: `show` reads it as text, `nudge` moves it one
    /// step (`-1` or `1`; the settings are clamped when they are saved).
    Stepper { show: fn(&Settings) -> String, nudge: fn(&mut Settings, i32) },
    /// A shortcut, recorded by pressing it (empty is none).
    Recorder(super::recorder::RecorderRow),
    /// The control of a row, drawn by the page.
    Custom(Draw),
    /// Content that stands on its own, with no row around it (the theme cards, a preview).
    Block(Draw),
}

pub(super) struct Setting {
    /// The core table's id for the row.
    pub id: &'static str,
    pub control: Control,
    pub describe: Option<Describe>,
    pub below: Option<Below>,
    pub chip: Option<Chip>,
}

impl Setting {
    pub(super) const fn new(id: &'static str, control: Control) -> Self {
        Self { id, control, describe: None, below: None, chip: None }
    }

    pub(super) const fn toggle(
        id: &'static str,
        get: fn(&Settings) -> bool,
        set: fn(&mut Settings, bool),
    ) -> Self {
        Self::new(id, Control::Toggle { get, set })
    }

    pub(super) const fn segmented(
        id: &'static str,
        options: &'static [Segment],
        get: fn(&Settings) -> usize,
        set: fn(&mut Settings, usize),
    ) -> Self {
        Self::new(id, Control::Segmented { options, get, set })
    }

    pub(super) const fn stepper(
        id: &'static str,
        show: fn(&Settings) -> String,
        nudge: fn(&mut Settings, i32),
    ) -> Self {
        Self::new(id, Control::Stepper { show, nudge })
    }

    pub(super) const fn recorder(id: &'static str, row: super::recorder::RecorderRow) -> Self {
        Self::new(id, Control::Recorder(row))
    }

    pub(super) const fn custom(id: &'static str, draw: Draw) -> Self {
        Self::new(id, Control::Custom(draw))
    }

    pub(super) const fn block(id: &'static str, draw: Draw) -> Self {
        Self::new(id, Control::Block(draw))
    }

    /// A description that says what is in use now, instead of the table's.
    pub(super) const fn describe(self, describe: Describe) -> Self {
        Self { describe: Some(describe), ..self }
    }

    /// A tag after the label while there is something to say.
    pub(super) const fn chip(self, chip: Chip) -> Self {
        Self { chip: Some(chip), ..self }
    }

    /// Something under the row, across the column, while there is something to say.
    pub(super) const fn below(self, below: Below) -> Self {
        Self { below: Some(below), ..self }
    }
}

/// A switch for a `bool` field of the settings.
macro_rules! toggle {
    ($id:literal, $field:ident) => {
        Setting::toggle($id, |s| s.$field, |s, on| s.$field = on)
    };
}
pub(super) use toggle;

/// Every control, by its row's id.
fn controls() -> &'static HashMap<&'static str, (Section, &'static Setting)> {
    static CONTROLS: OnceLock<HashMap<&'static str, (Section, &'static Setting)>> = OnceLock::new();
    CONTROLS.get_or_init(|| {
        Section::ALL
            .into_iter()
            .flat_map(|section| sections::controls(section).iter().map(move |s| (s.id, (section, s))))
            .collect()
    })
}

/// The control of the row `id`.
fn control(id: &str) -> Option<&'static Setting> {
    controls().get(id).map(|(_, setting)| *setting)
}

/// What the recorder of row `id` records into and checks.
pub(super) fn recorder_row(id: &str) -> Option<super::recorder::RecorderRow> {
    match control(id)?.control {
        Control::Recorder(row) => Some(row),
        _ => None,
    }
}

/// The project whose `.wormhole/` folder applies right now (the active terminal's, in the
/// window in front): what the rows it sets say.
pub(super) struct ProjectSource {
    pub name: SharedString,
    /// Its project folder's name on disk (`.wormhole`, or an older project's `.andromeda`).
    pub dir: String,
    pub root: PathBuf,
    /// The config keys its `config.json` set.
    pub keys: Vec<String>,
    /// Its `theme.json` is shown.
    pub theme: bool,
}

impl ProjectSource {
    /// The project of the window in front (`front`), when it sets anything.
    pub(super) fn current(front: &FrontWindow, cx: &App) -> Option<Self> {
        let store = cx.global::<SettingsStore>();
        let (keys, theme) = (store.project_keys().to_vec(), store.has_project_theme());
        if keys.is_empty() && !theme {
            return None;
        }
        let root = front.project.clone()?;
        let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default().into();
        let dir = wormhole_core::project::dir_name_at(&root);
        Some(Self { name, dir, root, keys, theme })
    }

    /// Whether this project sets `entry` (its theme.json counts for the theme).
    fn sets(&self, entry: &Entry) -> bool {
        settings_index::set_by_project(entry, &self.keys) || (self.theme && entry.id == "theme")
    }
}

/// What every row of a page needs to know, read once per draw.
struct PageState {
    ui: UiColors,
    /// The user's own settings against the defaults.
    values: Values,
    project: Option<ProjectSource>,
}

impl PageState {
    fn new(front: &FrontWindow, cx: &App) -> Self {
        let store = cx.global::<SettingsStore>();
        Self {
            ui: ui(cx),
            values: Values::of(store.user_settings()),
            project: ProjectSource::current(front, cx),
        }
    }
}

/// What a setting shows right now, read before the row is built so no copy of the settings
/// is held while the row borrows the window.
enum Shown {
    Toggle(bool),
    Segmented(usize),
    Stepper(String),
    Drawn,
}

fn shown(control: &Control, settings: &Settings) -> Shown {
    match control {
        Control::Toggle { get, .. } => Shown::Toggle(get(settings)),
        Control::Segmented { get, .. } => Shown::Segmented(get(settings)),
        Control::Stepper { show, .. } => Shown::Stepper(show(settings)),
        Control::Recorder(_) | Control::Custom(_) | Control::Block(_) => Shown::Drawn,
    }
}

impl SettingsWindow {
    /// A section's page: each group of the table under its label, each row with its control,
    /// what the search found marked.
    pub(super) fn render_page(&self, section: Section, cx: &mut Context<Self>) -> AnyElement {
        let page = PageState::new(&self.front, cx);
        let query = Query::new(&self.search.read(cx).value());
        let hits: HashMap<&'static str, Hit> = settings_index::search_visible(&query, settings(cx))
            .into_iter()
            .map(|hit| (hit.entry.id, hit))
            .collect();
        let mut column = div().flex().flex_col().gap_6();
        for group in section.groups() {
            let mut rows = div().flex().flex_col();
            if !group.title().is_empty() {
                rows = rows.child(style::setting_group_head(
                    page.ui,
                    group.title(),
                    (!group.aside().is_empty()).then_some(group.aside()),
                ));
            }
            if !group.blurb().is_empty() {
                rows = rows.child(div().pb_2().text_body().text_color(page.ui.muted).child(group.blurb()));
            }
            let mut any_row = false;
            let mut shown = 0;
            for entry in group.entries.iter().filter(|entry| !self.in_column(entry.id)) {
                let Some(setting) = control(entry.id) else {
                    tracing::warn!("settings: no control for {}", entry.id);
                    continue;
                };
                shown += 1;
                any_row |= !matches!(setting.control, Control::Block(_));
                rows = rows.child(self.render_entry(entry, setting, &page, hits.get(entry.id), cx));
                if let Some(below) = setting.below.and_then(|below| below(self, cx)) {
                    rows = rows.child(div().pb_2().child(below));
                }
            }
            // A group that only held the preview the column shows has nothing left to draw.
            if shown == 0 {
                continue;
            }
            column = column.child(if any_row { rows.rule_bottom(page.ui) } else { rows });
        }
        column.into_any_element()
    }

    /// One setting: a block as it draws itself, anything else in a `style::setting` row.
    fn render_entry(
        &self,
        entry: &'static Entry,
        setting: &'static Setting,
        page: &PageState,
        hit: Option<&Hit>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = page.ui;
        if self.in_column(entry.id) {
            return self.render_preview_note(entry, hit, cx);
        }
        if let Control::Block(draw) = &setting.control {
            return draw(self, cx);
        }
        let current = shown(&setting.control, settings(cx));
        let project = page.project.as_ref().filter(|project| project.sets(entry));
        let state = match project {
            Some(project) => SettingState::Project(project.name.clone()),
            None if page.values.entry_changed(entry) => SettingState::Changed,
            None => SettingState::Default,
        };
        let description = match project {
            Some(project) => Some(SharedString::from(settings_window_set_by_project(
                &project.name,
                &project.dir,
                if entry.id == "theme" { "theme.json" } else { "config.json" },
            ))),
            None => setting
                .describe
                .and_then(|describe| describe(self, cx))
                .or_else(|| (!entry.description().is_empty()).then(|| entry.description().into())),
        };
        // A description of the moment is not the table's: its search marks don't fit it.
        let found_description = hit
            .and_then(|hit| hit.description_range.clone())
            .filter(|_| setting.describe.is_none() && project.is_none());
        let extra = project.map(|project| {
            let root = project.root.clone();
            style::link_button(
                SharedString::from(format!("edit-in-project-{}", entry.id)),
                ui,
                text(SETTINGS_WINDOW_EDIT_IN_PROJECT),
                false,
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.projects.select(root.clone());
                this.show_section(Section::Projects, cx);
            }))
            .into_any_element()
        });
        let keys = entry.keys;
        let chip = setting.chip.and_then(|chip| chip(self, cx));
        let control = self.render_control(setting, current, ui, cx);
        // The row the palette asked for (`reveal_row`): marked as a search hit on its whole
        // label, and anchored so the page can scroll to it.
        let revealed = self.revealed_row().filter(|(id, _)| *id == entry.id).map(|(_, anchor)| anchor);
        let found = hit
            .and_then(|hit| hit.label_range.clone())
            .or_else(|| revealed.as_ref().map(|_| 0..entry.label().len()));
        style::setting(
            SharedString::from(format!("setting-{}", entry.id)),
            ui,
            SettingParts {
                label: entry.label().into(),
                description,
                chip,
                key: keys.first().copied().unwrap_or(""),
                state,
                found,
                found_description,
                control,
                extra,
                on_reset: (!keys.is_empty()).then(|| -> style::ClickHandler {
                    Box::new(move |_, _, cx| update_settings(cx, |s| *s = settings_index::reset(s, keys)))
                }),
            },
            cx,
        )
        .anchor_scroll(revealed)
        .into_any_element()
    }

    fn render_control(
        &self,
        setting: &'static Setting,
        current: Shown,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = setting.id;
        match (&setting.control, current) {
            (Control::Toggle { set, .. }, Shown::Toggle(on)) => {
                let set = *set;
                style::toggle(id, on, move |on, _, cx| update_settings(cx, |s| set(s, on))).into_any_element()
            }
            (Control::Segmented { options, set, .. }, Shown::Segmented(active)) => {
                let set = *set;
                style::segmented_form(ui)
                    .children(options.iter().enumerate().map(|(ix, option)| {
                        style::form_segment((id, ix), ui, option.icon, text(option.label), ix == active)
                            .on_click(move |_, _, cx| update_settings(cx, |s| set(s, ix)))
                    }))
                    .into_any_element()
            }
            (Control::Stepper { nudge, .. }, Shown::Stepper(value)) => {
                let nudge = *nudge;
                style::stepper(
                    ui,
                    id,
                    value,
                    move |_, _, cx| update_settings(cx, |s| nudge(s, -1)),
                    move |_, _, cx| update_settings(cx, |s| nudge(s, 1)),
                )
                .into_any_element()
            }
            (Control::Recorder(row), _) => self.render_recorder(id, *row, cx),
            (Control::Custom(draw) | Control::Block(draw), _) => draw(self, cx),
            // `shown` reads each control as its own kind.
            _ => div().into_any_element(),
        }
    }

    /// Search results: the rows the query found, each a live row with its control,
    /// grouped by section and group with a breadcrumb and a way to the section.
    pub(super) fn render_results(&self, query: &Query, cx: &mut Context<Self>) -> AnyElement {
        let page = PageState::new(&self.front, cx);
        let ui = page.ui;
        let hits = settings_index::search_visible(query, settings(cx));
        let mut column = div().flex().flex_col().gap_6();
        let mut start = 0;
        while start < hits.len() {
            let (section, group) = (hits[start].section, hits[start].group);
            let end = hits[start..]
                .iter()
                .position(|hit| hit.section != section || !std::ptr::eq(hit.group, group))
                .map_or(hits.len(), |n| start + n);
            let head = div()
                .flex()
                .items_center()
                .gap_2()
                .h(px(style::CONTROL_H))
                .pb_1()
                .child(style::icon(section_icon(section)).text_color(ui.muted))
                .child(div().text_label().text_color(ui.text).child(section.title()))
                .when(!group.title().is_empty(), |d| {
                    d.child(div().text_caption().text_color(ui.faint).child(format!("› {}", group.title())))
                })
                .child(div().flex_1())
                .child(
                    style::link_button(
                        ("open-section", start),
                        ui,
                        settings_window_open_section(section.title()),
                        true,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.show_section(section, cx))),
                );
            let mut rows = div().flex().flex_col().child(head);
            for hit in &hits[start..end] {
                if let Some(setting) = control(hit.entry.id) {
                    rows = rows.child(self.render_entry(hit.entry, setting, &page, Some(hit), cx));
                }
            }
            column = column.child(rows.rule_bottom(ui));
            start = end;
        }
        column.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use wormhole_core::config::Settings;
    use wormhole_core::config::settings_index::{self, SectionId};

    use super::{Control, controls, sections};
    use crate::settings_window::Section;

    fn every_setting() -> impl Iterator<Item = &'static super::Setting> {
        Section::ALL.into_iter().flat_map(|section| sections::controls(section).iter())
    }

    #[test]
    fn every_row_of_the_table_has_a_control_on_its_page_and_back() {
        let mut seen = HashSet::new();
        for section in SectionId::ALL {
            for setting in sections::controls(section) {
                assert!(seen.insert(setting.id), "{} has two controls", setting.id);
                let (found, _, _) = settings_index::entry(setting.id)
                    .unwrap_or_else(|| panic!("{} is not in the settings index", setting.id));
                assert_eq!(found, section, "{} is drawn on another page than its row", setting.id);
            }
        }
        for (_, _, entry) in settings_index::entries() {
            assert!(controls().contains_key(entry.id), "{} has no control", entry.id);
        }
    }

    #[test]
    fn a_toggle_reads_back_what_it_wrote() {
        for setting in every_setting() {
            if let Control::Toggle { get, set } = &setting.control {
                let mut s = Settings::default();
                set(&mut s, true);
                assert!(get(&s), "{}", setting.id);
                set(&mut s, false);
                assert!(!get(&s), "{}", setting.id);
            }
        }
    }

    #[test]
    fn a_segmented_control_reads_back_every_option() {
        for setting in every_setting() {
            if let Control::Segmented { options, get, set } = &setting.control {
                assert!(options.len() > 1, "{} has nothing to choose", setting.id);
                for ix in 0..options.len() {
                    let mut s = Settings::default();
                    set(&mut s, ix);
                    assert_eq!(get(&s), ix, "{} option {ix}", setting.id);
                }
            }
        }
    }

    #[test]
    fn a_stepper_moves_its_value_both_ways() {
        for setting in every_setting() {
            if let Control::Stepper { show, nudge } = &setting.control {
                let start = Settings::default();
                let mut up = start.clone();
                nudge(&mut up, 1);
                let mut down = start.clone();
                nudge(&mut down, -1);
                assert_ne!(show(&up), show(&start), "{} does not go up", setting.id);
                assert_ne!(show(&down), show(&start), "{} does not go down", setting.id);
            }
        }
    }
}
