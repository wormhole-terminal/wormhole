//! One card of the theme picker: a miniature of the app in a theme's own colors with its name
//! in a strip under it. A card stands for one theme; a click makes it the pick of its kind. Under
//! the pointer a card that isn't a pick offers to hide its theme, and its menu (a right click) does
//! too, with Move to Trash for a theme the person imported.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::registry::ThemeId;
use wormhole_core::theme::tokens::{Appearance, ThemeTokens};

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::card_height;
use crate::platform::{self, menu::Item, menu::Menu};
use crate::settings::UiColors;
use crate::settings::theme_trash;
use crate::settings::{settings, update_settings};
use crate::style;
use crate::theme_bridge::hsla;
use crate::ui::{Half, Look, ThemeCardProps};

const CARD_GROUP: &str = "theme-card";

/// One card: a theme, or a dark and light pair.
pub(crate) struct ThemeCard {
    pub(super) catalog: Rc<Vec<ThemeEntry>>,
    /// The themes the card stands for, by index into the catalog: one, or a dark and a light one. The
    /// first is also the card's element id.
    pub(super) halves: Vec<usize>,
    /// What the strip says: the family's name when the shelf holds one version of it, else
    /// the version's own.
    pub(super) label: SharedString,
    /// Every theme of the card is the pick of its kind.
    pub(super) picked: bool,
    pub(super) ui: UiColors,
}

// How many times cards were drawn, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    pub(super) static CARD_RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Leave the themes `values` out of the picker.
fn hide(values: Vec<String>, cx: &mut App) {
    update_settings(cx, move |s| {
        for value in &values {
            s.set_theme_hidden(value, true);
        }
    });
}

/// Make the themes the picks of their kinds: a dark one is the dark pick, a light one the light pick.
fn pick(themes: Vec<(String, Appearance)>, cx: &mut App) {
    update_settings(cx, move |s| {
        for (value, appearance) in themes {
            match appearance {
                Appearance::Dark => s.theme = value,
                Appearance::Light => s.theme_light = value,
            }
        }
    });
}

/// The file a theme the person imported lives in; `None` for a built-in.
fn file_of(entry: &ThemeEntry) -> Option<PathBuf> {
    match &entry.id {
        ThemeId::File(path) | ThemeId::Variant(path, _) => Some(path.clone()),
        ThemeId::Builtin(_) => None,
    }
}

/// The card's menu: hide the theme, and for one the person imported, move its file to the Trash
/// (after asking; a pick that names it falls back to the default of its appearance).
fn card_menu(entries: &[&ThemeEntry], picked: bool) -> Menu {
    let values: Vec<String> = entries.iter().map(|e| e.value.clone()).collect();
    let mut menu = Menu::new().item(
        Item::new(text(SETTINGS_APPEARANCE_HIDE_THEME))
            .enabled(!picked)
            .on_select(move |_, cx| hide(values.clone(), cx)),
    );
    // A pair is two themes: only a single theme's file is offered for the Trash.
    if let [entry] = entries
        && let Some(path) = file_of(entry)
    {
        let entry = (*entry).clone();
        menu = menu.separator().item(
            Item::new(text(SETTINGS_APPEARANCE_MOVE_TO_TRASH))
                .on_select(move |window, cx| ask_trash(&entry, &path, window, cx)),
        );
    }
    menu
}

fn ask_trash(entry: &ThemeEntry, path: &Path, window: &mut Window, cx: &mut App) {
    let which = theme_trash::affected(settings(cx), &|v| entry.is_selected_by(v));
    let answer = theme_trash::ask(&entry.label, which, false, window, cx);
    let (entry, path) = (entry.clone(), path.to_path_buf());
    window
        .spawn(cx, async move |cx| {
            if !answer.await {
                return;
            }
            cx.update(|_, cx| {
                let selects = |v: &str| entry.is_selected_by(v);
                match theme_trash::trash(cx, &path, &selects, platform::trash::move_to_trash) {
                    Ok(()) => cx.refresh_windows(),
                    Err(error) => tracing::warn!(
                        "theme picker: moving {} to the Trash failed: {error:#}",
                        path.display()
                    ),
                }
            })
            .ok();
        })
        .detach();
}

impl Render for ThemeCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        CARD_RENDERS.with(|n| n.set(n.get() + 1));
        let entries: Vec<&ThemeEntry> = self.halves.iter().map(|&ix| &self.catalog[ix]).collect();
        let (ui, picked) = (self.ui, self.picked);
        let first = self.halves[0];
        let picks: Vec<(String, Appearance)> =
            entries.iter().map(|e| (e.value.clone(), e.tokens.appearance)).collect();
        let hide_values: Vec<String> = entries.iter().map(|e| e.value.clone()).collect();
        let menu_halves = self.halves.clone();
        let look = Look::of(cx);
        let face = ThemeCardProps {
            id: ("theme-card", first).into(),
            spoken: self.label.clone(),
            name: self.label.clone(),
            mark: "✓".into(), // text: a check mark, not a word
            on: picked,
            on_pick: Some(Rc::new(move |_, _, cx| pick(picks.clone(), cx))),
            halves: entries.iter().map(|e| half(&e.tokens)).collect(),
        }
        .render(&look);
        // The card is a face the design draws; what is added here is behaviour: the right-click menu, the
        // hide button under the pointer, and the warning of a theme made with a newer Wormhole.
        let newer = entries.iter().find_map(|e| e.newer);
        div()
            .id(("theme-card-box", first))
            .group(CARD_GROUP)
            .relative()
            .w_full()
            .h(card_height())
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    let entries: Vec<&ThemeEntry> = menu_halves.iter().map(|&ix| &this.catalog[ix]).collect();
                    platform::show_menu(card_menu(&entries, this.picked), event.position, window, cx);
                }),
            )
            .child(face)
            // The theme in use can't be hidden: its card keeps its place while it is a pick.
            .when(!picked, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_1()
                        .right_1()
                        .invisible()
                        .group_hover(CARD_GROUP, |s| s.visible())
                        .child(
                            div().rounded(ui.radius).bg(ui.panel).child(
                                style::icon_button(
                                    ("theme-hide", first),
                                    ui,
                                    IconName::EyeOff,
                                    text(SETTINGS_APPEARANCE_HIDE_THEME),
                                )
                                .on_click(move |_, _, cx| {
                                    // The card under it would choose the theme.
                                    cx.stop_propagation();
                                    hide(hide_values.clone(), cx);
                                }),
                            ),
                        ),
                )
            })
            .children(newer.map(|skipped| {
                let count = i64::try_from(skipped).unwrap_or(i64::MAX);
                let said = settings_appearance_theme_newer(count);
                div()
                    .id(("theme-newer", first))
                    .absolute()
                    .top_1()
                    .left_1()
                    .aria_label(said.clone())
                    .tooltip(move |window, cx| Tooltip::new(said.clone()).build(window, cx))
                    .child(Icon::new(IconName::TriangleAlert).xsmall().text_color(ui.warning))
            }))
    }
}

/// A theme's thumbnail in the design's terms: the theme's own window colour, a strip for the sidebar
/// that is a shade darker or lighter by the window's lightness, and five bars in its terminal colours.
fn half(tokens: &ThemeTokens) -> Half {
    let bg = hsla(tokens.terminal.background);
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let light = bg.l > 0.5;
    let shade = |alpha: f32| Hsla { h: 0.0, s: 0.0, l: if light { 0.0 } else { 1.0 }, a: alpha };
    Half {
        bg,
        strip: shade(if light { 0.04 } else { 0.03 }),
        strip_line: shade(0.18),
        strip_rule: shade(if light { 0.08 } else { 0.06 }),
        c1: ansi(4),
        c2: ansi(2),
        c3: ansi(3),
        c4: hsla(tokens.terminal.foreground),
        c5: ansi(1),
    }
}
