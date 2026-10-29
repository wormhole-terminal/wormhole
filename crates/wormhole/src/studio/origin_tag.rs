//! The two small marks on a token's row: where its value comes from (yours, base, worked out), and,
//! for a theme with a light and a dark appearance, which of them the value is for (the chip). Each
//! says in its tooltip what it means; the chip's also says what a click does, since nothing else
//! shows that it can be clicked.

use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::*;
use wormhole_core::theme::tokens::appearance::{DARK, LIGHT};
use wormhole_text::keys::{
    STUDIO_APPEARANCE_DARK, STUDIO_APPEARANCE_LIGHT, STUDIO_ORIGIN_BASE, STUDIO_ORIGIN_TIP_WORKED_OUT,
    STUDIO_ORIGIN_TIP_YOURS, STUDIO_ORIGIN_WORKED_OUT, STUDIO_ORIGIN_YOURS, STUDIO_REACH_BOTH,
    STUDIO_REACH_DARK, STUDIO_REACH_LIGHT, studio_origin_tip_base, studio_reach_tip_both,
    studio_reach_tip_this,
};
use wormhole_text::text;

use super::document::places::{Origin, Reach};
use crate::settings::UiColors;
use crate::style::TextRole as _;

/// The word after a token's name that says where its value comes from. `from` names the theme the
/// base was copied from.
pub(super) fn origin_tag(token: &str, origin: Origin, from: &str, color: Hsla) -> Stateful<Div> {
    let word = match origin {
        Origin::Yours => text(STUDIO_ORIGIN_YOURS),
        Origin::Base => text(STUDIO_ORIGIN_BASE),
        Origin::WorkedOut => text(STUDIO_ORIGIN_WORKED_OUT),
    };
    let tip: SharedString = origin_tip(origin, from).into();
    div()
        .id(SharedString::from(format!("studio-origin-{token}")))
        .flex_none()
        .text_caption()
        .text_color(color)
        .child(word)
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
}

/// What the origin tag's tooltip says.
pub(super) fn origin_tip(origin: Origin, from: &str) -> String {
    match origin {
        Origin::Yours => text(STUDIO_ORIGIN_TIP_YOURS).to_string(),
        Origin::Base => studio_origin_tip_base(from),
        Origin::WorkedOut => text(STUDIO_ORIGIN_TIP_WORKED_OUT).to_string(),
    }
}

/// The chip of a row in a pair: "both", or the appearance (`shown`, its name in the file) the value
/// is for alone. Clicking it moves the value to the other.
pub(super) fn reach_chip(
    token: &str,
    reach: Reach,
    shown: &str,
    color: Hsla,
    ui: UiColors,
    on_click: impl Fn(String, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let word = reach_word(reach, shown);
    let tip: SharedString = reach_tip(reach, shown).into();
    let token = token.to_string();
    div()
        .id(SharedString::from(format!("studio-reach-{token}")))
        .flex_none()
        .cursor_pointer()
        .child(crate::style::status_badge(ui, color, word))
        .tooltip(move |window, cx| Tooltip::new(tip.clone()).build(window, cx))
        .on_click(move |_, window, cx| on_click(token.clone(), window, cx))
}

/// What the chip says: "both", or the appearance the value is for alone. An appearance that is
/// neither light nor dark (a hand-written file's) is called by its own name.
pub(super) fn reach_word(reach: Reach, shown: &str) -> SharedString {
    match (reach, shown) {
        (Reach::Both, _) => text(STUDIO_REACH_BOTH).into(),
        (Reach::ThisAppearance, DARK) => text(STUDIO_REACH_DARK).into(),
        (Reach::ThisAppearance, LIGHT) => text(STUDIO_REACH_LIGHT).into(),
        (Reach::ThisAppearance, other) => other.to_string().into(),
    }
}

/// What the chip's tooltip says: where the value is now, and where a click moves it.
pub(super) fn reach_tip(reach: Reach, shown: &str) -> String {
    let appearance = match shown {
        LIGHT => text(STUDIO_APPEARANCE_LIGHT).to_string(),
        DARK => text(STUDIO_APPEARANCE_DARK).to_string(),
        other => other.to_string(),
    };
    match reach {
        Reach::Both => studio_reach_tip_both(&appearance),
        Reach::ThisAppearance => studio_reach_tip_this(&appearance),
    }
}

#[cfg(test)]
mod tests {
    use super::{Origin, Reach, origin_tip, reach_tip, reach_word};

    #[test]
    fn a_value_for_one_appearance_is_called_by_that_appearance() {
        assert_eq!(reach_word(Reach::Both, "evening"), "both");
        assert_eq!(reach_word(Reach::ThisAppearance, "light"), "light");
        assert_eq!(reach_word(Reach::ThisAppearance, "dark"), "dark");
        assert_eq!(reach_word(Reach::ThisAppearance, "evening"), "evening", "not \"both\"");
    }

    #[test]
    fn the_tooltips_say_where_a_value_comes_from_and_what_the_chip_does() {
        assert!(origin_tip(Origin::Base, "Nord").contains("Nord"));
        assert!(origin_tip(Origin::Yours, "Nord").contains("itself"));
        assert!(reach_tip(Reach::Both, "dark").contains("Dark only"));
        assert!(reach_tip(Reach::ThisAppearance, "light").starts_with("Only Light"));
        assert!(reach_tip(Reach::ThisAppearance, "evening").contains("evening"));
    }
}
