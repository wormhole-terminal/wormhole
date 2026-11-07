//! The font picker: the current font as a select, drawn in its own face with a
//! "Bundled" tag (or what an empty choice follows, faint), and the size stepper beside it.
//! Its menu searches the bundled, custom and installed families, each in its own face,
//! Monospace | All for the terminal and code fonts; the font under the pointer shows in the
//! page's specimen. Used for the terminal, interface and code fonts.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Enter, Input, InputState, MoveDown, MoveUp};
use gpui_kit::component::popover::Popover;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::fonts::{FontSlot, FontSource, SYSTEM_FONT, display_name, grouped_font_choices};

use super::registry::{project_sets, update_row};
use super::{SettingsWindow, ui};
use crate::fonts::{FontCatalog, LoadedFonts};
use crate::select::{self, SelectTrigger};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};

const LIST_HEIGHT: f32 = 264.0;
const ENTRY_HEIGHT: f32 = 32.0;
/// The select's width, and its menu's.
const SELECT_W: f32 = 272.0;
const MENU_W: f32 = 340.0;

/// The expanded picker (only one at a time) and its search box.
pub(super) struct FontPickerState {
    pub(super) open: Option<FontSlot>,
    pub(super) search: Entity<InputState>,
    /// Terminal/code pickers: list proportional fonts too.
    show_all: bool,
    scroll: UniformListScrollHandle,
    /// The family under the pointer or keyboard highlight, shown in the specimen.
    pub(super) hovered: Option<String>,
    keyboard_choice: Option<String>,
}

impl FontPickerState {
    pub(super) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        Self {
            open: None,
            search: cx.new(|cx| InputState::new(window, cx).placeholder(text(SETTINGS_FONTS_SEARCH_FONTS))),
            show_all: false,
            scroll: UniformListScrollHandle::new(),
            hovered: None,
            keyboard_choice: None,
        }
    }

    pub(super) fn clear_keyboard_choice(&mut self) {
        if self.keyboard_choice.take().is_some() {
            self.hovered = None;
        }
    }
}

#[derive(Clone)]
enum Entry {
    /// The empty family: system font (interface) or terminal font (code).
    Inherit(&'static str),
    /// The caption over a group of families (bundled, custom, installed).
    Group(FontSource),
    Family(String),
}

impl Entry {
    /// A selectable family's stored value and spoken label. Group headings have neither.
    fn choice(&self) -> Option<(&str, &str)> {
        match self {
            Self::Inherit(label) => Some(("", label)),
            Self::Family(name) => Some((name, name)),
            Self::Group(_) => None,
        }
    }
}

fn font_row(slot: FontSlot) -> &'static str {
    match slot {
        FontSlot::Terminal => "font-terminal",
        FontSlot::Interface => "font-interface",
        FontSlot::Code => "font-code",
    }
}

impl SettingsWindow {
    /// Expand `slot`'s picker (collapsing any other), or collapse it.
    pub(super) fn toggle_font_picker(&mut self, slot: FontSlot, window: &mut Window, cx: &mut Context<Self>) {
        let picker = &mut self.font_picker;
        if picker.open == Some(slot) {
            picker.open = None;
        } else if !project_sets(font_row(slot), cx) {
            FontCatalog::ensure_loaded(cx);
            picker.open = Some(slot);
            picker.show_all = false;
            picker.hovered = None;
            picker.keyboard_choice = None;
            // The specimen shows the font being picked.
            self.specimen = slot;
            picker.search.update(cx, |input, cx| input.set_value("", window, cx));
            if let Some(ix) = self.entries(slot, cx).iter().position(|e| self.is_selected(slot, e, cx)) {
                self.font_picker.scroll.scroll_to_item(ix, ScrollStrategy::Center);
            }
        }
        cx.notify();
    }

    fn entries(&self, slot: FontSlot, cx: &App) -> Vec<Entry> {
        let query = self.font_picker.search.read(cx).value().to_string();
        let catalog = cx.try_global::<FontCatalog>();
        let names = catalog.map(|c| c.names.clone()).unwrap_or_default();
        let mono = catalog.and_then(|c| c.monospace.clone());
        let mono_only = slot.prefers_monospace() && !self.font_picker.show_all;
        let loaded = cx.try_global::<LoadedFonts>();
        let bundled = loaded.map(|l| l.bundled.clone()).unwrap_or_default();
        let custom = loaded.map(LoadedFonts::custom_families).unwrap_or_default();
        let choices = grouped_font_choices(&names, &bundled, &custom, mono.as_deref(), &query, mono_only);
        // Captions only say something once the bundled or custom fonts are listed.
        let captions = choices.iter().any(|(source, _)| *source != FontSource::Installed);

        let mut entries: Vec<Entry> = slot
            .inherit_label()
            .filter(|label| label.to_lowercase().contains(query.trim().to_lowercase().as_str()))
            .map(Entry::Inherit)
            .into_iter()
            .collect();
        let mut group = None;
        for (source, name) in choices {
            if captions && group != Some(source) {
                group = Some(source);
                entries.push(Entry::Group(source));
            }
            entries.push(Entry::Family(name));
        }
        entries
    }

    fn is_selected(&self, slot: FontSlot, entry: &Entry, cx: &App) -> bool {
        let family = settings(cx).slot_family(slot);
        match entry {
            Entry::Inherit(_) => family.trim().is_empty(),
            Entry::Group(_) => false,
            Entry::Family(name) => name == family,
        }
    }

    fn font_choice_anchor(&self, slot: FontSlot, entries: &[Entry], cx: &App) -> Option<usize> {
        let search_empty = self.font_picker.search.read(cx).value().trim().is_empty();
        let value = self
            .font_picker
            .keyboard_choice
            .as_deref()
            .or_else(|| search_empty.then(|| settings(cx).slot_family(slot)))?;
        entries.iter().position(|entry| entry.choice().is_some_and(|(choice, _)| choice == value))
    }

    fn move_font_choice(&mut self, slot: FontSlot, entries: &[Entry], down: bool, cx: &mut Context<Self>) {
        if self.font_picker.open != Some(slot) {
            return;
        }
        let anchor = self.font_choice_anchor(slot, entries, cx);
        let rows = entries.iter().enumerate();
        let next = if down {
            rows.skip(anchor.map_or(0, |ix| ix + 1))
                .find_map(|(ix, entry)| entry.choice().map(|choice| (ix, choice)))
        } else {
            rows.take(anchor.unwrap_or(entries.len()))
                .rev()
                .find_map(|(ix, entry)| entry.choice().map(|choice| (ix, choice)))
        };
        if let Some((ix, (value, label))) = next {
            self.font_picker.keyboard_choice = Some(value.to_owned());
            self.font_picker.hovered = (!value.is_empty()).then(|| value.to_owned());
            self.font_picker.scroll.scroll_to_item(ix, ScrollStrategy::Center);
            crate::platform::a11y::announce(label, false);
            cx.notify();
        }
    }

    fn choose_font(&mut self, slot: FontSlot, value: &str, cx: &mut Context<Self>) {
        if self.font_picker.open == Some(slot) {
            self.font_picker.open = None;
            update_row(font_row(slot), cx, |s| s.set_slot_family(slot, value.to_owned()));
            cx.notify();
        }
    }

    /// A font row's control: the font as a select (its menu floats under it) and its size.
    pub(super) fn font_control(&self, slot: FontSlot, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let store = cx.global::<SettingsStore>();
        let family = store.fonts.family(slot).to_string();
        let size = store.settings.slot_size(slot);
        let chosen = !store.settings.slot_family(slot).trim().is_empty();
        let open = self.font_picker.open == Some(slot);
        let bundled = cx.try_global::<LoadedFonts>().is_some_and(|loaded| loaded.bundled.contains(&family));
        let key = slot.key();
        let spoken = text(match slot {
            FontSlot::Terminal => SETTINGS_APPEARANCE_GROUP_TERMINAL,
            FontSlot::Interface => SETTINGS_APPEARANCE_GROUP_INTERFACE,
            FontSlot::Code => SETTINGS_APPEARANCE_GROUP_CODE,
        });
        let spoken_value = slot
            .inherit_label()
            .filter(|_| !chosen)
            .map_or_else(|| display_name(&family).to_owned(), str::to_owned);
        // The face: the family in its own face and where it comes from, or what an empty
        // choice follows ("Same as terminal", then the terminal font, faint).
        let face = match slot.inherit_label().filter(|_| !chosen) {
            Some(label) => {
                div().flex().items_center().gap_2().min_w_0().child(div().flex_none().child(label)).child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_right()
                        .text_caption()
                        .text_color(ui.faint)
                        .child(if family == SYSTEM_FONT {
                            "SF Pro".to_string() // text: name (the system font's own name)
                        } else {
                            display_name(&family).to_string()
                        }),
                )
            }
            None => div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_family(family.clone())
                        .child(display_name(&family).to_string()),
                )
                .when(bundled, |d| d.child(style::tint_chip(ui, None, text(SETTINGS_FONTS_BUNDLED)))),
        };

        let this = cx.weak_entity();
        let on_change = this.clone();
        // Only the open picker needs its (searched) entries.
        let menu = open.then(|| FontMenu::new(self, slot, this, cx));
        let popover = Popover::new(SharedString::from(format!("font-popover-{key}")))
            .anchor(Anchor::TopRight)
            .offset(px(6.0))
            .open(open)
            // Typing goes straight into the search box when the menu opens.
            .track_focus(&self.font_picker.search.read(cx).focus_handle(cx))
            .on_open_change(move |want_open, window, cx| {
                on_change
                    .update(cx, |this, cx| {
                        if (this.font_picker.open == Some(slot)) != *want_open {
                            this.toggle_font_picker(slot, window, cx);
                        }
                    })
                    .ok();
            })
            .trigger(
                SelectTrigger::new(SharedString::from(format!("font-picker-{key}")), ui, face.w_full())
                    .width(px(SELECT_W))
                    .accessibility(spoken, spoken_value)
                    .focusable_popup(),
            );
        let picker = select::menu_surface(popover, ui, px(MENU_W), select::menu_bg(ui, cx))
            .content(move |_, _, _| div().children(menu.as_ref().map(|menu| menu.render(ui))));
        let size_control = style::stepper(
            ui,
            match slot {
                FontSlot::Terminal => "font-size-terminal",
                FontSlot::Interface => "font-size-interface",
                FontSlot::Code => "font-size-code",
            },
            settings_fonts_size_points(&format!("{size:.0}")),
            cx.listener(move |_, _, _, cx| update_row(font_row(slot), cx, |s| s.step_slot_size(slot, -1.0))),
            cx.listener(move |_, _, _, cx| update_row(font_row(slot), cx, |s| s.step_slot_size(slot, 1.0))),
        );
        div().flex().items_center().gap_2().child(picker).child(size_control).into_any_element()
    }
}

/// What the open menu draws from, captured for the popover's render callback.
struct FontMenu {
    slot: FontSlot,
    entries: Rc<Vec<Entry>>,
    selected: String,
    keyboard_choice: Option<String>,
    /// Tag monospace families (when the list isn't already monospace only).
    tag_mono: bool,
    show_all: bool,
    mono: Arc<HashSet<String>>,
    symbolic: Arc<HashSet<String>>,
    status: String,
    search: Entity<InputState>,
    scroll: UniformListScrollHandle,
    this: WeakEntity<SettingsWindow>,
}

impl FontMenu {
    fn new(
        page: &SettingsWindow,
        slot: FontSlot,
        this: WeakEntity<SettingsWindow>,
        cx: &mut Context<SettingsWindow>,
    ) -> Self {
        let entries = Rc::new(page.entries(slot, cx));
        let catalog = cx.try_global::<FontCatalog>();
        let loading = catalog.is_none_or(|c| c.names.is_empty());
        let classifying = catalog.is_none_or(|c| c.monospace.is_none());
        let mono = catalog.and_then(|c| c.monospace.clone()).unwrap_or_default();
        let symbolic = catalog.map(|c| c.symbolic.clone()).unwrap_or_default();
        let show_all = page.font_picker.show_all;

        let status = if loading {
            text(SETTINGS_FONTS_LOADING_INSTALLED_FONTS).to_string()
        } else if slot.prefers_monospace() && !show_all && classifying {
            text(SETTINGS_FONTS_FINDING_MONOSPACE_FONTS).to_string()
        } else {
            let fonts = entries.iter().filter(|e| matches!(e, Entry::Family(_))).count();
            settings_fonts_count(whole(fonts))
        };
        Self {
            slot,
            entries,
            selected: settings(cx).slot_family(slot).to_string(),
            keyboard_choice: page.font_picker.keyboard_choice.clone(),
            tag_mono: !slot.prefers_monospace() || show_all,
            show_all,
            mono,
            symbolic,
            status,
            search: page.font_picker.search.clone(),
            scroll: page.font_picker.scroll.clone(),
            this,
        }
    }

    fn render(&self, ui: UiColors) -> AnyElement {
        let slot = self.slot;
        let entries = self.entries.clone();
        let (selected, tag_mono) = (self.selected.clone(), self.tag_mono);
        let keyboard_choice = self.keyboard_choice.clone();
        let (mono, symbolic, this) = (self.mono.clone(), self.symbolic.clone(), self.this.clone());
        let show_all_this = self.this.clone();

        let list = uniform_list(
            SharedString::from(format!("font-list-{}", slot.key())),
            entries.len(),
            move |range: std::ops::Range<usize>, _, _| {
                range
                    .map(|ix| {
                        entry_row(
                            slot,
                            ix,
                            &entries[ix],
                            &selected,
                            keyboard_choice.as_deref(),
                            tag_mono,
                            &mono,
                            &symbolic,
                            ui,
                            &this,
                        )
                    })
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.scroll)
        // Shrinks to a short search result instead of leaving a gap.
        .h(px((self.entries.len().max(1) as f32 * ENTRY_HEIGHT).min(LIST_HEIGHT)));

        let (page, choices) = (self.this.clone(), self.entries.clone());
        let step = Rc::new(move |down: bool, cx: &mut App| {
            page.update(cx, |this, cx| this.move_font_choice(slot, &choices, down, cx)).ok();
        });
        let (up, down) = (step.clone(), step);
        let (page, choices) = (self.this.clone(), self.entries.clone());
        div()
            .id("font-menu")
            .flex()
            .flex_col()
            .capture_action(move |_: &MoveUp, _, cx| {
                cx.stop_propagation();
                up(false, cx);
            })
            .capture_action(move |_: &MoveDown, _, cx| {
                cx.stop_propagation();
                down(true, cx);
            })
            .capture_action(move |action: &Enter, _, cx| {
                if !action.shift && !action.secondary {
                    cx.stop_propagation();
                    page.update(cx, |this, cx| {
                        let choice = this
                            .font_choice_anchor(slot, &choices, cx)
                            .and_then(|ix| choices[ix].choice())
                            .or_else(|| choices.iter().find_map(Entry::choice));
                        if let Some((value, _)) = choice {
                            this.choose_font(slot, value, cx);
                        }
                    })
                    .ok();
                }
            })
            .child(select::search_row(
                ui,
                false,
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().child(Input::new(&self.search).small().appearance(false))),
            ))
            .when(slot.prefers_monospace(), |d| {
                let segment = |ix: usize, label: &'static str, all: bool| {
                    let this = show_all_this.clone();
                    style::segment(("font-show", ix), ui, label, self.show_all == all).on_click(
                        move |_, _, cx| {
                            this.update(cx, |this, cx| {
                                this.font_picker.show_all = all;
                                this.font_picker.clear_keyboard_choice();
                                cx.notify();
                            })
                            .ok();
                        },
                    )
                };
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_1()
                        .rule_bottom(ui)
                        .child(
                            style::segmented(ui)
                                .child(segment(0, text(SETTINGS_FONTS_MONOSPACE), false))
                                .child(segment(1, text(SETTINGS_FONTS_ALL), true)),
                        )
                        .child(div().text_caption().text_color(ui.faint).child(self.status.clone())),
                )
            })
            .child(div().id("font-options").role(Role::ListBox).flex().flex_col().p_1().child(list).when(
                !slot.prefers_monospace(),
                |d| {
                    d.child(
                        div().px_2().py_1().text_caption().text_color(ui.muted).child(self.status.clone()),
                    )
                },
            ))
            .into_any_element()
    }
}

/// One family in the list, drawn in its own face (symbol fonts in the
/// interface font, since their glyphs can't spell the name).
#[allow(clippy::too_many_arguments)]
fn entry_row(
    slot: FontSlot,
    ix: usize,
    entry: &Entry,
    selected: &str,
    keyboard_choice: Option<&str>,
    tag_mono: bool,
    mono: &Arc<HashSet<String>>,
    symbolic: &Arc<HashSet<String>>,
    ui: UiColors,
    page: &WeakEntity<SettingsWindow>,
) -> AnyElement {
    let (label, family, active) = match entry {
        Entry::Inherit(label) => (label.to_string(), None, selected.trim().is_empty()),
        Entry::Group(source) => {
            // As tall as a family row: the list is uniform.
            return div()
                .id(("font-group", ix))
                .h(px(ENTRY_HEIGHT))
                .flex()
                .items_end()
                .px_2()
                .pb_1()
                .child(style::section_label(ui, source.label()))
                .into_any_element();
        }
        Entry::Family(name) => (name.clone(), Some(name.clone()), name == selected),
    };
    let own_face = family.as_ref().filter(|name| !symbolic.contains(*name)).cloned();
    let is_mono = family.as_ref().is_some_and(|name| mono.contains(name));
    let value = family.clone().unwrap_or_default();
    let highlighted = keyboard_choice == Some(value.as_str());
    let page = page.clone();
    let hover_page = page.clone();
    let hovered = family.clone();
    select::option(("font-entry", ix), ui, active, false)
        .role(Role::ListBoxOption)
        .aria_label(label.clone())
        .aria_selected(active)
        .when(highlighted, |d| d.bg(ui.list_selected).text_color(ui.list_selected_text))
        .w_full()
        .h(px(ENTRY_HEIGHT))
        .on_hover(move |over, _, cx| {
            let hovered = hovered.clone().filter(|_| *over);
            hover_page
                .update(cx, |this, cx| {
                    if *over || this.font_picker.hovered == hovered {
                        this.font_picker.hovered = hovered;
                        cx.notify();
                    }
                })
                .ok();
        })
        .on_click(move |_, _, cx| {
            page.update(cx, |this, cx| this.choose_font(slot, &value, cx)).ok();
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_heading()
                .when_some(own_face, |d, face| d.font_family(face))
                .when(family.is_none() && !highlighted, |d| d.text_color(ui.muted))
                .child(label),
        )
        .when(tag_mono && is_mono, |d| {
            let color = if highlighted { ui.list_selected_muted } else { ui.faint };
            d.child(div().flex_none().text_caption().text_color(color).child("monospace"))
        })
        .into_any_element()
}
