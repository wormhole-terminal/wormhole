//! The theme picker: the themes as cards 88 high, four to a row, each a miniature of the app in
//! that theme's own colors with its name in a strip under it. The mode decides what shows: under
//! Dark or Light one shelf of themes of that kind, under Match system two, "In Dark Mode" and "In
//! Light Mode". The pick of each shelf has the primary's ring, its name in 600 and a check, and a
//! click makes a card the pick of its kind. A theme that ships several versions has a card per
//! version, on the shelf of its kind. Until "Show all" the shelves show their first cards, the pick
//! always among them; the filter box narrows every shelf.
//!
//! Each card is a view of its own, drawn from a fixed-size box, so the settings window can
//! leave the ~40 of them alone while it redraws for a scroll tick or a keystroke in the filter:
//! only a card whose selection or look changed is told to draw again.

use std::cell::RefCell;
use std::rc::Rc;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use wormhole_core::config::ThemeMode;
use wormhole_core::theme::catalog::{self, ThemeEntry};
use wormhole_core::theme::tokens::Appearance;

mod card;

#[cfg(test)]
use self::card::CARD_RENDERS;
use self::card::ThemeCard;
use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::{SettingsStore, UiColors};
use crate::settings_window::sections::import_theme;
use crate::settings_window::{SettingsWindow, ui};
use crate::studio;
use crate::style::{self, TextRole};

/// How many cards show before "Show all": two rows, shared by the shelves.
const FIRST_CARDS: usize = 8;

/// The height of a card. A cached view is laid out from a style the window gives it, so this
/// has to be exact; everything in a card is in points, so it holds at every interface size.
fn card_height(_rem: Pixels) -> Pixels {
    px(style::THEME_CARD_H)
}

/// The picker's own state inside the settings window.
pub(crate) struct ThemePicker {
    filter: Entity<InputState>,
    /// How many themes the filter's placeholder counts ("Filter 14 themes").
    counted: usize,
    cards: RefCell<CardSet>,
    _subscription: Subscription,
}

impl ThemePicker {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let counted = entries(cx).len();
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(filter_placeholder(counted)));
        let subscription = cx.subscribe(&filter, |_, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        });
        Self { filter, counted, cards: RefCell::default(), _subscription: subscription }
    }

    /// Keep the filter's placeholder at the live count ("Filter 14 themes") as themes are
    /// imported, saved or dropped into the folder. Called from the window's refresh, not a draw.
    pub(crate) fn sync_placeholder(&mut self, window: &mut Window, cx: &mut App) {
        let count = entries(cx).len();
        if count != self.counted {
            self.counted = count;
            self.filter
                .update(cx, |filter, cx| filter.set_placeholder(filter_placeholder(count), window, cx));
        }
    }
}

/// The filter's placeholder: how many themes there are to filter.
fn filter_placeholder(themes: usize) -> String {
    match themes {
        0 => text(SETTINGS_APPEARANCE_FILTER_THEMES).to_string(),
        n => settings_appearance_filter_themes_count(whole(n)),
    }
}

/// The two picks: the theme of Dark Mode (`theme`) and of Light Mode (`theme_light`).
struct Picks<'a> {
    dark: &'a str,
    light: &'a str,
}

impl Picks<'_> {
    /// Whether `entry` is the pick of its own kind. A theme set as the pick of the other kind (a
    /// light `theme` chosen before there was a light pick) is no shelf's pick.
    fn picked(&self, entry: &ThemeEntry) -> bool {
        match entry.tokens.appearance {
            Appearance::Dark => entry.is_selected_by(self.dark),
            Appearance::Light => entry.is_selected_by(self.light),
        }
    }
}

/// What a card's strip says: a family's name when the family has one version of the card's kind
/// (its shelf then holds only that one), else the version's own name ("Soltari Dim").
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

/// One card per theme of the catalog, kept while the catalog is the same.
#[derive(Default)]
struct CardSet {
    catalog: Option<Rc<Vec<ThemeEntry>>>,
    cards: Vec<Entity<ThemeCard>>,
    /// Whether each card is its kind's pick, as last given.
    picked: Vec<bool>,
    /// Each card's names (its own and the family's), lowercased once, for the filter box.
    search: Vec<String>,
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
            let labels = labels(entries);
            self.picked = entries.iter().map(|entry| picks.picked(entry)).collect();
            self.cards = labels
                .into_iter()
                .zip(&self.picked)
                .enumerate()
                .map(|(entry, (label, &picked))| {
                    let catalog = entries.clone();
                    cx.new(|_| ThemeCard { catalog, entry, label, picked, ui })
                })
                .collect();
            self.search = entries
                .iter()
                .map(|entry| format!("{}\n{}", entry.label, entry.card_label()).to_lowercase())
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
        if look_changed {
            self.look = Some(theme);
        }
    }

    /// The cards at `shown` (indices into the catalog), each in the fixed box that lets the window
    /// skip it while it hasn't changed.
    fn views(&self, rem: Pixels, shown: &[usize]) -> Vec<AnyElement> {
        let slot = StyleRefinement::default().w_full().h(card_height(rem));
        shown
            .iter()
            .filter_map(|ix| self.cards.get(*ix))
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

/// The shelves a mode shows, in order.
fn shelves(mode: ThemeMode) -> &'static [Appearance] {
    match mode {
        ThemeMode::Dark => &[Appearance::Dark],
        ThemeMode::Light => &[Appearance::Light],
        ThemeMode::System => &[Appearance::Dark, Appearance::Light],
    }
}

/// The first `count` of a shelf's cards, with its pick among them in place of the last.
fn first_cards(all: &[usize], count: usize, picked: impl Fn(usize) -> bool) -> Vec<usize> {
    let mut first: Vec<usize> = all.iter().copied().take(count).collect();
    if let Some(pick) = all.iter().copied().skip(count).find(|ix| picked(*ix)) {
        first.pop();
        first.push(pick);
    }
    first
}

impl SettingsWindow {
    pub(super) fn render_theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let entries = entries(cx);
        let store = cx.global::<SettingsStore>();
        let (dark, light) = (store.settings.theme.clone(), store.settings.theme_light.clone());
        let kinds = shelves(store.settings.theme_mode);
        let picks = Picks { dark: &dark, light: &light };
        let theme = store.revisions.theme;
        let query = self.theme_picker.filter.read(cx).value().trim().to_lowercase();
        let mut set = self.theme_picker.cards.borrow_mut();
        set.sync(&entries, &picks, theme, ui, cx);

        let narrowed = !query.is_empty();
        let (mut total, mut hidden) = (0, 0);
        // The shelves are runs of one grid, each after its name across the whole row: cards nested in
        // a box per shelf lose their cached draw from one frame to the next.
        let mut cells: Vec<AnyElement> = Vec::new();
        for (place, &kind) in kinds.iter().enumerate() {
            let all: Vec<usize> = (0..entries.len())
                .filter(|ix| entries[*ix].tokens.appearance == kind)
                .filter(|ix| query.is_empty() || set.search[*ix].contains(&query))
                .collect();
            let shown = if narrowed || self.themes_all {
                all.clone()
            } else {
                first_cards(&all, FIRST_CARDS / kinds.len(), |ix| set.picked[ix])
            };
            total += all.len();
            hidden += all.len() - shown.len();
            // One shelf needs no name: the mode above says which kind it is.
            if kinds.len() > 1 && !shown.is_empty() {
                let heading = match kind {
                    Appearance::Dark => text(SETTINGS_APPEARANCE_IN_DARK_MODE),
                    Appearance::Light => text(SETTINGS_APPEARANCE_IN_LIGHT_MODE),
                };
                cells.push(
                    div()
                        .col_span_full()
                        .when(place > 0, |d| d.pt_2())
                        .text_label()
                        .text_color(ui.muted)
                        .child(heading)
                        .into_any_element(),
                );
            }
            cells.extend(set.views(self.rem, &shown));
        }

        let toolbar = div().w(px(FILTER_W)).child(
            Input::new(&self.theme_picker.filter)
                .small()
                .cleanable(true)
                .prefix(style::icon(IconName::Search).text_color(ui.muted)),
        );
        let footer = div()
            .flex()
            .items_center()
            .gap_2()
            .when(!narrowed && (self.themes_all || hidden > 0), |d| {
                d.child(
                    style::link_button(
                        "themes-all",
                        ui,
                        if self.themes_all {
                            text(SETTINGS_APPEARANCE_SHOW_FEWER).to_string()
                        } else {
                            settings_appearance_show_all_themes(&total.to_string())
                        },
                        false,
                    )
                    .text_color(ui.text)
                    .child(style::icon(if self.themes_all {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.themes_all = !this.themes_all;
                        cx.notify();
                    })),
                )
            })
            .child(div().flex_1())
            .child(
                Button::new("import-theme")
                    .outline()
                    .small()
                    .icon(IconName::ArrowDown)
                    .label(text(SETTINGS_APPEARANCE_IMPORT_THEME))
                    .on_click(cx.listener(|_, _, _, cx| import_theme(cx.weak_entity(), cx))),
            )
            .child(
                Button::new("open-studio")
                    .outline()
                    .small()
                    .icon(IconName::Palette)
                    .label(text(SETTINGS_APPEARANCE_THEME_STUDIO))
                    .child(style::keycap(
                        ui,
                        wormhole_core::keys::keymap::glyphs(
                            wormhole_core::keys::keymap::ActionId::ThemeStudio,
                        ),
                    ))
                    .on_click(cx.listener(|_, _, _, cx| studio::open_studio(cx))),
            );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(toolbar)
            .child(div().grid().grid_cols(4).gap_3().children(cells))
            .when(narrowed && total == 0, |d| {
                d.child(
                    div()
                        .py_4()
                        .text_body()
                        .text_color(ui.muted)
                        .child(text(SETTINGS_APPEARANCE_NO_THEMES_MATCH_THE_FILTER)),
                )
            })
            .child(footer)
            .into_any_element()
    }
}

/// The filter box in the theme toolbar.
const FILTER_W: f32 = 320.0;

/// The cards in a headless GPUI (needs `devctl`'s test support): that the window leaves
/// them alone, and that the box it gives them is the height they really have.
#[cfg(all(test, feature = "devctl"))]
mod tests {
    use std::path::Path;
    use std::rc::Rc;

    use gpui_kit::component::Theme as KitTheme;
    use gpui_kit::test::TestWindowExt as _;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
        ParentElement as _, Pixels, Render, Styled as _, TestAppContext, TestSupportExt as _, Window,
        WindowOptions, div, px,
    };
    use wormhole_core::theme::catalog;
    use wormhole_core::theme::tokens::Appearance;

    use super::filter_placeholder;

    /// The filter says how many themes there are ("Filter 14 themes").
    #[test]
    fn the_filter_counts_the_themes() {
        assert_eq!(filter_placeholder(14), "Filter 14 themes");
        assert_eq!(filter_placeholder(1), "Filter 1 theme");
        assert_eq!(filter_placeholder(0), "Filter themes", "nothing to count yet");
    }

    use super::{CARD_RENDERS, CardSet, ThemeCard, card_height};
    use crate::settings::{SettingsStore, ui};

    /// Draws the cards as the picker does, from a set it keeps.
    struct Host {
        entries: Rc<Vec<wormhole_core::theme::catalog::ThemeEntry>>,
        set: CardSet,
        selected: String,
    }

    impl Render for Host {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let (ui, theme) = (ui(cx), cx.global::<SettingsStore>().revisions.theme);
            let entries = self.entries.clone();
            let selected = self.selected.clone();
            let picks = super::Picks { dark: &selected, light: &selected };
            self.set.sync(&entries, &picks, theme, ui, cx);
            div()
                .w(px(900.0))
                .grid()
                .grid_cols(4)
                .gap_3()
                .children(self.set.views(window.rem_size(), &(0..self.entries.len()).collect::<Vec<_>>()))
        }
    }

    fn renders() -> usize {
        CARD_RENDERS.with(|n| n.get())
    }

    fn open(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<Host>) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            // The built-in themes only: nothing is read from the user's themes folder.
            let entries = Rc::new(catalog::catalog_in(Path::new("/nonexistent/wormhole-themes")));
            let selected = entries[0].value.clone();
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| {
                cx.new(|_| Host { entries, set: CardSet::default(), selected })
            })
            .expect("opens the window")
        })
    }

    fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) {
        cx.update(|cx| {
            handle.update(cx, |_, window, cx| window.draw(cx).clear(cx)).expect("the window is open");
        });
    }

    #[gpui_kit::test]
    fn a_redraw_leaves_the_cards_alone_until_one_changes(cx: &mut TestAppContext) {
        let (handle, host) = open(cx);
        draw(cx, handle);
        let cards = host.read_with(cx, |host, _| host.set.cards.len());
        assert!(cards >= 10, "the built-in themes are cards: {cards}");
        let drawn = renders();
        assert!(drawn >= cards, "every card is drawn at first: {drawn}");

        // The window draws again (a scroll tick, a keystroke in the filter): its own view
        // runs, the cards do not.
        cx.update(|cx| host.update(cx, |_, cx| cx.notify()));
        draw(cx, handle);
        assert_eq!(renders(), drawn, "the cards were drawn again for nothing");

        // Choosing another theme of the same kind tells the old pick and the new one, and only
        // those.
        let other = host.read_with(cx, |host, _| {
            let first = &host.entries[0];
            host.entries[1..]
                .iter()
                .find(|e| e.tokens.appearance == first.tokens.appearance)
                .expect("a second theme of the first one's kind")
                .value
                .clone()
        });
        cx.update(|cx| {
            host.update(cx, |host, cx| {
                host.selected = other;
                cx.notify();
            })
        });
        draw(cx, handle);
        assert_eq!(renders(), drawn + 2, "only the card that lost and the card that gained the selection");
    }

    /// A family has a card per version, each on the shelf of its kind: a family with one version
    /// of a kind is named by the family there, one with several by each version's own name.
    #[test]
    fn a_family_is_a_card_per_version_named_by_its_shelf() {
        let entries = catalog::catalog_in(Path::new("/nonexistent/wormhole-themes"));
        let labels = super::labels(&entries);
        let soltari: Vec<(&str, Appearance)> = entries
            .iter()
            .zip(&labels)
            .filter(|(e, _)| e.card_label() == "Soltari")
            .map(|(e, label)| (label.as_ref(), e.tokens.appearance))
            .collect();
        assert!(soltari.len() > 1, "Soltari ships several versions: {soltari:?}");
        for kind in [Appearance::Dark, Appearance::Light] {
            let of_kind: Vec<&str> = soltari.iter().filter(|(_, k)| *k == kind).map(|(l, _)| *l).collect();
            if of_kind.len() == 1 {
                assert_eq!(of_kind, ["Soltari"], "the only {kind:?} version is named by its family");
            } else {
                assert!(of_kind.iter().all(|l| *l != "Soltari"), "each {kind:?} version by its own name");
            }
        }
    }

    /// Collapsed, a shelf shows its first cards, with its pick in place of the last when it isn't
    /// among them.
    #[test]
    fn the_first_cards_hold_the_pick() {
        let all = [3, 5, 8, 9, 12];
        assert_eq!(super::first_cards(&all, 3, |_| false), [3, 5, 8]);
        assert_eq!(super::first_cards(&all, 3, |ix| ix == 5), [3, 5, 8]);
        assert_eq!(super::first_cards(&all, 3, |ix| ix == 12), [3, 5, 12]);
    }

    #[gpui_kit::test]
    fn a_card_is_as_tall_as_the_box_it_is_given(cx: &mut TestAppContext) {
        struct Natural(Entity<ThemeCard>);
        impl Render for Natural {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                // The card on its own, free to take the height it needs.
                div().id("card-box").test_support().w(px(280.0)).child(self.0.clone())
            }
        }
        let card = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            let entries = Rc::new(catalog::catalog_in(Path::new("/nonexistent/wormhole-themes")));
            let ui = ui(cx);
            let label = entries[0].label.clone().into();
            cx.new(|_| ThemeCard { catalog: entries, entry: 0, label, picked: true, ui })
        });
        let handle = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Natural(card)))
                .expect("opens the window")
                .0
        });
        // At the interface font sizes the app can have: GPUI Kit's root makes the window's
        // rem its theme's font size, and the interface size sets that (theme_bridge).
        for rem in [13.0, 16.0, 20.0, 26.0] {
            cx.update(|cx| KitTheme::update(cx, |theme| theme.font_size = px(rem)));
            cx.update(|cx| handle.update(cx, |_, window, cx| window.render_frame(cx)).expect("open"));
            let height: Pixels = cx.update(|cx| {
                handle.update(cx, |_, window, _| window.find("card-box").bounds().size.height).expect("open")
            });
            let expected = card_height(px(rem));
            assert!(
                (f32::from(height) - f32::from(expected)).abs() < 0.5,
                "at rem {rem} the card is {height:?} tall, its box {expected:?}"
            );
        }
    }
}
