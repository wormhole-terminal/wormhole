//! One card of the theme picker: a miniature of the app in a theme's own colors with its name
//! in a strip under it. A card stands for one theme; a click makes it the pick of its kind. Under
//! the pointer a card that isn't a pick offers to hide its theme, and its menu (a right click) does
//! too, with Move to Trash for a theme the person imported.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::catalog::ThemeEntry;
use wormhole_core::theme::registry::ThemeId;
use wormhole_core::theme::tokens::ThemeTokens;

#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::card_height;
use crate::platform::{self, menu::Item, menu::Menu};
use crate::settings::UiColors;
use crate::settings::picks::set_pick;
use crate::settings::update_settings;
use crate::style::{self, Edges, TextRole};
use crate::theme_bridge::{hsla, token};

// The preview is a drawing at its own small scale, not a control: its rules and bars do not
// follow the app's metrics (the card shows the previewed theme's colors).
const PREVIEW_RULE: f32 = 1.0;
const PREVIEW_BAR_H: f32 = 3.0;
const PREVIEW_PAD: f32 = 8.0;
const PREVIEW_GAP: f32 = 5.0;
/// The name strip under the miniature.
const STRIP_H: f32 = 28.0;
/// The ring round the card that is its shelf's pick, outside its edge.
const RING_WIDTH: f32 = 2.0;
/// The group the hide button answers to, shown while the pointer is on its card.
const CARD_GROUP: &str = "theme-card";

/// One theme's card.
pub(crate) struct ThemeCard {
    pub(super) catalog: Rc<Vec<ThemeEntry>>,
    /// The theme, by index into the catalog; it is also the card's element id.
    pub(super) entry: usize,
    /// What the strip says: the family's name when the shelf holds one version of it, else
    /// the version's own.
    pub(super) label: SharedString,
    /// It is the pick of its kind.
    pub(super) picked: bool,
    pub(super) ui: UiColors,
}

// How many times cards were drawn, per thread (tests run in parallel).
#[cfg(test)]
thread_local! {
    pub(super) static CARD_RENDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Leave the theme `value` out of the picker.
fn hide(value: &str, cx: &mut App) {
    let value = value.to_string();
    update_settings(cx, move |s| {
        s.set_theme_hidden(&value, true);
    });
}

/// The file a theme the person imported lives in; `None` for a built-in.
fn file_of(entry: &ThemeEntry) -> Option<PathBuf> {
    match &entry.id {
        ThemeId::File(path) | ThemeId::Variant(path, _) => Some(path.clone()),
        ThemeId::Builtin(_) => None,
    }
}

/// The card's menu: hide the theme, and for one the person imported, move its file to the Trash.
fn card_menu(entry: &ThemeEntry, picked: bool) -> Menu {
    let value = entry.value.clone();
    let mut menu = Menu::new().item(
        Item::new(text(SETTINGS_APPEARANCE_HIDE_THEME))
            .enabled(!picked)
            .on_select(move |_, cx| hide(&value, cx)),
    );
    if let Some(path) = file_of(entry) {
        menu = menu.separator().item(Item::new(text(SETTINGS_APPEARANCE_MOVE_TO_TRASH)).on_select(
            move |_, _| {
                if let Err(error) = platform::trash::move_to_trash(&path) {
                    tracing::warn!("theme picker: moving {} to the Trash failed: {error:#}", path.display());
                }
            },
        ));
    }
    menu
}

impl Render for ThemeCard {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(test)]
        CARD_RENDERS.with(|n| n.set(n.get() + 1));
        let entry = &self.catalog[self.entry];
        let (ui, picked) = (self.ui, self.picked);
        let tokens = &entry.tokens;
        let t = |name: &str| token(tokens, name);
        let (value, appearance) = (entry.value.clone(), tokens.appearance);
        let name_color = if picked { t("primary") } else { t("text.primary") };
        let hide_value = entry.value.clone();
        let menu_entry = self.entry;
        div()
            .id(("theme-card", self.entry))
            .group(CARD_GROUP)
            .relative()
            .w_full()
            .h(card_height(px(0.0)))
            .flex()
            .flex_col()
            .rounded(ui.radius_lg)
            .overflow_hidden()
            .cursor_pointer()
            .map(
                |d| if picked { d.ringed(ui, ui.primary, px(RING_WIDTH)) } else { d.outlined(ui, ui.border) },
            )
            .when(!picked, |d| d.hover(|d| d.border_color(ui.border_strong)))
            .on_click(cx.listener(move |_, _, _, cx| {
                let value = value.clone();
                update_settings(cx, |s| set_pick(s, value, appearance));
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    let menu = card_menu(&this.catalog[menu_entry], this.picked);
                    platform::show_menu(menu, event.position, window, cx);
                }),
            )
            .child(preview(tokens))
            // The theme in use can't be hidden: its card keeps its place while it is a pick.
            .when(!picked, |d| {
                d.child(
                    div()
                        .absolute()
                        .top_2()
                        .right_2()
                        .invisible()
                        .group_hover(CARD_GROUP, |s| s.visible())
                        .child(
                            div().rounded(ui.radius).bg(ui.panel).outlined(ui, ui.border).child(
                                style::icon_button(
                                    ("theme-hide", self.entry),
                                    ui,
                                    IconName::EyeOff,
                                    text(SETTINGS_APPEARANCE_HIDE_THEME),
                                )
                                .on_click(move |_, _, cx| {
                                    // The card under it would choose the theme.
                                    cx.stop_propagation();
                                    hide(&hide_value, cx);
                                }),
                            ),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(px(STRIP_H))
                    .flex_none()
                    .px_3()
                    .bg(solid(tokens, "surface.panel"))
                    .rule_top(UiColors { border: t("border"), ..ui })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_label()
                            .font_weight(if picked { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .text_color(name_color)
                            .child(self.label.clone()),
                    )
                    // Made with a newer Wormhole: it loads what this version knows.
                    .children(entry.newer.map(|skipped| {
                        let count = i64::try_from(skipped).unwrap_or(i64::MAX);
                        let said = settings_appearance_theme_newer(count);
                        div()
                            .id(("theme-newer", self.entry))
                            .flex_none()
                            .aria_label(said.clone())
                            .tooltip(move |window, cx| Tooltip::new(said.clone()).build(window, cx))
                            .child(Icon::new(IconName::TriangleAlert).xsmall().text_color(ui.warning))
                    }))
                    .when(picked, |d| {
                        d.child(Icon::new(IconName::Check).xsmall().text_color(t("primary")).flex_none())
                    }),
            )
    }
}

/// A token of the card's theme on the theme's own window color (surfaces may carry alpha for
/// a translucent window; a card is opaque).
fn solid(tokens: &ThemeTokens, name: &str) -> Hsla {
    match (tokens.get(name), tokens.get("surface.window").map(|c| c.with_alpha(1.0))) {
        (Some(color), Some(window)) => hsla(color.over(window)),
        _ => token(tokens, name),
    }
}

/// A miniature of the app in `tokens`: the sidebar with three rows, the first one selected, and
/// the terminal with lines in the theme's colors.
fn preview(tokens: &ThemeTokens) -> AnyElement {
    let t = |name: &str| token(tokens, name);
    let ansi = |i: usize| hsla(tokens.terminal.ansi[i]);
    let bar =
        |color: Hsla, width: f32| div().h(px(PREVIEW_BAR_H)).w(relative(width)).rounded_full().bg(color);
    let sidebar = div()
        .w(relative(0.28))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .p(px(PREVIEW_PAD))
        .bg(solid(tokens, "sidebar.background"))
        .border_r(px(PREVIEW_RULE))
        .border_color(t("border"))
        .child(div().h(px(PREVIEW_BAR_H * 2.0)).w(relative(0.9)).rounded_full().bg(t("sidebar.row.selected")))
        .child(bar(t("text.muted"), 0.6))
        .child(bar(t("text.muted"), 0.75));
    let terminal = div()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex()
        .flex_col()
        .gap(px(PREVIEW_GAP))
        .p(px(PREVIEW_PAD))
        .bg(hsla(tokens.terminal.background))
        .child(bar(t("primary"), 0.55))
        .child(bar(ansi(2), 0.8))
        .child(bar(ansi(3), 0.45))
        .child(bar(hsla(tokens.terminal.foreground), 0.7))
        .child(bar(ansi(1), 0.35));
    div().flex_1().min_h_0().w_full().flex().child(sidebar).child(terminal).into_any_element()
}
