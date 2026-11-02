//! The settings nav's profile card, and the tile colour of its rows. The rows themselves, a tile and name for
//! each page and under the open page a sub-row for each of its groups, are the `SettingsNav` component
//! (`ui/settings_nav.dc.html`), which is also the design; the window gives it its entries.
//!
//! The card is a tab stop and a button for VoiceOver, named by what it says: the keys reach it through the
//! window's own focus order.

use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::labels::{badge, count_badge};
use crate::style::scale::{Edges, NAV_STEP, SWATCH, TextRole};
use crate::theme_bridge;
use wormhole_core::theme::tokens::Color;

/// How alike two colours may be before a tile on the selection would vanish into it (their contrast ratio).
const TILE_SAME: f32 = 1.5;
/// The white wash a tile takes on the selection when its own colour is the selection's.
const TILE_WASH: f32 = 0.25;

/// The keyboard ring of a row, drawn inside the nav's inset. On the selection, which is the primary and so
/// the ring's own colour, it is the text colour instead.
fn focus_ring<E: StatefulInteractiveElement>(row: E, ui: UiColors, selected: bool) -> E {
    let color = if selected { ui.text } else { ui.focus };
    row.focus_visible(move |d| d.ringed(ui, color, ui.focus_ring_width))
}

/// What a tile is filled with: its colour, unless the row is the selection and the tile is nearly the
/// selection's own colour (the Appearance tile on the default primary), when it is a wash of the icon's
/// white so it still reads as a tile.
pub fn nav_tile_fill(ui: UiColors, tile: Hsla, selected: bool) -> Hsla {
    let alike =
        Color::contrast_ratio(theme_bridge::color(tile), theme_bridge::color(ui.nav_selected)) < TILE_SAME;
    if selected && alike { ui.tile_mark_brand.opacity(TILE_WASH) } else { tile }
}

/// The count of a search on the selected row: a wash of the text colour, so it reads on the primary.
fn selected_count(ui: UiColors, text: impl Into<SharedString>) -> Div {
    badge(ui)
        .px_1()
        .bg(ui.on_primary.opacity(TILE_WASH))
        .text_color(ui.on_primary)
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

/// The profile of the nav's card: its colour and letter, its name and the caption under it.
pub struct NavProfile {
    pub color: Hsla,
    pub letter: char,
    pub name: SharedString,
    pub caption: SharedString,
    pub selected: bool,
    /// The count a search found on its page.
    pub hits: Option<usize>,
}

/// The profile card above the nav's rows: its avatar, its name and "Profile" under it, for the page of the
/// profile Wormhole is in. Selected, it is the primary like a row, its avatar a wash of white. Add
/// `.on_click(…)`.
#[track_caller]
pub fn nav_profile_row(id: impl Into<ElementId>, ui: UiColors, profile: NavProfile) -> Stateful<Div> {
    let NavProfile { color, letter, name, caption, selected, hits } = profile;
    let (title, quiet) = if selected { (ui.on_primary, ui.on_primary) } else { (ui.text, ui.muted) };
    let avatar = if selected {
        div()
            .flex_none()
            .size(px(SWATCH))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(ui.tile_mark_brand.opacity(TILE_WASH))
            .text_color(ui.on_primary)
            .text_body()
            .font_weight(FontWeight::BOLD)
            .child(letter.to_uppercase().to_string())
            .into_any_element()
    } else {
        crate::style::profile_avatar(ui, color, letter, SWATCH).into_any_element()
    };
    let row = div()
        .id(id)
        .role(Role::Button)
        .aria_label(SharedString::from(format!("{caption}: {name}")))
        .aria_selected(selected)
        .focusable()
        .tab_stop(true)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(SWATCH + NAV_STEP * 2.0))
        .px_2()
        .rounded(ui.radius_sm)
        .cursor_pointer()
        .when(selected, |d| d.bg(ui.nav_selected))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(avatar)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().text_label().text_color(title).child(name))
                .child(div().truncate().text_caption().text_color(quiet).child(caption)),
        )
        .children(hits.map(|n| {
            let text = crate::style::scale::count_label(n);
            if selected { selected_count(ui, text) } else { count_badge(ui, text) }
        }));
    focus_ring(row, ui, selected)
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Hsla, rgb};

    use super::nav_tile_fill;
    use crate::settings::UiColors;

    fn colors() -> UiColors {
        UiColors {
            nav_selected: Hsla::from(rgb(0x0096ff)),
            tile_mark_brand: Hsla::from(rgb(0xffffff)),
            ..UiColors::default()
        }
    }

    #[test]
    fn a_tile_keeps_its_colour_on_the_selection_unless_that_is_the_selections_own() {
        let ui = colors();
        let appearance = Hsla::from(rgb(0x0a84ff));
        let git = Hsla::from(rgb(0xff9f0a));
        assert_eq!(nav_tile_fill(ui, git, true), git, "a different colour stays");
        assert_eq!(nav_tile_fill(ui, appearance, false), appearance, "off the selection it stays");
        let washed = nav_tile_fill(ui, appearance, true);
        assert_ne!(washed, appearance, "the blue tile on the blue selection would vanish");
        assert!((washed.a - 0.25).abs() < 1e-6 && washed.l > 0.99, "a wash of the icon's white: {washed:?}");
    }
}
