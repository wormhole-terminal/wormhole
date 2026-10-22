//! The token list: a filter field and All / Changed / Low contrast over
//! the tokens in folder bands, read like the sidebar: a swatch or shape at x 16, the name in
//! the code font at x 40, the value faint on the right.
//!
//! A view of its own, and a virtual list of the rows `token_rows` makes: a wheel tick redraws
//! this view alone and builds the rows in view, not the 107 tokens and their bands. The rows are
//! made again only when something they are made from changes.

use std::collections::BTreeSet;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use style::{BandKind, BandParts, Section};
use wormhole_core::theme::tokens::ThemeTokens;

use super::draft::Show;
use super::token_rows::{Band, Chip, Inputs, ListRow, TokenRow, build};
use super::{StudioWindow, color_chip, contrast, gallery, metric_chip, tokens_of, ui};
use crate::settings::{SettingsStore, UiColors, settings};
use crate::style::{self, Edges, TextRole};
use crate::window::aux_window;

/// The token list's width.
pub(super) const LIST_W: f32 = 300.0;

/// The longest value a row shows before it is cut short (an expression can be long).
const VALUE_MAX_W: f32 = 112.0;

/// Built ahead of the viewport and kept below it, so that a quick scroll doesn't pop rows in.
const OVERDRAW: f32 = 300.0;

/// What the token list did, counted.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Counts {
    pub(super) renders: usize,
    /// How many times the rows were made again.
    pub(super) syncs: usize,
    /// How many rows were built into elements.
    pub(super) rows_built: usize,
}

/// What the rows were made from last: when none of it differs, they are still right.
struct Key {
    tokens: Arc<ThemeTokens>,
    /// The window's count of edits to the draft (`StudioWindow::revision`).
    revision: u64,
    show: Show,
    area: gallery::Area,
    query: String,
    open_groups: BTreeSet<&'static str>,
    selected: Option<String>,
}

impl Key {
    fn is(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.tokens, &other.tokens)
            && self.revision == other.revision
            && self.show == other.show
            && self.area == other.area
            && self.query == other.query
            && self.open_groups == other.open_groups
            && self.selected == other.selected
    }
}

pub(super) struct TokenListView {
    studio: WeakEntity<StudioWindow>,
    /// The filter text.
    pub(super) filter: Entity<InputState>,
    show: Show,
    /// The bands that are open.
    open_groups: BTreeSet<&'static str>,
    state: ListState,
    rows: Vec<ListRow>,
    /// The Changed and Low contrast counts.
    shown_counts: (usize, usize),
    key: Option<Key>,
    /// The tokens that fall short of a contrast minimum, found for these tokens.
    low: Option<(Arc<ThemeTokens>, BTreeSet<String>)>,
    /// The work the view did (the counting tests).
    #[cfg(test)]
    pub(super) counts: Counts,
    _subscriptions: Vec<Subscription>,
}

impl TokenListView {
    pub(super) fn new(studio: &Entity<StudioWindow>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tokens"));
        Self {
            studio: studio.downgrade(),
            _subscriptions: vec![
                cx.observe(&filter, |_, _, cx| cx.notify()),
                // The window says when the selection, the area or the draft changes.
                cx.observe(studio, |_, _, cx| cx.notify()),
                cx.observe_global::<SettingsStore>(|_, cx| cx.notify()),
            ],
            filter,
            show: Show::All,
            open_groups: BTreeSet::new(),
            state: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            rows: Vec::new(),
            shown_counts: (0, 0),
            key: None,
            low: None,
            #[cfg(test)]
            counts: Counts::default(),
        }
    }

    /// How many rows the list has.
    #[cfg(test)]
    pub(super) fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Open `group`'s band (selecting a token in it shows it).
    pub(super) fn open_group(&mut self, group: &'static str, cx: &mut Context<Self>) {
        if self.open_groups.insert(group) {
            cx.notify();
        }
    }

    fn toggle_group(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if !self.open_groups.remove(name) {
            self.open_groups.insert(name);
        }
        cx.notify();
    }

    fn set_show(&mut self, show: Show, cx: &mut Context<Self>) {
        self.show = show;
        cx.notify();
    }

    /// devctl `studio-filter <all|changed|low> [text]`.
    #[cfg(feature = "devctl")]
    pub(super) fn devctl_filter(
        &mut self,
        show: Show,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show = show;
        self.filter.update(cx, |f, cx| f.set_value(text.to_string(), window, cx));
        cx.notify();
    }

    /// Make the rows again if what they are made from changed, telling the list what differs.
    fn sync(&mut self, tokens: &Arc<ThemeTokens>, cx: &App) {
        let Some(studio) = self.studio.upgrade() else { return };
        let studio = studio.read(cx);
        let key = Key {
            tokens: tokens.clone(),
            revision: studio.revision,
            show: self.show,
            area: studio.area,
            query: self.filter.read(cx).value().to_string(),
            open_groups: self.open_groups.clone(),
            selected: studio.selected.clone(),
        };
        if self.key.as_ref().is_some_and(|last| last.is(&key)) {
            return;
        }
        #[cfg(test)]
        {
            self.counts.syncs += 1;
        }
        if !self.low.as_ref().is_some_and(|(of, _)| Arc::ptr_eq(of, tokens)) {
            self.low = Some((tokens.clone(), contrast::low_contrast_tokens(tokens)));
        }
        let low = self.low.as_ref().map(|(_, low)| low).expect("just found");
        let built = build(&Inputs {
            draft: &studio.draft,
            initial: &studio.initial,
            tokens,
            low,
            show: self.show,
            area: key.area.part(),
            area_label: key.area.label(),
            query: &key.query,
            open_groups: &self.open_groups,
            selected: key.selected.as_deref(),
        });
        // Only the lines that came or went: the others keep their measured heights, and the list
        // its place, when a band opens in the middle of it.
        let old = &self.rows;
        let new = &built.rows;
        let head = old.iter().zip(new).take_while(|(a, b)| a.same_line(b)).count();
        let tail = old[head..]
            .iter()
            .rev()
            .zip(new[head..].iter().rev())
            .take_while(|(a, b)| a.same_line(b))
            .count();
        self.state.splice(head..old.len() - tail, new.len() - head - tail);
        self.shown_counts = (built.changed_count, built.low_count);
        self.rows = built.rows;
        self.key = Some(key);
    }

    fn band(&self, band: &Band, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let name = band.name;
        let marks = div()
            .flex_1()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .when(band.low > 0, |d| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .text_caption()
                        .text_color(ui.warning_text)
                        .child(style::icon(IconName::TriangleAlert).text_color(ui.warning))
                        .child(band.low.to_string()),
                )
            })
            .when(band.changed, |d| d.child(style::changed_dot(ui)));
        style::band(
            SharedString::from(format!("studio-group-{name}")),
            ui,
            BandParts {
                kind: BandKind::Section,
                section: if band.open { Section::Open } else { Section::Collapsed },
                title: name.into(),
                detail: Some(marks.into_any_element()),
                count: Some(band.size.to_string().into()),
                waiting: None,
                add: None,
                on_toggle: band.toggles.then(|| -> style::ClickHandler {
                    let this = cx.weak_entity();
                    Box::new(move |_, _, cx| {
                        let _ = this.update(cx, |list, cx| list.toggle_group(name, cx));
                    })
                }),
                drop_target: false,
                sticky: false,
                joined: band.joined,
            },
        )
        .into_any_element()
    }

    fn token_row(&self, row: &TokenRow, code_font: SharedString, ui: UiColors) -> AnyElement {
        let name = row.name.as_str();
        let selected = row.selected;
        let preview = match row.chip {
            Chip::Metric(value) => metric_chip(ui, name, value),
            Chip::Color(color) => color_chip(ui, color),
        };
        // The window's own update, not this view's: selecting opens the token's band here.
        let studio = self.studio.clone();
        let picked = row.name.clone();
        style::flush_row(SharedString::from(format!("token-{name}")), ui, selected)
            .h(px(style::ROW_H))
            .on_click(move |_, window, cx| {
                studio.update(cx, |studio, cx| studio.select_token(&picked, window, cx)).ok();
            })
            .child(style::lead(preview))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_caption()
                            .font_family(code_font.clone())
                            .child(SharedString::from(row.name.clone())),
                    )
                    .when(row.low, |d| d.child(style::icon(IconName::TriangleAlert).text_color(ui.warning)))
                    // On the selected row's tint, the dot takes the tint's text color.
                    .when(row.changed, |d| {
                        d.child(style::changed_dot(ui).when(selected, |d| d.bg(ui.selected_text)))
                    }),
            )
            .child(
                div()
                    .flex_none()
                    .max_w(px(VALUE_MAX_W))
                    .truncate()
                    .text_caption()
                    .font_family(code_font)
                    .text_color(if selected { ui.selected_muted } else { ui.faint })
                    .child(SharedString::from(row.value.clone())),
            )
            .into_any_element()
    }

    fn render_row(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(test)]
        {
            self.counts.rows_built += 1;
        }
        let ui = ui(cx);
        let Some(row) = self.rows.get(ix) else { return div().into_any_element() };
        let line = match row {
            ListRow::NotInArea(note) => div()
                .id("studio-not-in-area")
                .px_4()
                .py_2()
                .flex()
                .items_center()
                .gap_2()
                .text_caption()
                .text_color(ui.muted)
                .child(style::icon(IconName::Info).text_color(ui.faint))
                .child(div().min_w_0().child(note.clone()))
                .into_any_element(),
            ListRow::Band(band) => self.band(band, ui, cx),
            ListRow::Token(token) => {
                let (code_font, _) = crate::fonts::code_font(cx);
                self.token_row(token, code_font, ui)
            }
        };
        // Items are laid out at their content width unless told otherwise.
        div().w_full().when(ix + 1 == self.rows.len(), |d| d.pb_4()).child(line).into_any_element()
    }
}

impl Render for TokenListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        {
            self.counts.renders += 1;
        }
        let ui = ui(cx);
        let panel = aux_window::surfaces(cx, settings(cx).studio_translucent).panel;
        self.sync(&tokens_of(cx), cx);
        let (changed_count, low_count) = self.shown_counts;
        let show = |id: &'static str, label: String, value: Show| {
            style::segment(id, ui, label, self.show == value)
                .on_click(cx.listener(move |this, _, _, cx| this.set_show(value, cx)))
        };
        let header = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .child(
                Input::new(&self.filter)
                    .small()
                    .cleanable(true)
                    .prefix(style::icon(IconName::Search).text_color(ui.muted)),
            )
            .child(
                style::segmented(ui)
                    .child(show("studio-show-all", "All".into(), Show::All))
                    .child(show("studio-show-changed", format!("Changed · {changed_count}"), Show::Changed))
                    .child(show("studio-show-low", format!("Low contrast · {low_count}"), Show::LowContrast)),
            );
        div()
            .id("studio-token-list")
            .size_full()
            .flex()
            .flex_col()
            .rule_right(ui)
            .bg(panel)
            .child(header)
            .child(
                list(self.state.clone(), cx.processor(|this, ix, _window, cx| this.render_row(ix, cx)))
                    .flex_1()
                    .min_h_0(),
            )
    }
}
