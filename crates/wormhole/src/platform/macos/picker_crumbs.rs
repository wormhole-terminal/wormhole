//! The picker's breadcrumb: borderless buttons for the steps of where the person is, with a chevron between
//! them. A path too long for the panel drops its start, which an ellipsis stands for.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{
    NSAccessibility as _, NSButton, NSColor, NSFont, NSImageScaling, NSImageView, NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use wormhole_core::view::picker::Crumb;

use super::appkit::symbol;
use super::picker_views::Target;

/// How tall the breadcrumb is.
pub const HEIGHT: f64 = 24.0;
/// What the breadcrumb leaves out at its start, and between steps.
const ELLIPSIS_W: f64 = 16.0;
const SEPARATOR_W: f64 = 12.0;

/// Lay `crumbs` out in `view`, `room` wide, again from nothing.
pub fn fill(view: &NSView, crumbs: &[Crumb], target: &Target, room: f64) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    for old in view.subviews().iter() {
        old.removeFromSuperview();
    }
    let buttons: Vec<Retained<NSButton>> = crumbs
        .iter()
        .enumerate()
        .map(|(at, crumb)| button(mtm, &crumb.label, at, at + 1 == crumbs.len(), target))
        .collect();
    let widths: Vec<f64> = buttons.iter().map(|b| b.frame().size.width).collect();
    let first = first_crumb(&widths, room);
    let mut x = 0.0;
    if first > 0 {
        let dots = NSTextField::labelWithString(&NSString::from_str("…"), mtm);
        dots.setFont(Some(&NSFont::systemFontOfSize(12.0)));
        dots.setTextColor(Some(&NSColor::tertiaryLabelColor()));
        dots.setFrame(NSRect::new(NSPoint::new(x, 3.0), NSSize::new(ELLIPSIS_W, 17.0)));
        view.addSubview(&dots);
        x += ELLIPSIS_W;
    }
    for (at, button) in buttons.iter().enumerate().skip(first) {
        if at > first {
            let chevron = NSImageView::initWithFrame(mtm.alloc(), NSRect::ZERO);
            chevron.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
            chevron.setImage(symbol("chevron.right", "").as_deref());
            chevron.setContentTintColor(Some(&NSColor::tertiaryLabelColor()));
            chevron.setFrame(NSRect::new(NSPoint::new(x + 2.0, 7.0), NSSize::new(8.0, 10.0)));
            chevron.setAccessibilityElement(false);
            view.addSubview(&chevron);
            x += SEPARATOR_W;
        }
        let size = button.frame().size;
        button.setFrame(NSRect::new(NSPoint::new(x, ((HEIGHT - size.height) / 2.0).floor()), size));
        view.addSubview(button);
        x += size.width;
    }
}

/// A borderless button for a step: the current one in the text colour, the others quieter.
fn button(
    mtm: MainThreadMarker,
    label: &str,
    at: usize,
    current: bool,
    target: &Target,
) -> Retained<NSButton> {
    // SAFETY: `crumb:` is defined on `Target` with the `(id)sender` signature of an action; the panel keeps
    // the target alive, and AppKit holds it weakly.
    #[allow(unsafe_code)]
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(label),
            Some(target),
            Some(sel!(crumb:)),
            mtm,
        )
    };
    button.setBordered(false);
    button.setFont(Some(&NSFont::systemFontOfSize(12.0)));
    button.setTag(at as isize);
    let tint = if current { NSColor::labelColor() } else { NSColor::secondaryLabelColor() };
    button.setContentTintColor(Some(&tint));
    button.sizeToFit();
    button
}

/// Which step the breadcrumb starts at so the steps from it fit in `room`: all of them when they do, else the
/// last ones with room for the ellipsis that stands for the rest (and always at least the last).
pub fn first_crumb(widths: &[f64], room: f64) -> usize {
    for first in 0..widths.len() {
        let steps = widths[first..].iter().sum::<f64>() + SEPARATOR_W * (widths.len() - first - 1) as f64;
        let dots = if first > 0 { ELLIPSIS_W } else { 0.0 };
        if steps + dots <= room {
            return first;
        }
    }
    widths.len().saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::{ELLIPSIS_W, SEPARATOR_W, first_crumb};

    #[test]
    fn a_breadcrumb_that_fits_starts_at_its_first_step_and_a_long_one_drops_its_start() {
        let widths = [20.0, 60.0, 80.0, 70.0];
        assert_eq!(first_crumb(&widths, 400.0), 0);
        let all = 20.0 + 60.0 + 80.0 + 70.0 + 3.0 * SEPARATOR_W;
        assert_eq!(
            first_crumb(&widths, all - 1.0),
            1,
            "the ellipsis takes {ELLIPSIS_W} of what the first gave back"
        );
        assert_eq!(first_crumb(&widths, 10.0), 3, "at least the last step");
        assert_eq!(first_crumb(&[], 10.0), 0);
    }
}
