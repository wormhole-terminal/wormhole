//! Where everything sits inside the notice chip's capsule: the symbol, the words, and, on a warning that
//! stays, the ⓘ and × buttons at the trailing end. Plain numbers in the label's own space (the capsule's
//! bottom left is the origin, as AppKit's views are, not flipped), so the rules are tested without a window.
//! The capsule grows by the buttons, and a capsule too narrow for everything cuts the words first.

use objc2_foundation::{NSPoint, NSRect, NSSize};
use wormhole_core::view::notice::Buttons;

/// The chip's own metrics, AppKit's native ones for a small capsule (the app's style scale is for what
/// GPUI draws; this is system chrome, like a menu row in `rows`).
pub(super) const HEIGHT: f64 = 34.0;
/// Beside the symbol and the words.
pub(super) const PAD: f64 = 14.0;
pub(super) const ICON: f64 = 15.0;
pub(super) const GAP: f64 = 7.0;
/// A small button's square, which is its click target and the circle that lights up under the pointer, and
/// the size of the picture in it (AppKit's small control size, as the popovers' buttons).
pub(super) const BUTTON: f64 = 20.0;
pub(super) const BUTTON_ICON: f64 = 11.0;
/// Between the two buttons.
const BUTTON_GAP: f64 = 2.0;
/// Past the last button: less than `PAD`, because its circle already has room round the picture.
const END_PAD: f64 = 8.0;

/// Where everything sits in a capsule `chip_w` wide, for words `text` big.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Geometry {
    pub(super) icon: NSRect,
    pub(super) text: NSRect,
    /// The squares of the buttons that are there.
    pub(super) info: Option<NSRect>,
    pub(super) close: Option<NSRect>,
}

/// What the capsule needs beside its words.
pub(super) fn fixed(buttons: Buttons) -> f64 {
    let count = buttons.count();
    if count == 0 {
        return PAD + ICON + GAP + PAD;
    }
    let squares = count as f64 * BUTTON + (count - 1) as f64 * BUTTON_GAP;
    PAD + ICON + GAP + GAP + squares + END_PAD
}

pub(super) fn geometry(text: NSSize, chip_w: f64, buttons: Buttons) -> Geometry {
    // A capsule too narrow for them cuts the words (the label truncates the tail); the symbol and the
    // buttons stay.
    let words = text.width.min((chip_w - fixed(buttons)).max(0.0)).ceil();
    let centred = |side: f64| ((HEIGHT - side) / 2.0).floor();
    let square = |x: f64| NSRect::new(NSPoint::new(x, centred(BUTTON)), NSSize::new(BUTTON, BUTTON));
    // From the end inward: × last, ⓘ before it.
    let mut right = chip_w - END_PAD;
    let mut button = |present: bool| {
        present.then(|| {
            right -= BUTTON;
            let at = square(right);
            right -= BUTTON_GAP;
            at
        })
    };
    let close = button(buttons.close);
    let info = button(buttons.info);
    Geometry {
        icon: NSRect::new(NSPoint::new(PAD, centred(ICON)), NSSize::new(ICON, ICON)),
        text: NSRect::new(
            NSPoint::new(PAD + ICON + GAP, centred(text.height)),
            NSSize::new(words, text.height.ceil()),
        ),
        info,
        close,
    }
}

/// A button's picture: `BUTTON_ICON` square in the middle of its `square`.
pub(super) fn picture(square: NSRect) -> NSRect {
    let inset = ((BUTTON - BUTTON_ICON) / 2.0).floor();
    NSRect::new(
        NSPoint::new(square.origin.x + inset, square.origin.y + inset),
        NSSize::new(BUTTON_ICON, BUTTON_ICON),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Buttons = Buttons::NONE;
    const BOTH: Buttons = Buttons { info: true, close: true, act: false };
    const CROSS: Buttons = Buttons { info: false, close: true, act: false };

    fn words(width: f64) -> NSSize {
        NSSize::new(width, 16.0)
    }

    #[test]
    fn the_words_get_what_the_capsule_has_beside_the_symbol() {
        let g = geometry(words(80.0), fixed(NONE) + 80.0, NONE);
        assert_eq!(g.text.size.width, 80.0);
        assert!(g.info.is_none() && g.close.is_none());
    }

    #[test]
    fn the_symbol_and_the_words_sit_in_the_middle_of_the_height() {
        let g = geometry(words(80.0), fixed(NONE) + 80.0, NONE);
        assert_eq!(g.icon.origin.y, ((HEIGHT - ICON) / 2.0).floor());
        assert_eq!(g.text.origin.y, ((HEIGHT - 16.0) / 2.0).floor());
        assert!(g.icon.origin.x + g.icon.size.width + GAP <= g.text.origin.x + 1e-9);
    }

    #[test]
    fn words_wider_than_the_capsule_are_cut_and_the_symbol_stays() {
        let g = geometry(words(2000.0), 300.0, NONE);
        assert_eq!(g.text.size.width, 300.0 - fixed(NONE));
        assert_eq!(g.icon.size.width, ICON);
    }

    #[test]
    fn a_capsule_with_no_room_for_words_has_none() {
        let g = geometry(words(50.0), 0.0, NONE);
        assert_eq!(g.text.size.width, 0.0);
    }

    #[test]
    fn the_capsule_grows_by_each_button() {
        assert!(fixed(CROSS) > fixed(NONE));
        assert_eq!(fixed(BOTH) - fixed(CROSS), BUTTON + BUTTON_GAP);
    }

    #[test]
    fn the_buttons_sit_at_the_end_the_cross_last_and_the_words_end_before_them() {
        let w = fixed(BOTH) + 120.0;
        let g = geometry(words(120.0), w, BOTH);
        let (info, close) = (g.info.expect("info"), g.close.expect("close"));
        assert_eq!(close.origin.x + close.size.width + END_PAD, w, "the cross ends the capsule");
        assert_eq!(info.origin.x + info.size.width + BUTTON_GAP, close.origin.x);
        assert_eq!(close.origin.y, ((HEIGHT - BUTTON) / 2.0).floor());
        assert!(g.text.origin.x + g.text.size.width + GAP <= info.origin.x + 1e-9, "the words end before ⓘ");
    }

    #[test]
    fn a_lone_cross_is_at_the_end_too() {
        let w = fixed(CROSS) + 60.0;
        let g = geometry(words(60.0), w, CROSS);
        assert!(g.info.is_none());
        assert_eq!(g.close.map(|c| c.origin.x + c.size.width + END_PAD), Some(w));
    }

    #[test]
    fn a_narrow_capsule_cuts_the_words_before_it_squeezes_the_buttons() {
        let w = fixed(BOTH) + 40.0;
        let g = geometry(words(500.0), w, BOTH);
        assert_eq!(g.text.size.width, 40.0, "only the words give way");
        assert_eq!(g.close.map(|c| c.size.width), Some(BUTTON));
        assert_eq!(g.info.map(|c| c.size.width), Some(BUTTON));
    }

    #[test]
    fn a_picture_is_centred_in_its_square() {
        let square = NSRect::new(NSPoint::new(100.0, 7.0), NSSize::new(BUTTON, BUTTON));
        let p = picture(square);
        assert_eq!(p.size.width, BUTTON_ICON);
        assert!(p.origin.x > square.origin.x && p.origin.x + p.size.width < square.origin.x + BUTTON);
    }
}
