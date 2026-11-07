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
use std::rc::Rc;
use std::sync::OnceLock;

use gpui_kit::assets::IconName;
use gpui_kit::component::{Disableable as _, switch::Switch};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::config::settings_index::{self, Entry, Hit, Query, Values};
use wormhole_core::theme::fonts::{FontSlot, SYSTEM_FONT, display_name};
use wormhole_core::theme::tokens::BranchTint;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use super::{FrontWindow, SettingsWindow, section_icon, sections, ui};
use crate::platform::{self, Item, Menu};
use crate::select::{Choice, SelectTrigger};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::style::{self, RowKeys, SettingParts, SettingState, TextRole};

/// The widest the summary of a block's value grows on the Changed page before it is cut.
const SUMMARY_MAX_W: f32 = 240.0;

/// Draws part of a page that is more than a switch, a segmented control or a stepper.
pub(super) type Draw = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> AnyElement;
/// Says something more current than the table's description (the font in use).
pub(super) type Describe = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// A tag after a row's label that says something of the moment ("Time Sensitive").
pub(super) type Chip = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// What a row shows under itself, across the column, when it has something to say (the reset's
/// question).
pub(super) type Below = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<AnyElement>;

/// What a row does when the person's attention comes to it (the pointer over it, its menu opened): the
/// page's preview takes up its setting.
pub(super) type Touched = fn(&mut SettingsWindow, &mut Context<SettingsWindow>);

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
    /// A short list of labelled values, opened as a platform menu.
    Select {
        width: f32,
        options: fn(&Settings) -> Vec<Choice<String>>,
        get: fn(&Settings) -> String,
        set: fn(&mut Settings, &str),
    },
    /// A shortcut, recorded by pressing it (empty is none).
    Recorder(super::recorder::RecorderRow),
    /// The control of a row, drawn by the page.
    Custom(Draw),
    /// Content that stands on its own, with no row around it (the theme cards, a preview).
    Block(Draw),
    /// A whole row the page draws itself (a permission and its button): it sits on the group's card with
    /// the rows around it, and is no setting of its own, so the Changed page never lists it.
    Item(Draw),
}

pub(super) struct Setting {
    /// The core table's id for the row.
    pub id: &'static str,
    pub control: Control,
    pub describe: Option<Describe>,
    pub below: Option<Below>,
    pub chip: Option<Chip>,
    pub touched: Option<Touched>,
    /// The control runs over more than one line (see `SettingParts::tall`).
    pub tall: bool,
}

impl Setting {
    pub(super) const fn new(id: &'static str, control: Control) -> Self {
        Self { id, control, describe: None, below: None, chip: None, touched: None, tall: false }
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

    pub(super) const fn select(
        id: &'static str,
        width: f32,
        options: fn(&Settings) -> Vec<Choice<String>>,
        get: fn(&Settings) -> String,
        set: fn(&mut Settings, &str),
    ) -> Self {
        Self::new(id, Control::Select { width, options, get, set })
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

    pub(super) const fn item(id: &'static str, draw: Draw) -> Self {
        Self::new(id, Control::Item(draw))
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

    /// Something to do when the row is hovered or its menu opened.
    pub(super) const fn touched(self, touched: Touched) -> Self {
        Self { touched: Some(touched), ..self }
    }

    /// A control of more than one line: the label sits beside its first line.
    pub(super) const fn tall(self) -> Self {
        Self { tall: true, ..self }
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
pub(super) fn control(id: &str) -> Option<&'static Setting> {
    controls().get(id).map(|(_, setting)| *setting)
}

/// The current ownership is checked at activation as well as at render time: a popup may have been
/// opened while the setting was still editable.
pub(super) fn project_sets(id: &str, cx: &App) -> bool {
    let store = cx.global::<SettingsStore>();
    (id == "theme" && store.has_project_theme())
        || settings_index::entry(id)
            .is_some_and(|(_, _, entry)| settings_index::set_by_project(entry, store.project_keys()))
}

pub(super) fn update_row(id: &str, cx: &mut App, edit: impl FnOnce(&mut Settings)) {
    if !project_sets(id, cx) {
        update_settings(cx, edit);
    }
}

/// Read the current choices when the menu opens, including custom saved values.
pub(super) fn select_menu(id: &str, cx: &App) -> Option<Menu> {
    let setting = control(id)?;
    let id = setting.id;
    if project_sets(id, cx) {
        return None;
    }
    let Control::Select { options, get, set, .. } = &setting.control else { return None };
    let current = get(settings(cx));
    let set = *set;
    Some(options(settings(cx)).into_iter().fold(Menu::new(), |menu, option| {
        let item = Item::new(option.label().clone()).checked(option.value == current);
        menu.item(item.on_select(move |_, cx| update_row(id, cx, |s| set(s, &option.value))))
    }))
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
pub(super) struct PageState {
    pub(super) ui: UiColors,
    /// The user's own settings against the defaults.
    pub(super) values: Values,
    project: Option<ProjectSource>,
    /// The rows are drawn on the Changed page, not on their own: each differs from its default, so its reset
    /// arrow is always there, and a block, which draws a whole page's worth, is a row that says what it is
    /// set to.
    changed_page: bool,
}

impl PageState {
    pub(super) fn new(front: &FrontWindow, cx: &App) -> Self {
        Self::read(front, false, cx)
    }

    /// For the rows of the Changed page.
    pub(super) fn for_changed_page(front: &FrontWindow, cx: &App) -> Self {
        Self::read(front, true, cx)
    }

    fn read(front: &FrontWindow, changed_page: bool, cx: &App) -> Self {
        let store = cx.global::<SettingsStore>();
        Self {
            ui: ui(cx),
            values: Values::of(store.user_settings()),
            project: ProjectSource::current(front, cx),
            changed_page,
        }
    }
}

/// The person's current values of `keys` as JSON, for a row's Copy as JSON. Their own, not what Zen mode or
/// a project overlays: the file is what is copied.
fn current_json(cx: &App, keys: &[&str]) -> String {
    Values::of(cx.global::<SettingsStore>().user_settings()).json(keys)
}

/// What a block's row says on the Changed page, where the block cannot draw its whole page: the values of the
/// keys that differ, as the snippet Copy as JSON would give, in one muted line.
fn block_summary(ui: UiColors, values: &Values, entry: &Entry) -> AnyElement {
    let changed: Vec<&str> = entry.keys.iter().copied().filter(|key| values.changed(key)).collect();
    div()
        .max_w(px(SUMMARY_MAX_W))
        .truncate()
        .text_caption()
        .text_color(ui.muted)
        .child(values.json(&changed))
        .into_any_element()
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
        Control::Select { .. }
        | Control::Recorder(_)
        | Control::Custom(_)
        | Control::Block(_)
        | Control::Item(_) => Shown::Drawn,
    }
}

/// A disabled switch still conveys its state. Other project values are readouts, so the
/// editable element, including any menu or focus handle in it, is never mounted.
fn project_control(entry: &Entry, setting: &Setting, current: Shown, ui: UiColors, cx: &App) -> AnyElement {
    let store = cx.global::<SettingsStore>();
    let settings = store.settings_without_zen();
    let value: SharedString = match (&setting.control, current) {
        (Control::Toggle { .. }, Shown::Toggle(on)) => {
            return div()
                .flex()
                .items_center()
                .gap_2()
                .children(
                    (entry.id == "primary-color")
                        .then(|| settings.primary())
                        .flatten()
                        .map(|color| project_readout(ui, color.to_hex().into())),
                )
                .child(Switch::new(entry.id).checked(on).disabled(true).accessibility_label(entry.label()))
                .into_any_element();
        }
        (Control::Segmented { options, .. }, Shown::Segmented(active)) => {
            let label = options.get(active).map_or("", |option| text(option.label));
            match (entry.id, settings.branch_tint()) {
                ("branch-color", Some(BranchTint::Color(color))) => {
                    format!("{label} · {}", color.to_hex()).into()
                }
                _ => label.into(),
            }
        }
        (Control::Stepper { .. }, Shown::Stepper(value)) => value.into(),
        (Control::Select { options, get, .. }, _) => {
            let value = get(settings);
            options(settings)
                .into_iter()
                .find(|option| option.value == value)
                .map_or_else(|| value.into(), |option| option.label().clone())
        }
        _ => match entry.id {
            "theme" => store.base_tokens().name.clone().into(),
            "opacity" => format!("{:.0} %", store.asked_opacity() * 100.0).into(),
            "line-height" => format!("{:.2}", settings.line_height).into(),
            "density" => settings.density.label().into(),
            "font-terminal" => project_font_value(store, FontSlot::Terminal),
            "font-interface" => project_font_value(store, FontSlot::Interface),
            "font-code" => project_font_value(store, FontSlot::Code),
            // A new project-scoped custom row still renders read-only if it has no formatted readout yet.
            _ => return Empty.into_any_element(),
        },
    };
    project_readout(ui, value).into_any_element()
}

fn project_readout(ui: UiColors, value: SharedString) -> Stateful<Div> {
    div()
        .id("project-readout")
        .role(Role::Label)
        .aria_label(value.clone())
        .max_w(px(SUMMARY_MAX_W))
        .truncate()
        .text_label()
        .text_color(ui.muted)
        .child(value)
}

fn project_font_value(store: &SettingsStore, slot: FontSlot) -> SharedString {
    let family = store.fonts.family(slot);
    let name = if family == SYSTEM_FONT { "SF Pro" } else { display_name(family) }; // text: name (the system font)
    let size = settings_fonts_size_points(&format!("{:.0}", store.settings.slot_size(slot)));
    format!("{name} · {size}").into()
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
        let mut column = div().flex().flex_col().gap_6().children(super::zen_note::draw(section, cx));
        for group in section.groups() {
            let mut page_group = div().flex().flex_col().gap_2();
            if !group.title().is_empty() {
                page_group = page_group.child(style::setting_group_head(
                    page.ui,
                    group.title(),
                    (!group.aside().is_empty()).then_some(group.aside()),
                ));
            }
            if !group.blurb().is_empty() {
                page_group = page_group.child(style::setting_group_blurb(page.ui, group.blurb()));
            }
            // The rows go on one card; a block draws itself, so it ends the card and stands between two.
            let mut rows: Vec<AnyElement> = Vec::new();
            let mut shown = 0;
            for entry in group.entries.iter().filter(|entry| !self.in_column(entry.id)) {
                // A row behind a feature flag that is off is not on the page at all.
                if !entry.is_shown(settings(cx)) {
                    continue;
                }
                let Some(setting) = control(entry.id) else {
                    tracing::warn!("settings: no control for {}", entry.id);
                    continue;
                };
                shown += 1;
                let entry_view = self.render_entry(entry, setting, &page, hits.get(entry.id), cx);
                if matches!(setting.control, Control::Block(_)) {
                    if !rows.is_empty() {
                        page_group =
                            page_group.child(style::setting_group(page.ui, std::mem::take(&mut rows)));
                    }
                    page_group = page_group.child(entry_view);
                    continue;
                }
                // What hangs under a row belongs to it: one item of the card, with no divider between.
                rows.push(match setting.below.and_then(|below| below(self, cx)) {
                    Some(below) => div()
                        .flex()
                        .flex_col()
                        .child(entry_view)
                        .child(div().pb_2().child(below))
                        .into_any_element(),
                    None => entry_view,
                });
            }
            // A group that only held the preview the column shows has nothing left to draw.
            if shown == 0 {
                continue;
            }
            if !rows.is_empty() {
                page_group = page_group.child(style::setting_group(page.ui, rows));
            }
            column = column.child(page_group);
        }
        column.into_any_element()
    }

    /// One setting: a block as it draws itself, anything else in a `style::setting` row. On the Changed page
    /// (`page.changed_page`) a block is a row too, as the rest are.
    pub(super) fn render_entry(
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
        let listed = page.changed_page;
        if let Control::Item(draw) = &setting.control {
            return draw(self, cx);
        }
        if let Control::Block(draw) = &setting.control
            && !listed
        {
            return draw(self, cx);
        }
        // The person's own values: Zen mode draws the window with less, and says so at the top of the page.
        let current = shown(&setting.control, crate::settings::settings_without_zen(cx));
        let project = page.project.as_ref().filter(|project| project.sets(entry));
        let changed = page.values.entry_changed(entry);
        let state = match project {
            Some(project) => SettingState::Project(project.name.clone()),
            None if changed => SettingState::Changed,
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
            .on_click(cx.listener(move |this, _, window, cx| {
                this.projects.select(root.clone());
                this.navigate_to(Section::Projects, window, cx);
            }))
            .into_any_element()
        });
        let keys = entry.keys;
        let chip = setting.chip.and_then(|chip| chip(self, cx));
        let control = if project.is_some() {
            project_control(entry, setting, current, ui, cx)
        } else if listed && matches!(setting.control, Control::Block(_)) {
            block_summary(ui, &page.values, entry)
        } else {
            self.render_control(setting, current, ui, cx)
        };
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
                visible_hint: entry.visible_hint && project.is_none(),
                chip,
                keys: RowKeys { keys, json: current_json },
                state,
                found,
                found_description,
                control,
                tall: setting.tall,
                extra,
                // Only a changed row can reset, so only it makes the closure.
                on_reset: (!keys.is_empty() && changed).then(|| -> style::Reset {
                    Rc::new(move |_, cx| update_row(entry.id, cx, |s| *s = settings_index::reset(s, keys)))
                }),
                reset_arrow: if listed {
                    crate::style::ResetArrow::Always
                } else {
                    crate::style::ResetArrow::OnHover
                },
            },
            cx,
        )
        .anchor_scroll(revealed)
        .when_some(setting.touched, |row, touched| {
            row.on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    touched(this, cx);
                }
            }))
        })
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
                let label = settings_index::entry(id).map_or("", |(_, _, entry)| entry.label());
                style::toggle(id, on, move |on, _, cx| update_row(id, cx, |s| set(s, on)))
                    .accessibility_label(label)
                    .into_any_element()
            }
            (Control::Segmented { options, set, .. }, Shown::Segmented(active)) => {
                let set = *set;
                style::form_segments(
                    id,
                    ui,
                    options.iter().map(|option| (option.icon, text(option.label).into())),
                    active,
                    move |ix, _, cx| update_row(id, cx, |s| set(s, ix)),
                )
                .into_any_element()
            }
            (Control::Stepper { nudge, .. }, Shown::Stepper(value)) => {
                let nudge = *nudge;
                style::stepper(
                    ui,
                    id,
                    value,
                    move |_, _, cx| update_row(id, cx, |s| nudge(s, -1)),
                    move |_, _, cx| update_row(id, cx, |s| nudge(s, 1)),
                )
                .into_any_element()
            }
            (Control::Select { width, options, get, set }, _) => {
                let current = get(settings(cx));
                let choices = options(settings(cx));
                let label = choices
                    .iter()
                    .find(|option| option.value == current)
                    .map_or_else(|| current.clone().into(), |option| option.label().clone());
                if platform::native_menus() {
                    let spoken = settings_index::entry(id).map_or("", |(_, _, entry)| entry.label());
                    let owner = cx.weak_entity();
                    return SelectTrigger::new(
                        SharedString::from(format!("select-{id}-trigger")),
                        ui,
                        label.clone(),
                    )
                    .width(px(*width))
                    .with_menu(spoken, label, move |cx| {
                        if let Some(touched) = setting.touched {
                            owner.update(cx, |this, cx| touched(this, cx)).ok();
                        }
                        select_menu(id, cx).unwrap_or_default()
                    })
                    .into_any_element();
                }
                self.select(id, div().child(label).into_any_element(), *width, choices, current, *set, cx)
            }
            (Control::Recorder(row), _) => self.render_recorder(id, *row, cx),
            (Control::Custom(draw) | Control::Block(draw) | Control::Item(draw), _) => draw(self, cx),
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
                .px_4()
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
                    .on_click(cx.listener(move |this, _, window, cx| this.navigate_to(section, window, cx))),
                );
            let mut rows = Vec::new();
            for hit in &hits[start..end] {
                if let Some(setting) = control(hit.entry.id) {
                    rows.push(self.render_entry(hit.entry, setting, &page, Some(hit), cx));
                }
            }
            column = column
                .child(div().flex().flex_col().gap_2().child(head).child(style::setting_group(ui, rows)));
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

    #[cfg(feature = "devctl")]
    #[gpui_kit::test]
    fn a_stale_row_activation_cannot_write_a_new_project_override(cx: &mut gpui_kit::TestAppContext) {
        use crate::settings::SettingsStore;

        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            let before = cx.global::<SettingsStore>().user_settings().diff_wrap;
            let activate = |cx: &mut gpui_kit::App| {
                super::update_row("diff-wrap", cx, |s| s.diff_wrap = !before);
            };
            let overlay = serde_json::json!({ "diff_wrap": !before });
            SettingsStore::set_project_overrides(cx, overlay.as_object().cloned());
            activate(cx);
            assert_eq!(cx.global::<SettingsStore>().user_settings().diff_wrap, before);
            SettingsStore::set_project_overrides(cx, None);
            activate(cx);
            assert_ne!(cx.global::<SettingsStore>().user_settings().diff_wrap, before);
        });
    }

    #[test]
    fn a_select_reads_back_each_offered_value() {
        for setting in every_setting() {
            if let Control::Select { options, get, set, .. } = &setting.control {
                let defaults = Settings::default();
                let choices = options(&defaults);
                assert!(!choices.is_empty(), "{} has no choices", setting.id);
                assert!(
                    choices.iter().any(|choice| choice.value == get(&defaults)),
                    "{} does not offer its default",
                    setting.id
                );
                for choice in choices {
                    let mut changed = defaults.clone();
                    set(&mut changed, &choice.value);
                    assert_eq!(get(&changed), choice.value, "{} does not read its choice", setting.id);
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
