//! AppKit's side of `platform::vibrancy`: an `NSVisualEffectView` and a plain `NSBox` fill behind
//! GPUI's view, and the `NSGlassEffectView`s of Liquid Glass. Their material follows the theme's
//! appearance (`appearance`).

use gpui_kit::{Hsla, Window};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyClass;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBox, NSBoxType, NSColor, NSGlassEffectView, NSTitlePosition, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindowOrderingMode,
};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use wormhole_core::theme::tokens::Appearance;

use super::appearance;
use super::appkit::ns_view;
use crate::platform::vibrancy::GlassRect;

/// One window's blur and the fill under it.
pub struct BlurViews {
    effect: Retained<NSVisualEffectView>,
    backing: Retained<NSBox>,
}

impl BlurViews {
    /// Behind GPUI's view: the blur, and under it the fill (hidden until `set_backing`).
    #[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
    pub fn install(window: &Window, enabled: bool) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let view = ns_view(window)?;
        // SAFETY: GPUI's view, on the main thread; AppKit only reads the hierarchy here.
        let parent = unsafe { view.superview() }?;

        let effect = NSVisualEffectView::initWithFrame(mtm.alloc(), parent.bounds());
        effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::Active);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        effect.setHidden(!enabled);
        appearance::follow_effect(&effect);
        parent.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, Some(&view));

        let backing = NSBox::initWithFrame(mtm.alloc(), parent.bounds());
        backing.setBoxType(NSBoxType::Custom);
        backing.setTitlePosition(NSTitlePosition::NoTitle);
        backing.setBorderWidth(0.0);
        backing.setContentViewMargins(NSSize::new(0.0, 0.0));
        backing.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        backing.setHidden(true);
        parent.addSubview_positioned_relativeTo(&backing, NSWindowOrderingMode::Below, Some(&effect));
        Some(Self { effect, backing })
    }

    pub fn set_backing(&self, color: Option<Hsla>) {
        match color {
            Some(color) => {
                let c = color.to_rgb();
                let fill = NSColor::colorWithSRGBRed_green_blue_alpha(
                    f64::from(c.r),
                    f64::from(c.g),
                    f64::from(c.b),
                    f64::from(c.a),
                );
                self.backing.setFillColor(&fill);
                self.backing.setHidden(false);
            }
            None => self.backing.setHidden(true),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.effect.setHidden(!enabled);
    }
}

/// GPUI's view, its parent, and the glass views made so far (a pool: the nth region uses the
/// nth view, and the rest are hidden).
pub struct GlassViews {
    mtm: MainThreadMarker,
    parent: Retained<NSView>,
    anchor: Retained<NSView>,
    glass: Vec<Retained<NSGlassEffectView>>,
}

impl GlassViews {
    #[allow(unsafe_code)] // AppKit interop; GPUI hands out its live NSView on the main thread.
    pub fn find(window: &Window) -> Option<Self> {
        // `NSGlassEffectView` only exists from macOS 26; naming the class on an older
        // system would abort, so ask the runtime first.
        AnyClass::get(c"NSGlassEffectView")?;
        let mtm = MainThreadMarker::new()?;
        let anchor = ns_view(window)?;
        // SAFETY: see above.
        let parent = unsafe { anchor.superview() }?;
        Some(Self { mtm, parent, anchor, glass: Vec::new() })
    }

    pub fn place(&mut self, rects: &[GlassRect]) {
        let base = self.anchor.frame();
        let flipped = self.parent.isFlipped();
        while self.glass.len() < rects.len() {
            let view = NSGlassEffectView::initWithFrame(self.mtm.alloc(), NSRect::ZERO);
            appearance::follow_effect(&view);
            // GPUI lays the regions out from the window's top, and `Glass::commit` places nothing while
            // they stay where they were. So a glass keeps its distance from the top when the window
            // grows or shrinks: from the bottom, it would slide away from its card.
            view.setAutoresizingMask(if flipped {
                NSAutoresizingMaskOptions::ViewMaxYMargin
            } else {
                NSAutoresizingMaskOptions::ViewMinYMargin
            });
            // Just under GPUI's view, over the window's blur.
            self.parent.addSubview_positioned_relativeTo(
                &view,
                NSWindowOrderingMode::Below,
                Some(&self.anchor),
            );
            self.glass.push(view);
        }
        for (ix, view) in self.glass.iter().enumerate() {
            let Some(rect) = rects.get(ix) else {
                view.setHidden(true);
                continue;
            };
            let (x, w, h) = (f64::from(rect.x), f64::from(rect.w), f64::from(rect.h));
            let y = f64::from(rect.y);
            let y = if flipped { base.origin.y + y } else { base.origin.y + base.size.height - y - h };
            view.setCornerRadius(f64::from(rect.radius));
            view.setFrame(NSRect::new(NSPoint::new(base.origin.x + x, y), NSSize::new(w, h)));
            view.setHidden(false);
        }
    }
}

/// The theme's appearance: the blur and glass views, and the traffic lights (`window`), follow it.
pub fn set_material_appearance(appearance: Appearance) {
    appearance::set(appearance);
}
