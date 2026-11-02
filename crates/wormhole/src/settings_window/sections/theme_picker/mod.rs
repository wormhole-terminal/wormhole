//! The theme picker, as the design draws it: one block in the Theme group holding the sets of theme cards in
//! four columns, and under them the links and the Import and Studio buttons (`ThemesBlock`, `ThemeSet` and
//! `ThemeCard`, compiled from the design's markup). "Choose" says which sets: "One theme for both" shows
//! dark and light pairs on one card each; "Dark and light separately" shows a set for Dark Mode and one for
//! Light Mode. The pick of a set has the primary's ring and a check under it. Until "Show all" a set shows
//! its first cards, the pick always among them. A hidden theme (`hidden_themes`) is on no set unless it is
//! a pick; the link "N hidden" lists them, each with Show.
//!
//! Each card is a view of its own, drawn from a fixed-size box, so the settings window can leave the ~40 of
//! them alone while it redraws for a scroll tick: only a card whose selection or look changed is told to draw
//! again.

use std::cell::RefCell;
use std::rc::Rc;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use wormhole_core::theme::catalog::{self, ThemeEntry};
use wormhole_core::theme::pairs::{self, ThemePair};
use wormhole_core::theme::tokens::Appearance;

mod card;

#[cfg(test)]
use self::card::CARD_RENDERS;
use self::card::ThemeCard;
use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::{SettingsStore, UiColors, settings, update_settings};
use crate::settings_window::sections::import_theme;
use crate::settings_window::{SettingsWindow, ui};
use crate::studio;
use crate::style;
use crate::ui::{Handler, Look, ThemeSetProps, ThemesBlockProps};

/// How many cards show before "Show all": two rows of four for the pairs, one row for each kind.
const FIRST_CARDS: usize = 8;

/// The height of a card: the thumbnail, the gap and the name's line (the design's `ThemeCard`).
fn card_height() -> Pixels {
    px(style::THEME_CARD_H)
}

/// The picker's own state inside the settings window.
#[derive(Default)]
pub(crate) struct ThemePicker {
    cards: RefCell<CardSet>,
    /// The list of hidden themes is open.
    hidden_open: bool,
}

/// The two picks: the theme of Dark Mode (`theme`) and of Light Mode (`theme_light`).
struct Picks<'a> {
    dark: &'a str,
    light: &'a str,
}

impl Picks<'_> {
    /// Whether `entry` is the pick of its own kind. A theme set as the pick of the other kind (a
    /// light `theme` chosen before there was a light pick) is no set's pick.
    fn picked(&self, entry: &ThemeEntry) -> bool {
        match entry.tokens.appearance {
            Appearance::Dark => entry.is_selected_by(self.dark),
            Appearance::Light => entry.is_selected_by(self.light),
        }
    }

    /// Whether every half of `pair` is a pick.
    fn picked_pair(&self, pair: &ThemePair, entries: &[ThemeEntry]) -> bool {
        pair.halves().all(|ix| self.picked(&entries[ix]))
    }
}

/// Whether the two picks are the halves of one pair: then "One theme for both" is what the person chose,
/// until they say otherwise.
pub(crate) fn picks_are_a_pair(cx: &mut App) -> bool {
    let entries = entries(cx);
    let (dark, light) = {
        let s = settings(cx);
        (s.theme.clone(), s.theme_light.clone())
    };
    let picks = Picks { dark: &dark, light: &light };
    pairs::pairs(&entries).iter().any(|pair| pair.is_whole() && picks.picked_pair(pair, &entries))
}

/// What a card's strip says: a family's name when the family has one version of the card's kind
/// (its set then holds only that one), else the version's own name ("Soltari Dim").
fn labels(entries: &[ThemeEntry]) -> Vec<SharedString> {
    let mut labels: Vec<SharedString> = entries.iter().map(|e| SharedString::from(e.label.clone())).collect();
    for family in catalog::cards(entries) {
        for &ix in &family {
            let kind = entries[ix].tokens.appearance;
            let alone = family.iter().filter(|&&other| entries[other].tokens.appearance == kind).count() == 1;
            if alone {
                labels[ix] = entries[ix].card_label().to_string().into();
            }
        }
    }
    labels
}

/// The cards of a catalog, kept while it is the same: one per pair and one per theme.
#[derive(Default)]
struct CardSet {
    catalog: Option<Rc<Vec<ThemeEntry>>>,
    pairs: Vec<ThemePair>,
    pair_cards: Vec<Entity<ThemeCard>>,
    pair_picked: Vec<bool>,
    cards: Vec<Entity<ThemeCard>>,
    /// Whether each card is its kind's pick, as last given.
    picked: Vec<bool>,
    /// What each card's strip says, for the list of hidden themes too.
    names: Vec<SharedString>,
    /// The theme revision the cards were last given.
    look: Option<u64>,
}

impl CardSet {
    /// Make the cards match `entries`, and tell the ones whose pick or look changed.
    fn sync<T: 'static>(
        &mut self,
        entries: &Rc<Vec<ThemeEntry>>,
        picks: &Picks,
        theme: u64,
        ui: UiColors,
        cx: &mut Context<T>,
    ) {
        if !self.catalog.as_ref().is_some_and(|catalog| Rc::ptr_eq(catalog, entries)) {
            self.names = labels(entries);
            self.picked = entries.iter().map(|entry| picks.picked(entry)).collect();
            self.cards = self
                .names
                .iter()
                .zip(&self.picked)
                .enumerate()
                .map(|(entry, (label, &picked))| {
                    let catalog = entries.clone();
                    let label = label.clone();
                    cx.new(|_| ThemeCard { catalog, halves: vec![entry], label, picked, ui })
                })
                .collect();
            self.pairs = pairs::pairs(entries);
            self.pair_picked = self.pairs.iter().map(|pair| picks.picked_pair(pair, entries)).collect();
            self.pair_cards = self
                .pairs
                .iter()
                .zip(&self.pair_picked)
                .map(|(pair, &picked)| {
                    let catalog = entries.clone();
                    let (halves, label) = (pair.halves().collect(), SharedString::from(pair.name.clone()));
                    cx.new(|_| ThemeCard { catalog, halves, label, picked, ui })
                })
                .collect();
            self.catalog = Some(entries.clone());
            self.look = Some(theme);
            return;
        }
        let look_changed = self.look != Some(theme);
        for ((card, entry), old) in self.cards.iter().zip(entries.iter()).zip(self.picked.iter_mut()) {
            let picked = picks.picked(entry);
            if look_changed || *old != picked {
                *old = picked;
                card.update(cx, |card, cx| {
                    card.picked = picked;
                    card.ui = ui;
                    cx.notify();
                });
            }
        }
        for ((card, pair), old) in self.pair_cards.iter().zip(&self.pairs).zip(self.pair_picked.iter_mut()) {
            let picked = picks.picked_pair(pair, entries);
            if look_changed || *old != picked {
                *old = picked;
                card.update(cx, |card, cx| {
                    card.picked = picked;
                    card.ui = ui;
                    cx.notify();
                });
            }
        }
        if look_changed {
            self.look = Some(theme);
        }
    }

    /// `cards` at `shown`, each in the fixed box that lets the window skip it while it hasn't changed.
    fn views(cards: &[Entity<ThemeCard>], shown: &[usize]) -> Vec<AnyElement> {
        let slot = StyleRefinement::default().w_full().h(card_height());
        shown
            .iter()
            .filter_map(|ix| cards.get(*ix))
            .map(|card| AnyView::from(card.clone()).cached(slot.clone()).into_any_element())
            .collect()
    }
}

/// Every theme resolved once; rebuilt only when the themes dir changes
/// (imports, Studio saves, files dropped in by hand), since resolving all of
/// them on every render would be wasteful.
#[derive(Default)]
struct PreviewCache {
    fingerprint: Option<Vec<(std::path::PathBuf, u64, Option<std::time::SystemTime>)>>,
    entries: Rc<Vec<ThemeEntry>>,
}

impl Global for PreviewCache {}

fn entries(cx: &mut App) -> Rc<Vec<ThemeEntry>> {
    let fingerprint = catalog::fingerprint_in(&crate::active_profile::paths().themes_dir());
    let cache = cx.default_global::<PreviewCache>();
    if cache.fingerprint.as_ref() != Some(&fingerprint) {
        cache.entries = Rc::new(catalog::catalog_in(&crate::active_profile::paths().themes_dir()));
        cache.fingerprint = Some(fingerprint);
    }
    cache.entries.clone()
}

/// The first `count` of a set's cards, with its pick among them in place of the last.
fn first_cards(all: &[usize], count: usize, picked: impl Fn(usize) -> bool) -> Vec<usize> {
    let mut first: Vec<usize> = all.iter().copied().take(count).collect();
    if let Some(pick) = all.iter().copied().skip(count).find(|ix| picked(*ix)) {
        first.pop();
        first.push(pick);
    }
    first
}

impl SettingsWindow {
    /// Whether the picker shows a card per kind (Dark and light separately), else pairs.
    pub(crate) fn themes_separate(&self, cx: &mut App) -> bool {
        self.themes_separate.unwrap_or_else(|| !picks_are_a_pair(cx))
    }

    /// Choose → "One theme for both": the light pick becomes the partner of the dark one.
    pub(crate) fn choose_pairing(&mut self, separate: bool, cx: &mut Context<Self>) {
        self.themes_separate = Some(separate);
        if !separate {
            let entries = entries(cx);
            let dark = settings(cx).theme.clone();
            let found = pairs::pairs(&entries)
                .into_iter()
                .find(|pair| pair.dark.is_some_and(|ix| entries[ix].is_selected_by(&dark)));
            if let Some(light) = found.and_then(|pair| pair.light) {
                let value = entries[light].value.clone();
                update_settings(cx, move |s| s.theme_light = value);
            }
        }
        cx.notify();
    }

    pub(super) fn render_theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let separate = self.themes_separate(cx);
        let entries = entries(cx);
        let store = cx.global::<SettingsStore>();
        let (dark, light) = (store.settings.theme.clone(), store.settings.theme_light.clone());
        let hidden_themes = store.settings.hidden_themes.clone();
        let picks = Picks { dark: &dark, light: &light };
        let theme = store.revisions.theme;
        let mut set = self.theme_picker.cards.borrow_mut();
        set.sync(&entries, &picks, theme, ui, cx);

        let is_hidden = |ix: usize| hidden_themes.contains(&entries[ix].value);
        let (mut total, mut hidden_cards) = (0, 0);
        let mut sets: Vec<AnyElement> = Vec::new();
        let mut add = |caption: Option<SharedString>,
                       all: Vec<usize>,
                       picked: &dyn Fn(usize) -> bool,
                       cards: &[Entity<ThemeCard>],
                       count: usize| {
            let shown = if self.themes_all { all.clone() } else { first_cards(&all, count, picked) };
            total += all.len();
            hidden_cards += all.len() - shown.len();
            if shown.is_empty() {
                return;
            }
            sets.push(
                ThemeSetProps {
                    id: ("theme-set", sets.len()).into(),
                    spoken: caption.clone().unwrap_or_else(|| text(SETTINGS_APPEARANCE_PAIRING_BOTH).into()),
                    caption,
                    cards: CardSet::views(cards, &shown),
                }
                .render(&Look::of(cx))
                .into_any_element(),
            );
        };
        if separate {
            for (kind, caption) in [
                (Appearance::Dark, SETTINGS_APPEARANCE_IN_DARK_MODE),
                (Appearance::Light, SETTINGS_APPEARANCE_IN_LIGHT_MODE),
            ] {
                let all: Vec<usize> = (0..entries.len())
                    .filter(|ix| entries[*ix].tokens.appearance == kind)
                    .filter(|ix| set.picked[*ix] || !is_hidden(*ix))
                    .collect();
                add(Some(text(caption).into()), all, &|ix| set.picked[ix], &set.cards, FIRST_CARDS / 2);
            }
        } else {
            let all: Vec<usize> = (0..set.pairs.len())
                .filter(|ix| set.pair_picked[*ix] || !set.pairs[*ix].halves().all(&is_hidden))
                .collect();
            add(None, all, &|ix| set.pair_picked[ix], &set.pair_cards, FIRST_CARDS);
        }

        let toggle_all = cx.listener(|this, _, _, cx| {
            this.themes_all = !this.themes_all;
            cx.notify();
        });
        let toggle_hidden = cx.listener(|this, _, _, cx| {
            this.theme_picker.hidden_open = !this.theme_picker.hidden_open;
            cx.notify();
        });
        let import = cx.listener(|_, _, _, cx| import_theme(cx.weak_entity(), cx));
        let show_all = if self.themes_all {
            text(SETTINGS_APPEARANCE_SHOW_FEWER).to_string()
        } else {
            settings_appearance_show_all_themes(&total.to_string())
        };
        let block = ThemesBlockProps {
            id: "themes-block".into(),
            spoken: text(SETTINGS_APPEARANCE_THEME).into(),
            show_all: if self.themes_all || hidden_cards > 0 { show_all.into() } else { "".into() },
            on_show_all: Some(Rc::new(move |e: &ClickEvent, w: &mut Window, cx: &mut App| {
                toggle_all(e, w, cx)
            }) as Handler),
            hidden: (!hidden_themes.is_empty())
                .then(|| settings_appearance_hidden_themes(&hidden_themes.len().to_string()).into()),
            on_hidden: Some(Rc::new(move |e: &ClickEvent, w: &mut Window, cx: &mut App| {
                toggle_hidden(e, w, cx)
            }) as Handler),
            sets: div().flex().flex_col().gap(px(12.0)).children(sets).into_any_element(),
            buttons: div()
                .flex()
                .gap_2()
                .child(
                    style::form_button("import-theme")
                        .label(text(SETTINGS_APPEARANCE_IMPORT_THEME))
                        .on_click(move |e, w, cx| import(e, w, cx)),
                )
                .child(
                    style::form_button("open-studio")
                        .label(text(SETTINGS_APPEARANCE_THEME_STUDIO))
                        .on_click(|_, _, cx| studio::open_studio(cx)),
                )
                .into_any_element(),
        }
        .render(&Look::of(cx));
        let hidden_list = (!hidden_themes.is_empty())
            .then(|| self.render_hidden_themes(&hidden_themes, &entries, &set, ui, cx));
        div().flex().flex_col().child(block).children(hidden_list).into_any_element()
    }

    /// "Hidden themes (n)" and, while it is open, a row per hidden theme with Show. A name no theme
    /// has any more (its file is gone) is listed as it is written, so it can still be taken off.
    fn render_hidden_themes(
        &self,
        hidden: &[String],
        entries: &[ThemeEntry],
        set: &CardSet,
        ui: UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let open = self.theme_picker.hidden_open;
        let toggle = style::link_button(
            "themes-hidden",
            ui,
            settings_appearance_hidden_themes(&hidden.len().to_string()),
            false,
        )
        .text_color(ui.text)
        .child(style::icon(if open { IconName::ChevronUp } else { IconName::ChevronDown }))
        .on_click(cx.listener(|this, _, _, cx| {
            this.theme_picker.hidden_open = !this.theme_picker.hidden_open;
            cx.notify();
        }));
        let rows = open.then(|| {
            hidden.iter().enumerate().map(|(ix, value)| {
                let found = entries.iter().position(|entry| &entry.value == value);
                let name =
                    found.and_then(|at| set.names.get(at).cloned()).unwrap_or_else(|| value.clone().into());
                let kind = found.map(|at| match entries[at].tokens.appearance {
                    Appearance::Dark => text(SETTINGS_APPEARANCE_DARK).into(),
                    Appearance::Light => text(SETTINGS_APPEARANCE_LIGHT).into(),
                });
                let value = value.clone();
                let show =
                    style::link_button(("theme-show", ix), ui, text(SETTINGS_APPEARANCE_SHOW_THEME), false)
                        .on_click(move |_, _, cx| {
                            let value = value.clone();
                            update_settings(cx, move |s| {
                                s.set_theme_hidden(&value, false);
                            });
                        })
                        .into_any_element();
                style::setting(
                    ("hidden-theme", ix),
                    ui,
                    style::SettingParts {
                        label: name,
                        description: kind,
                        visible_hint: false,
                        chip: None,
                        keys: style::RowKeys::NONE,
                        state: style::SettingState::Default,
                        found: Vec::new(),
                        found_description: Vec::new(),
                        control: show,
                        tall: false,
                        extra: None,
                        under: None,
                        on_reset: None,
                    },
                    cx,
                )
                .into_any_element()
            })
        });
        div()
            .flex()
            .flex_col()
            .child(div().flex().child(toggle))
            .children(rows.into_iter().flatten())
            .into_any_element()
    }
}
