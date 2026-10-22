//! The theme picker: a filter, a grid of cards 88 high, four to a row, each a miniature
//! of the app in that theme's own colors with its name in a strip under it. The first two rows
//! show until "Show all"; the card of the theme in use has the primary's ring, its name in 600
//! and a check, and the dark and light picks of Match system wear a moon and a sun. A theme that
//! ships several versions is one card, with a swatch per version in its strip (see `card`).
//!
//! Each card is a view of its own, drawn from a fixed-size box, so the settings window can
//! leave the ~40 of them alone while it redraws for a scroll tick or a keystroke in the filter:
//! only a card whose selection, mark or look changed is told to draw again.

use std::cell::RefCell;
use std::rc::Rc;

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
use crate::style::{self, TextRole};
use crate::workspace::settings_page::sections::import_theme;
use crate::workspace::settings_page::{SettingsWindow, ui};
use crate::workspace::studio;

/// How many cards show before "Show all": two rows.
const FIRST_CARDS: usize = 8;
/// Of those, how many are light themes.
const FIRST_LIGHT: usize = 2;

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
    /// `None` shows dark and light themes.
    appearance: Option<Appearance>,
    cards: RefCell<CardSet>,
    _subscription: Subscription,
}

impl ThemePicker {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<SettingsWindow>) -> Self {
        let counted = card_count(cx);
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(filter_placeholder(counted)));
        let subscription = cx.subscribe(&filter, |_, _, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        });
        Self { filter, counted, appearance: None, cards: RefCell::default(), _subscription: subscription }
    }

    /// Keep the filter's placeholder at the live count ("Filter 14 themes") as themes are
    /// imported, saved or dropped into the folder. Called from the window's refresh, not a draw.
    pub(crate) fn sync_placeholder(&mut self, window: &mut Window, cx: &mut App) {
        let count = card_count(cx);
        if count != self.counted {
            self.counted = count;
            self.filter
                .update(cx, |filter, cx| filter.set_placeholder(filter_placeholder(count), window, cx));
        }
    }
}

/// How many cards there are, as "Show all" counts them: a family of versions is one.
fn card_count(cx: &mut App) -> usize {
    catalog::cards(&entries(cx)).len()
}

/// The filter's placeholder: how many themes there are to filter.
fn filter_placeholder(count: usize) -> String {
    match count {
        0 => "Filter themes".to_string(),
        1 => "Filter 1 theme".to_string(),
        n => format!("Filter {n} themes"),
    }
}

/// What a card says about itself besides its themes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CardLook {
    /// The theme in use is one of the card's.
    active: bool,
    /// The dark pick (`theme`) is one of the card's.
    dark_pick: bool,
    /// The light pick (`theme_light`) is one of the card's.
    light_pick: bool,
    /// Which of the card's themes it draws (an index into the catalog).
    shown: usize,
    /// Which of a family's versions are in use or picked, by their place in the card (a file
    /// ships at most `tokens::MAX_VARIANTS`, 32).
    picked: u32,
}

/// What decides each card's look: the dark pick, the light pick, which is shown, and the
/// kind of theme the filter asks for.
struct Picks<'a> {
    dark: &'a str,
    light: &'a str,
    shown: &'a str,
    mode: Option<Appearance>,
}

impl Picks<'_> {
    /// A card's look: what its themes say together, and the one it draws: the one in use, else
    /// a pick, else its first, preferring those of the kind the filter asks for. A pick is
    /// marked on a theme of its own kind only: a light theme set as the dark pick (a `theme`
    /// chosen before there was a light pick) wears no moon.
    fn look(&self, entries: &[ThemeEntry], members: &[usize]) -> CardLook {
        let mut look = CardLook { shown: members[0], ..CardLook::default() };
        let mut best = usize::MAX;
        for (place, &ix) in members.iter().enumerate() {
            let entry = &entries[ix];
            let light = entry.tokens.appearance == Appearance::Light;
            let active = entry.is_selected_by(self.shown);
            let pick = if light { entry.is_selected_by(self.light) } else { entry.is_selected_by(self.dark) };
            look.active |= active;
            look.dark_pick |= pick && !light;
            look.light_pick |= pick && light;
            if (active || pick) && place < 32 {
                look.picked |= 1 << place;
            }
            let fits = self.mode.is_none_or(|m| entry.tokens.appearance == m);
            let rank = match (fits, active, pick) {
                (false, ..) => 3,
                (true, true, _) => 0,
                (true, false, true) => 1,
                (true, false, false) => 2,
            };
            if rank < best {
                best = rank;
                look.shown = ix;
            }
        }
        look
    }
}

impl CardSet {
    /// The cards `show` accepts (in `order`), each in the fixed box that lets the window skip
    /// it while it hasn't changed.
    fn views(&self, rem: Pixels, order: &[usize], show: impl Fn(usize) -> bool) -> Vec<AnyElement> {
        let slot = StyleRefinement::default().w_full().h(card_height(rem));
        order
            .iter()
            .copied()
            .filter(|ix| *ix < self.cards.len() && show(*ix))
            .map(|ix| AnyView::from(self.cards[ix].clone()).cached(slot.clone()).into_any_element())
            .collect()
    }
}

/// One card per theme of the catalog, and one per family of versions, kept while the catalog
/// is the same.
#[derive(Default)]
struct CardSet {
    catalog: Option<Rc<Vec<ThemeEntry>>>,
    cards: Vec<Entity<ThemeCard>>,
    /// Each card's themes, by index into the catalog.
    members: Vec<Rc<[usize]>>,
    /// Each card's look as last given.
    looks: Vec<CardLook>,
    /// Each card's names (the family's and every version's), lowercased once, for the filter box.
    labels: Vec<String>,
    /// The theme revision the cards were last given.
    look: Option<u64>,
}

impl CardSet {
    /// Make the cards match `entries`, and tell the ones whose look or selection changed.
    fn sync<T: 'static>(
        &mut self,
        entries: &Rc<Vec<ThemeEntry>>,
        picks: &Picks,
        theme: u64,
        ui: UiColors,
        cx: &mut Context<T>,
    ) {
        if !self.catalog.as_ref().is_some_and(|catalog| Rc::ptr_eq(catalog, entries)) {
            self.members = catalog::cards(entries).into_iter().map(Rc::from).collect();
            self.looks = self.members.iter().map(|members| picks.look(entries, members)).collect();
            self.cards = self
                .members
                .iter()
                .zip(&self.looks)
                .enumerate()
                .map(|(card, (members, look))| {
                    let (catalog, members, look) = (entries.clone(), members.clone(), *look);
                    cx.new(|_| ThemeCard { catalog, card, members, look, ui })
                })
                .collect();
            self.labels = self
                .members
                .iter()
                .map(|members| {
                    let names = members.iter().map(|ix| entries[*ix].label.as_str());
                    let all: Vec<&str> =
                        std::iter::once(entries[members[0]].card_label()).chain(names).collect();
                    all.join("\n").to_lowercase()
                })
                .collect();
            self.catalog = Some(entries.clone());
            self.look = Some(theme);
            return;
        }
        let look_changed = self.look != Some(theme);
        for ((card, members), old) in self.cards.iter().zip(&self.members).zip(self.looks.iter_mut()) {
            let look = picks.look(entries, members);
            if look_changed || *old != look {
                *old = look;
                card.update(cx, |card, cx| {
                    card.look = look;
                    card.ui = ui;
                    cx.notify();
                });
            }
        }
        if look_changed {
            self.look = Some(theme);
        }
    }

    /// The kind of theme a card draws.
    fn appearance(&self, entries: &[ThemeEntry], card: usize) -> Appearance {
        entries[self.looks[card].shown].tokens.appearance
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

/// Which cards show before "Show all": the first dark themes and the first light ones, with
/// the picks in use always among them.
fn first_cards(entries: &[ThemeEntry], set: &CardSet) -> Vec<usize> {
    let of = |appearance: Appearance| -> Vec<usize> {
        (0..set.cards.len()).filter(|card| set.appearance(entries, *card) == appearance).collect()
    };
    let mut shown = Vec::new();
    for (appearance, count) in
        [(Appearance::Dark, FIRST_CARDS - FIRST_LIGHT), (Appearance::Light, FIRST_LIGHT)]
    {
        let all = of(appearance);
        let mut first: Vec<usize> = all.iter().copied().take(count).collect();
        for card in &all {
            let look = set.looks[*card];
            if (look.active || look.dark_pick || look.light_pick) && !first.contains(card) {
                first.pop();
                first.push(*card);
            }
        }
        shown.extend(first);
    }
    shown.sort_unstable();
    shown
}

impl SettingsWindow {
    pub(super) fn render_theme_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let entries = entries(cx);
        let store = cx.global::<SettingsStore>();
        let (dark, light) = (store.settings.theme.clone(), store.settings.theme_light.clone());
        // The mode and, under Match system, the system's appearance decide which pick is shown.
        let shown = store.shown_theme().to_string();
        let mode = self.theme_picker.appearance;
        let picks = Picks { dark: &dark, light: &light, shown: &shown, mode };
        let theme = cx.global::<SettingsStore>().revisions.theme;
        let query = self.theme_picker.filter.read(cx).value().trim().to_lowercase();
        let mut set = self.theme_picker.cards.borrow_mut();
        set.sync(&entries, &picks, theme, ui, cx);

        let narrowed = !query.is_empty() || mode.is_some();
        let first = (!narrowed && !self.themes_all).then(|| first_cards(&entries, &set));
        // A family's card draws a version of the filter's kind when it has one.
        let matches = |card: usize| {
            mode.is_none_or(|m| m == set.appearance(&entries, card))
                && (query.is_empty() || set.labels[card].contains(&query))
                && first.as_ref().is_none_or(|first| first.contains(&card))
        };
        // Themes like the one shown first (dark ones in a dark theme; light in a light one),
        // each kind in the catalog's order.
        let shown_look = entries.iter().find(|e| e.is_selected_by(picks.shown)).map(|e| e.tokens.appearance);
        let mut order: Vec<usize> = (0..set.cards.len()).collect();
        order.sort_by_key(|card| Some(set.appearance(&entries, *card)) != shown_look);
        let cards = set.views(self.rem, &order, matches);
        let shown = cards.len();

        let mode_segment = |ix: usize, label: &'static str, value: Option<Appearance>| {
            style::form_segment(("theme-filter", ix), ui, None, label, mode == value).on_click(cx.listener(
                move |this, _, _, cx| {
                    this.theme_picker.appearance = value;
                    cx.notify();
                },
            ))
        };
        let toolbar = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div().w(px(FILTER_W)).child(
                    Input::new(&self.theme_picker.filter)
                        .small()
                        .cleanable(true)
                        .prefix(style::icon(IconName::Search).text_color(ui.muted)),
                ),
            )
            .child(
                style::segmented_form(ui)
                    .child(mode_segment(0, "All", None))
                    .child(mode_segment(1, "Dark", Some(Appearance::Dark)))
                    .child(mode_segment(2, "Light", Some(Appearance::Light))),
            );
        let total = set.cards.len();
        let footer = div()
            .flex()
            .items_center()
            .gap_2()
            .when(!narrowed, |d| {
                d.child(
                    style::link_button(
                        "themes-all",
                        ui,
                        if self.themes_all { "Show fewer".to_string() } else { format!("Show all {total}") },
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
                    .label("Import theme…")
                    .on_click(cx.listener(|_, _, _, cx| import_theme(cx.weak_entity(), cx))),
            )
            .child(
                Button::new("open-studio")
                    .outline()
                    .small()
                    .icon(IconName::Palette)
                    .label("Theme Studio")
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
            .child(div().grid().grid_cols(4).gap_3().children(cards))
            .when(shown == 0, |d| {
                d.child(div().py_4().text_body().text_color(ui.muted).child("No themes match the filter."))
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
    use wormhole_core::theme::registry::DEFAULT_THEME;

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
            let picks = super::Picks { dark: &selected, light: "", shown: &selected, mode: None };
            self.set.sync(&entries, &picks, theme, ui, cx);
            div().w(px(900.0)).grid().grid_cols(4).gap_3().children(self.set.views(
                window.rem_size(),
                &(0..self.entries.len()).collect::<Vec<_>>(),
                |_| true,
            ))
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

        // Choosing another theme tells the old and the new one, and only those (the dark pick
        // and the theme shown move together here). The first card is a family: the other theme
        // is one of a card of its own.
        let other = host.read_with(cx, |host, _| {
            host.entries.iter().find(|e| e.family.is_none()).expect("a theme of one version").value.clone()
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

    /// A family's card under Match system: a version's swatch sets that version's pick only, the
    /// card around it both picks.
    #[gpui_kit::test]
    fn a_swatch_picks_its_version_and_the_card_its_family(cx: &mut TestAppContext) {
        struct Natural(Entity<ThemeCard>);
        impl Render for Natural {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                div().id("card-box").test_support().w(px(280.0)).child(self.0.clone())
            }
        }
        let card = cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_global(SettingsStore::for_test());
            let entries = Rc::new(catalog::catalog_in(Path::new("/nonexistent/wormhole-themes")));
            let members: Rc<[usize]> = catalog::cards(&entries)
                .into_iter()
                .find(|card| entries[card[0]].card_label() == "Soltari")
                .expect("Soltari is a family")
                .into();
            let picks = super::Picks {
                dark: "Tokyo Night",
                light: "Catppuccin Latte",
                shown: "Tokyo Night",
                mode: None,
            };
            let look = picks.look(&entries, &members);
            let ui = ui(cx);
            cx.new(|_| ThemeCard { catalog: entries, card: 0, members, look, ui })
        });
        let handle = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Natural(card)))
                .expect("opens the window")
                .0
        });
        let picks = |cx: &mut TestAppContext| {
            cx.update(|cx| {
                let s = &cx.global::<SettingsStore>().settings;
                (s.theme.clone(), s.theme_light.clone())
            })
        };
        let click = |cx: &mut TestAppContext, x: f32, y: f32| {
            cx.update(|cx| {
                handle
                    .update(cx, |_, window, cx| {
                        window.render_frame(cx);
                        window.click_at("card-box", gpui_kit::point(px(x), px(y)), cx);
                    })
                    .expect("open")
            });
        };
        // The strip's right end holds the two swatches, 24 wide each inside its 12 of padding:
        // Light, then Dark.
        let height = f32::from(card_height(px(13.0)));
        click(cx, 280.0 - 12.0 - 24.0 - 12.0, height - 14.0);
        assert_eq!(picks(cx), (DEFAULT_THEME.into(), "Soltari Light".into()), "the light pick only");
        click(cx, 40.0, 30.0);
        assert_eq!(picks(cx), ("Soltari Dark".into(), "Soltari Light".into()), "both picks");
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
            let look = super::CardLook { active: true, dark_pick: true, ..super::CardLook::default() };
            cx.new(|_| ThemeCard { catalog: entries, card: 0, members: Rc::from(vec![0]), look, ui })
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
