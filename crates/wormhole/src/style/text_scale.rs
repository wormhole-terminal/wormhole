//! The text size of the Settings and Theme Studio windows: the one place that turns the person's step
//! (`settings_text_size`) into a size.
//!
//! The roles in [`super::TextRole`] and the spacing in `_1`..`_6` are GPUI's rems, so a window's text and
//! spacing follow one number: its rem size, which `theme_bridge::rem_size` derives from the interface font.
//! A step scales that number for the two windows and nothing else (the main window and the terminal never
//! call [`apply`]), so a larger step grows text and the space between it together, as the Interface font's
//! size does for the whole app. Heights fixed in points (`CONTROL_H`, `ROW_H`) stay as they are; the steps
//! stop where the text still fits them.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;
use wormhole_core::config::TextSize;

use crate::settings::settings;

/// The factor of each step, on the window's rem size. Eighths, so the caption role (12 pt) lands on 10.5,
/// 12, 13.5 and 15 pt and the body role (14) on 12.25, 14, 15.75 and 17.5: the same ratio between the
/// roles at every step. `Default` is exactly 1, so nobody who never touches the setting sees a change.
/// The top step is where body text still fits the 28 pt control and row heights without growing them
/// (a line of 17.5 pt text is 28.3 pt tall); beyond it those heights would have to scale too.
pub const SMALL: f32 = 0.875;
pub const DEFAULT: f32 = 1.0;
pub const LARGE: f32 = 1.125;
pub const LARGER: f32 = 1.25;

/// What `size` multiplies the window's rem size by.
pub const fn factor(size: TextSize) -> f32 {
    match size {
        TextSize::Small => SMALL,
        TextSize::Default => DEFAULT,
        TextSize::Large => LARGE,
        TextSize::Larger => LARGER,
    }
}

/// Set this window's rem size for the person's step. Called first in a window's `render`: the Kit's root
/// has just set the theme's size, and what is laid out after this call (the whole window) takes ours.
pub fn apply(window: &mut Window, cx: &App) {
    let rem = cx.theme().font_size * factor(settings(cx).settings_text_size);
    if window.rem_size() != rem {
        window.set_rem_size(rem);
    }
}

/// A width in points that holds text (a column of lists and chips), grown with the step so the text in it
/// keeps its room: the Studio's token list and editor. At the default step it is `width` itself.
pub fn scaled(width: f32, cx: &App) -> Pixels {
    px(width * factor(settings(cx).settings_text_size))
}

#[cfg(test)]
mod tests {
    use wormhole_core::config::TextSize;

    use super::factor;

    #[test]
    fn the_default_step_changes_nothing() {
        assert!((factor(TextSize::Default) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_steps_grow_in_the_order_of_the_menu() {
        let factors: Vec<f32> = TextSize::ALL.iter().map(|size| factor(*size)).collect();
        assert!(factors.windows(2).all(|pair| pair[0] < pair[1]), "{factors:?}");
    }
}
