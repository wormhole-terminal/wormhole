//! The font picker: the current font as a select, drawn in its own face with a
//! "Bundled" tag (or what an empty choice follows, faint), and the size stepper beside it.
//! Its menu searches the bundled, custom and installed families, each in its own face,
//! Monospace | All for the terminal and code fonts; the font under the pointer shows in the
//! page's specimen. Used for the terminal, interface and code fonts.

use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::popover::Popover;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::fonts::{FontSlot, FontSource, SYSTEM_FONT, display_name, grouped_font_choices};

use super::{SettingsWindow, ui};
use crate::fonts::{FontCatalog, LoadedFonts};
use crate::select::{self, SelectTrigger};
use crate::settings::update_settings;
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
    /// The family under the pointer in the open menu, which the specimen shows.
    pub(super) hovered: Option<String>,
}

impl FontPickerState {
    pub(super) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        Self {
            open: None,
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search fonts")),
            show_all: false,
            scroll: UniformListScrollHandle::new(),
            hovered: None,
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

impl SettingsWindow {
    /// Expand `slot`'s picker (collapsing any other), or collapse it.
    pub(super) fn toggle_font_picker(&mut self, slot: FontSlot, window: &mut Window, cx: &mut Context<Self>) {
        let picker = &mut self.font_picker;
        if picker.open == Some(slot) {
            picker.open = None;
        } else {
            FontCatalog::ensure_loaded(cx);
            picker.open = Some(slot);
            picker.show_all = false;
            picker.hovered = None;
            // The specimen shows the font being picked.
            self.specimen = slot;
            picker.search.update(cx, |input, cx| input.set_value("", window, cx));
            let focus = picker.search.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
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
                            "SF Pro".to_string()
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
                .when(bundled, |d| d.child(style::tint_chip(ui, None, "Bundled"))),
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
                    .width(px(SELECT_W)),
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
            format!("{size:.0} pt"),
            cx.listener(move |_, _, _, cx| update_settings(cx, |s| s.step_slot_size(slot, -1.0))),
            cx.listener(move |_, _, _, cx| update_settings(cx, |s| s.step_slot_size(slot, 1.0))),
        );
        div().flex().items_center().gap_2().child(picker).child(size_control).into_any_element()
    }
}

/// What the open menu draws from, captured for the popover's render callback.
struct FontMenu {
    slot: FontSlot,
    entries: Rc<Vec<Entry>>,
    selected: String,
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
            "Loading installed fonts…".to_string()
        } else if slot.prefers_monospace() && !show_all && classifying {
            "Finding monospace fonts…".to_string()
        } else {
            let fonts = entries.iter().filter(|e| matches!(e, Entry::Family(_))).count();
            format!("{fonts} font{}", if fonts == 1 { "" } else { "s" })
        };
        Self {
            slot,
            entries,
            selected: settings(cx).slot_family(slot).to_string(),
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
        let (mono, symbolic, this) = (self.mono.clone(), self.symbolic.clone(), self.this.clone());
        let show_all_this = self.this.clone();

        let list = uniform_list(
            SharedString::from(format!("font-list-{}", slot.key())),
            entries.len(),
            move |range: std::ops::Range<usize>, _, _| {
                range
                    .map(|ix| {
                        entry_row(slot, ix, &entries[ix], &selected, tag_mono, &mono, &symbolic, ui, &this)
                    })
                    .collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.scroll)
        // Shrinks to a short search result instead of leaving a gap.
        .h(px((self.entries.len().max(1) as f32 * ENTRY_HEIGHT).min(LIST_HEIGHT)));

        div()
            .flex()
            .flex_col()
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
                                .child(segment(0, "Monospace", false))
                                .child(segment(1, "All", true)),
                        )
                        .child(div().text_caption().text_color(ui.faint).child(self.status.clone())),
                )
            })
            .child(div().flex().flex_col().p_1().child(list).when(!slot.prefers_monospace(), |d| {
                d.child(div().px_2().py_1().text_caption().text_color(ui.muted).child(self.status.clone()))
            }))
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
    let page = page.clone();
    let hover_page = page.clone();
    let hovered = family.clone();
    select::option(("font-entry", ix), ui, active, false)
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
            let value = value.clone();
            page.update(cx, |this, cx| {
                this.font_picker.open = None;
                cx.notify();
            })
            .ok();
            update_settings(cx, |s| s.set_slot_family(slot, value));
        })
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_heading()
                .when_some(own_face, |d, face| d.font_family(face))
                .when(family.is_none(), |d| d.text_color(ui.muted))
                .child(label),
        )
        .when(tag_mono && is_mono, |d| {
            d.child(div().flex_none().text_caption().text_color(ui.faint).child("monospace"))
        })
        .into_any_element()
}
