//! The settings' controls. What every setting is called, what it says, the words that find it
//! and the `config.json` keys it writes are data in core (`wormhole_core::config::settings_index`,
//! one table for the pages, the search and the command palette); this module gives each of its
//! rows a [`Control`] (how it is drawn, how it reads and writes the [`Settings`]) and draws a
//! page from the two: one renderer for every row (`style::setting`), with its changed, project
//! and search states.
//!
//! A row that is more than a switch, a menu or a recorder (a font select, the
//! theme cards) is a [`Control::Custom`] or [`Control::Block`]: a function of the window.
//! A new setting is a row in the core table plus a [`Setting`] in its section's list here;
//! a test fails while one has no other.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;
use wormhole_core::config::settings_index::{self, Entry, Hit, Query, Values};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

use super::{FrontWindow, SettingsWindow, section_icon, sections, ui};
use crate::select::{self, MenuOptions};
use crate::settings::update_settings;
use crate::settings::{Settings, SettingsStore, UiColors, settings};
use crate::style::{self, RowKeys, SettingParts, SettingState, TextRole};

/// The widest the summary of a block's value grows on the Changed page before it is cut.
const SUMMARY_MAX_W: f32 = 240.0;

/// Draws part of a page that is more than a switch, a menu or a recorder.
pub(super) type Draw = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> AnyElement;
/// Says something more current than the table's description (the font in use).
pub(super) type Describe = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// A tag after a row's label that says something of the moment ("Time Sensitive").
pub(super) type Chip = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<SharedString>;
/// What a row shows under itself, across the column, when it has something to say (the reset's
/// question).
pub(super) type Below = fn(&SettingsWindow, &mut Context<SettingsWindow>) -> Option<AnyElement>;

/// The words of one option of a [`Control::Menu`].
pub(super) enum Label {
    /// A message of the catalogs: a table of options is a `const`, which cannot read the language in
    /// use, so it is looked up as the control is drawn.
    Message(Key),
    /// A message with a whole number in it ("4 spaces"): how to say the number, and the number.
    Counted(fn(u32) -> String, u32),
}

impl Label {
    pub(super) fn words(&self) -> SharedString {
        match self {
            Self::Message(key) => text(*key).into(),
            Self::Counted(say, n) => say(*n).into(),
        }
    }
}

/// One option of a [`Control::Menu`].
pub(super) struct MenuOption {
    pub label: Label,
}

impl MenuOption {
    pub(super) const fn text(label: Key) -> Self {
        Self { label: Label::Message(label) }
    }

    pub(super) const fn counted(say: fn(u32) -> String, n: u32) -> Self {
        Self { label: Label::Counted(say, n) }
    }
}

/// What a menu's setting is now, read from the settings.
pub(super) enum Current {
    /// The option at this place.
    At(usize),
    /// None of the options: a value written in `config.json` by hand. Shown as it is, and kept until
    /// another option is chosen.
    Other(String),
}

/// How a menu reads its setting.
#[derive(Clone, Copy)]
pub(super) enum Read {
    /// Every value of the setting is an option.
    Index(fn(&Settings) -> usize),
    /// The setting is a number or a mix of keys, so a value may be none of the options.
    Value(fn(&Settings) -> Current),
}

pub(super) struct Menu {
    pub options: &'static [MenuOption],
    pub read: Read,
    /// Chooses the option at a place; never called with a place past the options.
    pub set: fn(&mut Settings, usize),
}

impl Menu {
    pub(super) fn current(&self, settings: &Settings) -> Current {
        match self.read {
            Read::Index(get) => Current::At(get(settings).min(self.options.len().saturating_sub(1))),
            Read::Value(get) => get(settings),
        }
    }
}

pub(super) enum Control {
    /// A switch.
    Toggle { get: fn(&Settings) -> bool, set: fn(&mut Settings, bool) },
    /// A few options in a menu that shows the current one.
    Menu(Menu),
    /// A shortcut, recorded by pressing it (empty is none).
    Recorder(super::recorder::RecorderRow),
    /// The control of a row, drawn by the page.
    Custom(Draw),
    /// Content that stands on its own, with no row around it and outside the group's card (the theme
    /// cards, a preview, a notice).
    Block(Draw),
    /// Rows the page draws itself (a list of agents, of Probes): they sit in the group's card as any
    /// row does, and the Changed page and the search show them as one line, as a [`Control::Block`].
    Rows(Draw),
}

impl Control {
    /// Drawn by the page as a whole rather than as a label with a control: the Changed page and the
    /// search have no room for it, so they say what it is set to.
    fn draws_itself(&self) -> bool {
        matches!(self, Self::Block(_) | Self::Rows(_))
    }

    /// Whether it sits in its group's card: every row does, a standing block does not.
    fn in_card(&self) -> bool {
        !matches!(self, Self::Block(_))
    }
}

pub(super) struct Setting {
    /// The core table's id for the row.
    pub id: &'static str,
    pub control: Control,
    pub describe: Option<Describe>,
    pub below: Option<Below>,
    pub chip: Option<Chip>,
    /// The control runs over more than one line (see `SettingParts::tall`).
    pub tall: bool,
    /// What the setting is by default, in the control's own words, where the defaults of its keys don't say
    /// it (the colours: "Theme's colour", not "On"). The Changed page shows it.
    pub default_words: Option<fn() -> SharedString>,
}

impl Setting {
    pub(super) const fn new(id: &'static str, control: Control) -> Self {
        Self { id, control, describe: None, below: None, chip: None, tall: false, default_words: None }
    }

    pub(super) const fn toggle(
        id: &'static str,
        get: fn(&Settings) -> bool,
        set: fn(&mut Settings, bool),
    ) -> Self {
        Self::new(id, Control::Toggle { get, set })
    }

    /// A menu over a setting of which every value is one of the options.
    pub(super) const fn menu(
        id: &'static str,
        options: &'static [MenuOption],
        get: fn(&Settings) -> usize,
        set: fn(&mut Settings, usize),
    ) -> Self {
        Self::new(id, Control::Menu(Menu { options, read: Read::Index(get), set }))
    }

    /// A menu over a setting that may hold a value none of the options is.
    pub(super) const fn menu_of_values(
        id: &'static str,
        options: &'static [MenuOption],
        get: fn(&Settings) -> Current,
        set: fn(&mut Settings, usize),
    ) -> Self {
        Self::new(id, Control::Menu(Menu { options, read: Read::Value(get), set }))
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

    pub(super) const fn rows(id: &'static str, draw: Draw) -> Self {
        Self::new(id, Control::Rows(draw))
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

    /// The default, in words, for the Changed page.
    pub(super) const fn default_words(self, words: fn() -> SharedString) -> Self {
        Self { default_words: Some(words), ..self }
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

/// The ids of the rows drawn as menu controls, for devctl, which opens them by name: the registry's menus
/// and the custom rows that draw one themselves.
#[cfg(feature = "devctl")]
pub(super) fn menu_ids() -> impl Iterator<Item = &'static str> {
    // The agents' "Hooks on" menus, named as `agents/installed.rs` names them.
    const CUSTOM: [&str; 11] = [
        "agent-hooks-Claude",
        "agent-hooks-Codex",
        "agent-hooks-Gemini",
        "density",
        "open-in",
        "default-shell",
        "icon-theme",
        "motion-insert",
        "motion-fold",
        "motion-status",
        "motion-column",
    ];
    controls()
        .values()
        .filter(|(_, setting)| matches!(setting.control, Control::Menu(_)))
        .map(|(_, setting)| setting.id)
        .chain(CUSTOM)
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
}

impl PageState {
    pub(super) fn new(front: &FrontWindow, cx: &App) -> Self {
        let store = cx.global::<SettingsStore>();
        Self {
            ui: ui(cx),
            values: Values::of(store.user_settings()),
            project: ProjectSource::current(front, cx),
        }
    }
}

/// The person's current values of `keys` as JSON, for a row's Copy as JSON. Their own, not what Zen mode or
/// a project overlays: the file is what is copied.
pub(super) fn current_json(cx: &App, keys: &[&str]) -> String {
    Values::of(cx.global::<SettingsStore>().user_settings()).json(keys)
}

/// What a setting shows right now, read before the row is built so no copy of the settings
/// is held while the row borrows the window.
enum Shown {
    Toggle(bool),
    Menu(Current),
    Drawn,
}

fn shown(control: &Control, settings: &Settings) -> Shown {
    match control {
        Control::Toggle { get, .. } => Shown::Toggle(get(settings)),
        Control::Menu(menu) => Shown::Menu(menu.current(settings)),
        Control::Recorder(_) | Control::Custom(_) | Control::Block(_) | Control::Rows(_) => Shown::Drawn,
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
        let mut column = div().flex().flex_col().gap_6().children(super::zen_note::draw(section, cx));
        for (at, group) in section.groups().iter().enumerate() {
            // Each group is where its sub-row in the nav scrolls to: only the picked one carries the anchor.
            let mut rows =
                div().id(("settings-group", at)).flex().flex_col().anchor_scroll(self.group.anchor_of(at));
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
            // The rows of a group share one card; a block that stands on its own (a preview, a notice) is
            // drawn as it is and closes the card, so the rows after it start another.
            let mut card: Vec<AnyElement> = Vec::new();
            let mut shown = 0;
            for entry in group.entries {
                let Some(setting) = self.drawn(entry, settings(cx)) else { continue };
                shown += 1;
                let element = self.render_entry(entry, setting, &page, hits.get(entry.id), None, cx);
                if !setting.control.in_card() {
                    if !card.is_empty() {
                        rows = rows.child(style::group_card(page.ui, std::mem::take(&mut card)));
                    }
                    rows = rows.child(element);
                    continue;
                }
                card.push(element);
                if let Some(below) = setting.below.and_then(|below| below(self, cx)) {
                    card.push(div().pb_2().child(below).into_any_element());
                }
            }
            // A group none of whose rows this build draws (a flag is off) has nothing to show.
            if shown == 0 {
                continue;
            }
            if !card.is_empty() {
                rows = rows.child(style::group_card(page.ui, card));
            }
            // The group its sub-row in the nav picked is where the page scrolls to; it alone has an anchor
            // (and so an id), which costs the others nothing.
            column = column.child(match self.group.anchor_of(at) {
                Some(anchor) => {
                    rows.id(("settings-group", at)).anchor_scroll(Some(anchor)).into_any_element()
                }
                None => rows.into_any_element(),
            });
        }
        column.into_any_element()
    }

    /// The control of `entry` if the page draws its row: shown for these settings (not behind a feature
    /// flag that is off).
    pub(super) fn drawn(&self, entry: &'static Entry, settings: &Settings) -> Option<&'static Setting> {
        if !entry.is_shown(settings) {
            return None;
        }
        let setting = control(entry.id);
        if setting.is_none() {
            tracing::warn!("settings: no control for {}", entry.id);
        }
        setting
    }

    /// Whether the page draws a row of `group`. The nav lists a group under its page only if so, so a
    /// sub-row never leads to nothing.
    pub(super) fn group_drawn(&self, group: &'static settings_index::Group, settings: &Settings) -> bool {
        group.entries.iter().any(|entry| self.drawn(entry, settings).is_some())
    }

    /// One setting: a block as it draws itself, anything else in a `style::setting` row. `default` is the
    /// Changed page's: the line under the label that says what the setting is by default (it takes the
    /// description's place, which the setting's own page keeps), and a block, which draws a whole page's
    /// worth, is a row there that says what it is set to.
    pub(super) fn render_entry(
        &self,
        entry: &'static Entry,
        setting: &'static Setting,
        page: &PageState,
        hit: Option<&Hit>,
        default: Option<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = page.ui;
        let block = setting.control.draws_itself();
        if block && default.is_none() {
            let (Control::Block(draw) | Control::Rows(draw)) = &setting.control else {
                return div().into_any_element();
            };
            let content = draw(self, cx);
            // A standing block (the theme cards) has no row to carry its keys, so it carries the menu.
            if matches!(setting.control, Control::Block(_)) && !entry.keys.is_empty() {
                let keys = entry.keys;
                let on_reset = page.values.entry_changed(entry).then(|| -> style::Reset {
                    Rc::new(move |_, cx| update_settings(cx, |s| *s = settings_index::reset(s, keys)))
                });
                return style::keyed_block(
                    SharedString::from(format!("setting-{}", entry.id)),
                    RowKeys { keys, json: current_json },
                    on_reset,
                    content,
                )
                .into_any_element();
            }
            return content;
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
            None if default.is_some() => default.clone(),
            None => setting
                .describe
                .and_then(|describe| describe(self, cx))
                .or_else(|| (!entry.description().is_empty()).then(|| entry.description().into())),
        };
        // A description of the moment is not the table's: its search marks don't fit it.
        let found_description = hit
            .map(|hit| hit.description_ranges.clone())
            .filter(|_| setting.describe.is_none() && project.is_none() && default.is_none())
            .unwrap_or_default();
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
        let control = if block {
            // Only the Changed page gets here with a block: what it is set to, in one muted line.
            div()
                .max_w(px(SUMMARY_MAX_W))
                .truncate()
                .text_caption()
                .text_color(ui.muted)
                .child(current_json(cx, entry.keys))
                .into_any_element()
        } else {
            self.render_control(setting, entry.label(), current, ui, cx)
        };
        // The row the palette asked for (`reveal_row`): marked as a search hit on its whole
        // label, and anchored so the page can scroll to it.
        let revealed = self.revealed_row().filter(|(id, _)| *id == entry.id).map(|(_, anchor)| anchor);
        let found = match hit.map(|hit| hit.label_ranges.clone()).filter(|ranges| !ranges.is_empty()) {
            Some(ranges) => ranges,
            None if revealed.is_some() => std::iter::once(0..entry.label().len()).collect(),
            None => Vec::new(),
        };
        style::setting(
            SharedString::from(format!("setting-{}", entry.id)),
            ui,
            SettingParts {
                label: entry.label().into(),
                description,
                visible_hint: (entry.visible_hint || default.is_some()) && project.is_none(),
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
                    Rc::new(move |_, cx| update_settings(cx, |s| *s = settings_index::reset(s, keys)))
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
        label: &'static str,
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
            (Control::Menu(menu), Shown::Menu(current)) => menu_control(id, label, menu, current, ui),
            (Control::Recorder(row), _) => self.render_recorder(id, *row, cx),
            (Control::Custom(draw) | Control::Block(draw) | Control::Rows(draw), _) => draw(self, cx),
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
            let mut rows = Vec::new();
            for hit in &hits[start..end] {
                if let Some(setting) = control(hit.entry.id) {
                    rows.push(self.render_entry(hit.entry, setting, &page, Some(hit), None, cx));
                }
            }
            column = column.child(div().flex().flex_col().child(head).child(style::group_card(ui, rows)));
            start = end;
        }
        column.into_any_element()
    }
}

/// The name of the row `id` in the table, for a menu control's VoiceOver label.
pub(super) fn row_label(id: &str) -> &'static str {
    settings_index::entry(id).map_or("", |(_, _, entry)| entry.label())
}

/// A row's menu: the current value in its face. A value no option names (set by hand in
/// `config.json`) is shown as it is and listed last, checked, so it stays until another is chosen.
fn menu_control(
    id: &'static str,
    label: &'static str,
    menu: &'static Menu,
    current: Current,
    ui: UiColors,
) -> AnyElement {
    let options = menu.options;
    let set = menu.set;
    let (value, other, at) = match current {
        Current::At(at) => (options[at].label.words(), None, at),
        Current::Other(words) => {
            let words = SharedString::from(words);
            (words.clone(), Some(words), options.len())
        }
    };
    select::menu_select(id, ui, label, value, move |_| {
        let words = options.iter().map(|option| option.label.words()).chain(other);
        MenuOptions::words(words, Some(at), move |ix, _, cx| {
            if ix < options.len() {
                update_settings(cx, |s| set(s, ix));
            }
        })
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use wormhole_core::config::Settings;
    use wormhole_core::config::settings_index::{self, SectionId};

    use super::{Control, Menu, Read, controls, sections};
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
    fn a_menu_reads_back_every_option() {
        for setting in every_setting() {
            if let Control::Menu(Menu { options, read: Read::Index(get), set }) = &setting.control {
                assert!(options.len() > 1, "{} has nothing to choose", setting.id);
                for ix in 0..options.len() {
                    let mut s = Settings::default();
                    set(&mut s, ix);
                    assert_eq!(get(&s), ix, "{} option {ix}", setting.id);
                }
            }
        }
    }
}
