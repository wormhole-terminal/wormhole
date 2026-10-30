//! One line of the picker's list as AppKit views, and how its parts are laid out: a header, a note, or a row with
//! an icon, its words (the matched letters in a heavier weight), grey detail, a badge and a chevron.
//!
//! The panel keeps a few of these and fills them again as the list moves, so a list of hundreds costs the
//! views of the lines in view.

use std::ops::Range;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread as _, MainThreadMarker};
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityRowRole, NSBox, NSBoxType, NSColor, NSFont, NSFontAttributeName,
    NSFontManager, NSFontTraitMask, NSFontWeightSemibold, NSForegroundColorAttributeName, NSImageScaling,
    NSImageView, NSLineBreakMode, NSTextField, NSTitlePosition,
};
use objc2_foundation::{NSMutableAttributedString, NSPoint, NSRect, NSSize, NSString};
use wormhole_core::view::picker::IndicatorTone;

use super::appkit::symbol;
use super::picker_views::{RowView, units};
use crate::platform::picker::{Events, Line, MAX_INDICATORS, ROW};

/// Inside a row: from its edges, between its parts, and the sizes of its icon and chevron.
const PAD: f64 = 10.0;
const GAP: f64 = 8.0;
const ICON: f64 = 16.0;
const CHEVRON: f64 = 10.0;
/// The selection is a rounded bar a little in from the list's edges.
const SELECTION_RADIUS: f64 = 8.0;
const BADGE_PAD: f64 = 6.0;
const BADGE_H: f64 = 16.0;
const INDICATOR_SIZE: f64 = 14.0;
const INDICATOR_GAP: f64 = 4.0;

fn indicator_space(count: usize) -> f64 {
    count.min(MAX_INDICATORS) as f64 * (INDICATOR_SIZE + INDICATOR_GAP)
}

/// Where the parts of a row go, in the row's own space (from its top left; the row is flipped).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frames {
    pub icon: NSRect,
    pub label: NSRect,
    pub detail: NSRect,
    pub badge: Option<NSRect>,
    pub chevron: Option<NSRect>,
}

/// Lay out a row `width` wide given how wide each part wants to be. The chevron and the badge keep the right
/// edge; the words get what is left, the label before the detail, and each is cut at the tail when it has to be.
pub fn frames(width: f64, label: f64, detail: f64, badge: Option<f64>, chevron: bool) -> Frames {
    let centred = |side: f64| ((ROW as f64 - side) / 2.0).floor();
    let mut right = width - PAD;
    let chevron_at = chevron.then(|| {
        right -= CHEVRON;
        let at = NSRect::new(NSPoint::new(right, centred(CHEVRON)), NSSize::new(CHEVRON, CHEVRON));
        right -= GAP;
        at
    });
    let badge_at = badge.map(|w| {
        let w = w + 2.0 * BADGE_PAD;
        right -= w;
        let at = NSRect::new(NSPoint::new(right, centred(BADGE_H)), NSSize::new(w, BADGE_H));
        right -= GAP;
        at
    });
    let left = PAD + ICON + GAP;
    let room = (right - left).max(0.0);
    let label_w = label.min(room).ceil();
    let after = (room - label_w - GAP).max(0.0);
    let detail_w = if detail > 0.0 { detail.min(after).ceil() } else { 0.0 };
    let line = |x: f64, w: f64| NSRect::new(NSPoint::new(x, centred(17.0)), NSSize::new(w, 17.0));
    Frames {
        icon: NSRect::new(NSPoint::new(PAD, centred(ICON)), NSSize::new(ICON, ICON)),
        label: line(left, label_w),
        detail: line(left + label_w + GAP, detail_w),
        badge: badge_at,
        chevron: chevron_at,
    }
}

/// A row, as views.
pub struct Slot {
    pub view: Retained<RowView>,
    selection: Retained<NSBox>,
    icon: Retained<NSImageView>,
    label: Retained<NSTextField>,
    detail: Retained<NSTextField>,
    badge: Retained<NSBox>,
    badge_text: Retained<NSTextField>,
    chevron: Retained<NSImageView>,
    indicators: Vec<Retained<NSImageView>>,
}

fn text(mtm: MainThreadMarker) -> Retained<NSTextField> {
    let field = NSTextField::labelWithString(&NSString::from_str(""), mtm);
    field.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
    field.setFont(Some(&NSFont::systemFontOfSize(13.0)));
    field
}

fn picture(mtm: MainThreadMarker) -> Retained<NSImageView> {
    let image = NSImageView::initWithFrame(mtm.alloc(), NSRect::ZERO);
    image.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
    image
}

impl Slot {
    /// The actual AppKit font applied to this row's label, for native-panel inspection.
    #[cfg(feature = "devctl")]
    #[allow(unsafe_code)]
    pub fn font_name(&self) -> Option<String> {
        let text = self.label.attributedStringValue();
        if text.length() == 0 {
            return None;
        }
        // SAFETY: index zero is inside this nonempty string; no effective range is needed.
        let font =
            unsafe { text.attribute_atIndex_effectiveRange(NSFontAttributeName, 0, std::ptr::null_mut()) }?;
        font.downcast_ref::<NSFont>().map(|font| font.fontName().to_string())
    }

    pub fn new(mtm: MainThreadMarker, tx: Events) -> Self {
        let view = RowView::new(mtm, tx);
        // SAFETY: a constant AppKit defines, read on the main thread.
        #[allow(unsafe_code)]
        view.setAccessibilityRole(Some(unsafe { NSAccessibilityRowRole }));

        let selection = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
        selection.setBoxType(NSBoxType::Custom);
        selection.setTitlePosition(NSTitlePosition::NoTitle);
        selection.setBorderWidth(0.0);
        selection.setCornerRadius(SELECTION_RADIUS);
        selection.setContentViewMargins(NSSize::new(0.0, 0.0));
        let badge = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
        badge.setBoxType(NSBoxType::Custom);
        badge.setTitlePosition(NSTitlePosition::NoTitle);
        badge.setBorderWidth(0.0);
        badge.setCornerRadius(4.0);
        badge.setContentViewMargins(NSSize::new(0.0, 0.0));

        let (icon, label, detail, badge_text, chevron) =
            (picture(mtm), text(mtm), text(mtm), text(mtm), picture(mtm));
        let indicators: Vec<_> = (0..MAX_INDICATORS).map(|_| picture(mtm)).collect();
        detail.setFont(Some(&NSFont::systemFontOfSize(12.0)));
        badge_text.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        chevron.setImage(symbol("chevron.right", "").as_deref());
        view.addSubview(&selection);
        view.addSubview(&icon);
        view.addSubview(&label);
        view.addSubview(&detail);
        view.addSubview(&badge);
        view.addSubview(&chevron);
        for indicator in &indicators {
            indicator.setHidden(true);
            view.addSubview(indicator);
        }
        // The words of the badge sit inside its box, which draws the capsule.
        badge.addSubview(&badge_text);
        Self { view, selection, icon, label, detail, badge, badge_text, chevron, indicators }
    }

    /// Show `line` as line number `at` of the list, `width` wide, selected or not.
    pub fn fill(&self, at: usize, line: Line<'_>, selected: bool, width: f64) {
        self.view.show(
            at,
            match line {
                Line::Row(row) => Some(&row.key),
                _ => None,
            },
            matches!(line, Line::Row(row) if row.context_menu),
        );
        self.view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, f64::from(ROW))));
        let (selected_text, quiet) = if selected {
            (
                NSColor::alternateSelectedControlTextColor(),
                NSColor::alternateSelectedControlTextColor().colorWithAlphaComponent(0.7),
            )
        } else {
            (NSColor::labelColor(), NSColor::secondaryLabelColor())
        };
        self.selection.setHidden(!selected);
        self.selection.setFillColor(&NSColor::controlAccentColor());
        self.selection.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, f64::from(ROW))));
        self.view.setAccessibilitySelected(selected);
        for indicator in &self.indicators {
            indicator.setHidden(true);
        }
        match line {
            Line::Header(title) => {
                self.parts(false, false, false);
                self.label.setFont(Some(&NSFont::systemFontOfSize_weight(11.0, semibold())));
                self.label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                self.label.setStringValue(&NSString::from_str(title));
                self.view.setAccessibilityLabel(Some(&NSString::from_str(title)));
                let f = frames(width, 1e6, 0.0, None, false);
                self.label.setFrame(NSRect::new(
                    NSPoint::new(PAD, f.label.origin.y),
                    NSSize::new(width - 2.0 * PAD, 17.0),
                ));
                self.selection.setHidden(true);
            }
            Line::Note(words) => {
                self.parts(false, false, false);
                self.label.setFont(Some(&NSFont::systemFontOfSize(13.0)));
                self.label.setTextColor(Some(&NSColor::secondaryLabelColor()));
                self.label.setStringValue(&NSString::from_str(words));
                self.view.setAccessibilityLabel(Some(&NSString::from_str(words)));
                let f = frames(width, 1e6, 0.0, None, false);
                self.label.setFrame(NSRect::new(
                    NSPoint::new(PAD, f.label.origin.y),
                    NSSize::new(width - 2.0 * PAD, 17.0),
                ));
                self.selection.setHidden(true);
            }
            Line::Row(row) => {
                self.parts(true, row.detail.is_some(), row.badge.is_some());
                self.chevron.setHidden(!row.chevron);
                self.icon.setImage(symbol(&row.icon, "").as_deref());
                let icon_color = row.icon_color.filter(|_| !selected).map(|[r, g, b]| {
                    super::appkit::ns_color([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
                });
                self.icon.setContentTintColor(Some(icon_color.as_deref().unwrap_or(&quiet)));
                self.label.setFont(Some(&NSFont::systemFontOfSize(13.0)));
                self.label.setAttributedStringValue(&marked(
                    &row.label,
                    &row.marks,
                    &selected_text,
                    row.font_family.as_deref(),
                    row.font_embedded,
                ));
                self.label.sizeToFit();
                let label_w = self.label.frame().size.width;
                let mut detail_w = 0.0;
                if let Some(detail) = &row.detail {
                    self.detail.setTextColor(Some(&quiet));
                    self.detail.setStringValue(&NSString::from_str(detail));
                    self.detail.sizeToFit();
                    detail_w = self.detail.frame().size.width;
                }
                let mut badge_w = None;
                if let Some(badge) = &row.badge {
                    self.badge_text.setTextColor(Some(&quiet));
                    self.badge_text.setStringValue(&NSString::from_str(badge));
                    self.badge_text.sizeToFit();
                    badge_w = Some(self.badge_text.frame().size.width.ceil());
                    let fill = if selected {
                        NSColor::alternateSelectedControlTextColor().colorWithAlphaComponent(0.22)
                    } else {
                        NSColor::quaternaryLabelColor()
                    };
                    self.badge.setFillColor(&fill);
                }
                self.chevron.setContentTintColor(Some(&quiet));
                let reserved = indicator_space(row.indicators.len());
                let separation = if reserved > 0.0 { INDICATOR_GAP } else { 0.0 };
                let f = frames(width - reserved - separation, label_w, detail_w, badge_w, row.chevron);
                self.icon.setFrame(f.icon);
                self.label.setFrame(f.label);
                self.detail.setFrame(f.detail);
                if let Some(at) = f.badge {
                    self.badge.setFrame(at);
                    let inner = NSRect::new(
                        NSPoint::new(BADGE_PAD, ((BADGE_H - 14.0) / 2.0).floor()),
                        NSSize::new(at.size.width - 2.0 * BADGE_PAD, 14.0),
                    );
                    self.badge_text.setFrame(inner);
                }
                if let Some(at) = f.chevron {
                    self.chevron.setFrame(at);
                }
                for (at, (view, indicator)) in self.indicators.iter().zip(&row.indicators).enumerate() {
                    let color = if selected {
                        NSColor::alternateSelectedControlTextColor()
                    } else {
                        match indicator.tone {
                            IndicatorTone::Neutral => NSColor::secondaryLabelColor(),
                            IndicatorTone::Attention => NSColor::systemOrangeColor(),
                            IndicatorTone::Error => NSColor::systemRedColor(),
                            IndicatorTone::Active => NSColor::systemBlueColor(),
                            IndicatorTone::Success => NSColor::systemGreenColor(),
                        }
                    };
                    view.setImage(symbol(&indicator.symbol, &indicator.label).as_deref());
                    view.setContentTintColor(Some(&color));
                    view.setFrame(NSRect::new(
                        NSPoint::new(
                            width - PAD - reserved + at as f64 * (INDICATOR_SIZE + INDICATOR_GAP),
                            (f64::from(ROW) - INDICATOR_SIZE) / 2.0,
                        ),
                        NSSize::new(INDICATOR_SIZE, INDICATOR_SIZE),
                    ));
                    view.setHidden(false);
                }
                let mut spoken = row.label.clone();
                if let Some(badge) = &row.badge {
                    spoken.push_str(", ");
                    spoken.push_str(badge);
                }
                if let Some(detail) = &row.detail {
                    spoken.push_str(", ");
                    spoken.push_str(detail);
                }
                for indicator in &row.indicators {
                    spoken.push_str(", ");
                    spoken.push_str(&indicator.label);
                }
                self.view.setAccessibilityLabel(Some(&NSString::from_str(&spoken)));
            }
        }
    }

    /// Which parts are showing.
    fn parts(&self, icon: bool, detail: bool, badge: bool) {
        self.icon.setHidden(!icon);
        self.detail.setHidden(!detail);
        self.badge.setHidden(!badge);
        self.chevron.setHidden(!icon);
    }
}

/// The weight the matched letters and the headers have.
#[allow(unsafe_code)]
fn semibold() -> f64 {
    // SAFETY: a constant AppKit defines, read on the main thread.
    unsafe { NSFontWeightSemibold }
}

/// `words` in the text colour, the byte ranges `marks` of it in a heavier weight.
fn marked(
    words: &str,
    marks: &[Range<usize>],
    color: &NSColor,
    family: Option<&str>,
    embedded: bool,
) -> Retained<NSMutableAttributedString> {
    let text = NSMutableAttributedString::initWithString(
        NSMutableAttributedString::alloc(),
        &NSString::from_str(words),
    );
    let whole = objc2_foundation::NSRange::new(0, words.encode_utf16().count());
    let family_font = |weight| {
        let family = family?;
        let mtm = MainThreadMarker::new()?;
        NSFontManager::sharedFontManager(mtm).fontWithFamily_traits_weight_size(
            &NSString::from_str(family),
            NSFontTraitMask(0),
            weight,
            13.0,
        )
    };
    let bundled = (embedded && family == Some(wormhole_core::theme::fonts::bundled::FAMILY))
        .then(embedded_fonts)
        .flatten();
    let plain = bundled
        .as_ref()
        .map(|fonts| fonts.0.clone())
        .or_else(|| family_font(5))
        .unwrap_or_else(|| NSFont::systemFontOfSize(13.0));
    let heavy = bundled
        .as_ref()
        .map(|fonts| fonts.1.clone())
        .or_else(|| family_font(9))
        .unwrap_or_else(|| NSFont::systemFontOfSize_weight(13.0, semibold()));
    // SAFETY: the attribute names are AppKit's constants, the values are the objects AppKit documents for them
    // and every range is inside the string.
    #[allow(unsafe_code)]
    unsafe {
        text.addAttribute_value_range(NSFontAttributeName, as_object(&*plain), whole);
        text.addAttribute_value_range(NSForegroundColorAttributeName, as_object(color), whole);
        for mark in marks {
            text.addAttribute_value_range(NSFontAttributeName, as_object(&*heavy), units(words, mark));
        }
    }
    text
}

/// Core Text and NSFont are toll-free bridged. Build directly from the bundled bytes so
/// previewing them does not register a process-wide font or shadow a user's custom face.
#[allow(unsafe_code)]
fn embedded_fonts() -> Option<(Retained<NSFont>, Retained<NSFont>)> {
    use objc2_core_foundation::{CFData, CFRetained};
    use objc2_core_graphics::{CGDataProvider, CGFont};
    use objc2_core_text::CTFont;
    use std::cell::OnceCell;

    fn face(bytes: &[u8]) -> Option<Retained<NSFont>> {
        // SAFETY: Core Foundation copies these valid bytes. Each object retains the previous
        // one as needed. CTFont and NSFont are toll-free bridged; transferring the +1 retain
        // to Retained preserves ownership.
        unsafe {
            let data = CFData::new(None, bytes.as_ptr(), bytes.len().try_into().ok()?)?;
            let provider = CGDataProvider::with_cf_data(Some(&data))?;
            let graphics = CGFont::with_data_provider(&provider)?;
            let font = CTFont::with_graphics_font(&graphics, 13.0, std::ptr::null(), None);
            Retained::from_raw(CFRetained::into_raw(font).as_ptr().cast::<NSFont>())
        }
    }

    thread_local! {
        static FONTS: OnceCell<Option<(Retained<NSFont>, Retained<NSFont>)>> = const { OnceCell::new() };
    }
    FONTS.with(|fonts| {
        fonts
            .get_or_init(|| {
                let faces = wormhole_core::theme::fonts::bundled::FACES;
                Some((face(faces[0])?, face(faces[1])?))
            })
            .clone()
    })
}

fn as_object<T: AsRef<AnyObject>>(value: &T) -> &AnyObject {
    value.as_ref()
}

#[cfg(test)]
mod tests {
    use super::{
        CHEVRON, GAP, ICON, INDICATOR_GAP, INDICATOR_SIZE, MAX_INDICATORS, PAD, frames, indicator_space,
    };

    #[test]
    fn status_symbols_leave_room_for_the_badge_and_chevron() {
        let reserved = indicator_space(99);
        assert_eq!(reserved, MAX_INDICATORS as f64 * (INDICATOR_SIZE + INDICATOR_GAP));
        assert_eq!(indicator_space(0), 0.0);
        let width = 400.0;
        let f = frames(width - reserved - INDICATOR_GAP, 120.0, 100.0, Some(30.0), true);
        assert!(f.chevron.unwrap().origin.x + CHEVRON < width - PAD - reserved);
    }

    #[test]
    fn embedded_font_preview_uses_the_bundled_faces() {
        let (regular, bold) = super::embedded_fonts().expect("embedded faces load");
        assert_eq!(regular.familyName().unwrap().to_string(), "JetBrains Mono");
        assert_eq!(regular.fontName().to_string(), "JetBrainsMono-Regular");
        assert_eq!(bold.fontName().to_string(), "JetBrainsMono-Bold");
    }

    #[test]
    fn the_label_comes_after_the_icon_and_the_detail_after_the_label() {
        let f = frames(400.0, 100.0, 60.0, None, false);
        assert_eq!(f.label.origin.x, PAD + ICON + GAP);
        assert_eq!(f.label.size.width, 100.0);
        assert_eq!(f.detail.origin.x, f.label.origin.x + 100.0 + GAP);
        assert_eq!(f.detail.size.width, 60.0);
    }

    #[test]
    fn the_chevron_and_the_badge_keep_the_right_edge() {
        let f = frames(400.0, 50.0, 0.0, Some(30.0), true);
        let chevron = f.chevron.expect("a chevron");
        assert_eq!(chevron.origin.x + CHEVRON, 400.0 - PAD);
        let badge = f.badge.expect("a badge");
        assert_eq!(badge.origin.x + badge.size.width, chevron.origin.x - GAP);
    }

    #[test]
    fn a_narrow_row_cuts_the_detail_first_then_the_label() {
        let f = frames(200.0, 120.0, 120.0, None, false);
        assert_eq!(f.label.size.width, 120.0);
        assert!(f.detail.size.width < 120.0 && f.detail.size.width >= 0.0);
        let g = frames(100.0, 500.0, 50.0, Some(40.0), true);
        assert_eq!(g.label.size.width, 0.0, "the badge and the chevron keep their places");
        assert_eq!(g.detail.size.width, 0.0);
    }
}
