//! What the picker panel is made of: Liquid Glass on macOS 26 and later, the frosted blur of the systems before
//! it, a plain fill under Reduce Transparency or Increase Contrast. The notice chip chooses the same way
//! (`notice::material`), so one switch (devctl's `notice-material`) turns both.

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::Bool;
use objc2_app_kit::{
    NSBezierPath, NSBox, NSBoxType, NSColor, NSGlassEffectView, NSImage, NSImageResizingMode,
    NSTitlePosition, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
    NSVisualEffectView,
};
use objc2_foundation::{NSEdgeInsets, NSRect, NSSize};

use crate::platform::picker::Material;

/// The panel's corners, and the border of the plain fill (the glass has an edge of its own).
const RADIUS: f64 = 18.0;
const EDGE: f64 = 1.0;

/// The material's view, and what holds the content.
pub enum Backdrop {
    Glass(Retained<NSGlassEffectView>),
    Frosted(Retained<NSVisualEffectView>),
    Solid(Retained<NSBox>),
}

impl Backdrop {
    pub fn make(material: Material, mtm: MainThreadMarker) -> Self {
        match material {
            Material::Glass => {
                let glass = NSGlassEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                glass.setCornerRadius(RADIUS);
                Self::Glass(glass)
            }
            Material::Frosted => {
                let view = NSVisualEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                view.setMaterial(NSVisualEffectMaterial::Popover);
                view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                // Active, so the blur doesn't go flat while the app is in the background.
                view.setState(NSVisualEffectState::Active);
                view.setMaskImage(Some(&rounded()));
                Self::Frosted(view)
            }
            Material::Solid => {
                let fill = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
                fill.setBoxType(NSBoxType::Custom);
                fill.setTitlePosition(NSTitlePosition::NoTitle);
                fill.setBorderWidth(EDGE);
                fill.setCornerRadius(RADIUS);
                fill.setContentViewMargins(NSSize::new(0.0, 0.0));
                fill.setFillColor(&NSColor::windowBackgroundColor());
                fill.setBorderColor(&NSColor::separatorColor());
                Self::Solid(fill)
            }
        }
    }

    pub fn material(&self) -> Material {
        match self {
            Self::Glass(_) => Material::Glass,
            Self::Frosted(_) => Material::Frosted,
            Self::Solid(_) => Material::Solid,
        }
    }

    pub fn view(&self) -> &NSView {
        match self {
            Self::Glass(view) => view,
            Self::Frosted(view) => view,
            Self::Solid(view) => view,
        }
    }

    /// Put `body` inside, filling it.
    pub fn hold(&self, body: &NSView) {
        match self {
            Self::Glass(glass) => glass.setContentView(Some(body)),
            Self::Frosted(effect) => effect.addSubview(body),
            Self::Solid(fill) => {
                if let Some(content) = fill.contentView() {
                    content.addSubview(body);
                }
            }
        }
    }
}

/// An opaque rounded rectangle that stretches in the middle: the blur behind a window is cut by a mask image.
fn rounded() -> Retained<NSImage> {
    let handler = block2::RcBlock::new(move |bounds: NSRect| -> Bool {
        NSColor::blackColor().set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, RADIUS, RADIUS).fill();
        Bool::YES
    });
    let side = 2.0 * RADIUS + 2.0;
    let image = NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(side, side), false, &handler);
    image.setCapInsets(NSEdgeInsets { top: RADIUS, left: RADIUS, bottom: RADIUS, right: RADIUS });
    image.setResizingMode(NSImageResizingMode::Stretch);
    image
}
