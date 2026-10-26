//! AppKit's side of `platform::notice`: a borderless, non-activating panel that is a child of the app window's
//! `NSWindow`, wide as that window and as tall as the chip with the gap it rests at and room below for the
//! glass's shadow, whose top edge sits on the title row's lower edge. The window's own edge is the clip: the
//! chip drops in from above it and goes back up behind it.
//!
//! The chip is a view tree, `holder` (the layer Core Animation moves) > the material (Liquid Glass, the HUD's
//! frosted blur or a plain box) > `label` (an SF Symbol and the words). The drop is a `CASpringAnimation` on
//! the holder's layer, the way up a basic animation, and Reduce Motion an opacity animation, so GPUI draws
//! nothing while it plays. The theme's appearance goes on the holder (never on an `NSWindow`: GPUI reads that
//! one as the system's).

use std::cell::Cell;

use gpui_kit::Window;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2_app_kit::{
    NSAnimatablePropertyContainer as _, NSAnimationContext, NSBackingStoreType, NSBezierPath, NSBox,
    NSBoxType, NSColor, NSFont, NSGlassEffectView, NSImage, NSImageScaling, NSImageView, NSLineBreakMode,
    NSPanel, NSTextField, NSTitlePosition, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
    NSVisualEffectState, NSVisualEffectView, NSWindow, NSWindowCollectionBehavior, NSWindowOrderingMode,
    NSWindowStyleMask,
};
use objc2_core_foundation::CGAffineTransform;
use objc2_foundation::{NSNumber, NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::{
    CAAnimation, CABasicAnimation, CALayer, CAMediaTiming as _, CAMediaTimingFunction, CASpringAnimation,
    CATransition, kCATransitionFade,
};
use wormhole_core::view::motion::CubicBezier;
use wormhole_core::view::motion::notice::{
    DROP, Entrance, FADE_EASE, FADE_IN, FADE_OUT, REST_GAP, SWAP, UP, UP_EASE, travel,
};
use wormhole_core::view::notice::Glyph;

use super::appearance;
use super::appkit::{ns_window, symbol};
use crate::platform::notice::{Content, Material};

/// The chip's own metrics, AppKit's native ones for a small capsule (the app's style scale is for what
/// GPUI draws; this is system chrome, like a menu row in `rows`).
const HEIGHT: f64 = 34.0;
/// Beside the symbol and the words.
const PAD: f64 = 14.0;
const ICON: f64 = 15.0;
const GAP: f64 = 7.0;
/// The chip keeps this far from the window's side edges, whatever the words.
const MARGIN: f64 = 16.0;
/// Under the chip, inside the panel: where the glass's shadow falls.
const SHADOW_ROOM: f64 = 14.0;
/// The words' size: the system's, as a menu row's.
const FONT: f64 = 13.0;
/// The border of the plain fill, which has no glass edge to show where it ends.
const EDGE: f64 = 1.0;

thread_local! {
    /// devctl's `notice-material`.
    #[cfg(feature = "devctl")]
    static FORCED: Cell<Option<Material>> = const { Cell::new(None) };
}

#[cfg(feature = "devctl")]
pub fn force_material(material: Option<Material>) {
    FORCED.set(material);
}

/// What the chip is made of now: Reduce Transparency and Increase Contrast ask for a plain fill, otherwise
/// Liquid Glass where the system has it and the HUD's blur where it hasn't.
fn material() -> Material {
    #[cfg(feature = "devctl")]
    if let Some(forced) = FORCED.get() {
        return forced;
    }
    if super::look::reduce_transparency() || super::look::increase_contrast() {
        Material::Solid
    } else if AnyClass::get(c"NSGlassEffectView").is_some() {
        // Named only once it is known to exist: on an older system the class lookup aborts.
        Material::Glass
    } else {
        Material::Frosted
    }
}

/// The material's view, and what holds the label inside it.
enum Backdrop {
    Glass(Retained<NSGlassEffectView>),
    Frosted(Retained<NSVisualEffectView>),
    Solid(Retained<NSBox>),
}

impl Backdrop {
    fn make(material: Material, mtm: MainThreadMarker) -> Self {
        match material {
            Material::Glass => Self::Glass(NSGlassEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO)),
            Material::Frosted => {
                let view = NSVisualEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                view.setMaterial(NSVisualEffectMaterial::HUDWindow);
                view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                // Active, so the blur doesn't go flat while the app is in the background.
                view.setState(NSVisualEffectState::Active);
                Self::Frosted(view)
            }
            Material::Solid => {
                let fill = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
                fill.setBoxType(NSBoxType::Custom);
                fill.setTitlePosition(NSTitlePosition::NoTitle);
                fill.setBorderWidth(EDGE);
                fill.setContentViewMargins(NSSize::new(0.0, 0.0));
                fill.setFillColor(&NSColor::windowBackgroundColor());
                fill.setBorderColor(&NSColor::separatorColor());
                Self::Solid(fill)
            }
        }
    }

    fn material(&self) -> Material {
        match self {
            Self::Glass(_) => Material::Glass,
            Self::Frosted(_) => Material::Frosted,
            Self::Solid(_) => Material::Solid,
        }
    }

    fn view(&self) -> &NSView {
        match self {
            Self::Glass(view) => view,
            Self::Frosted(view) => view,
            Self::Solid(view) => view,
        }
    }

    /// Put `label` inside, filling it.
    fn hold(&self, label: &NSView) {
        match self {
            Self::Glass(glass) => glass.setContentView(Some(label)),
            Self::Frosted(effect) => effect.addSubview(label),
            Self::Solid(fill) => {
                if let Some(content) = fill.contentView() {
                    content.addSubview(label);
                }
            }
        }
    }

    /// Cut the material to a capsule of `size`.
    fn round(&self, size: NSSize) {
        let radius = size.height / 2.0;
        match self {
            Self::Glass(glass) => glass.setCornerRadius(radius),
            Self::Solid(fill) => fill.setCornerRadius(radius),
            // The blur behind a window is cut by the view's mask image, not by its layer.
            Self::Frosted(effect) => effect.setMaskImage(Some(&capsule(size))),
        }
    }
}

/// An opaque capsule of `size`, for the blur to be cut to.
fn capsule(size: NSSize) -> Retained<NSImage> {
    let handler = block2::RcBlock::new(move |bounds: NSRect| -> Bool {
        NSColor::blackColor().set();
        let radius = bounds.size.height / 2.0;
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, radius, radius).fill();
        Bool::YES
    });
    NSImage::imageWithSize_flipped_drawingHandler(size, false, &handler)
}

/// Where everything sits for words of a given width.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    /// The capsule.
    chip: NSSize,
    /// In the label's own space.
    icon: NSRect,
    text: NSRect,
}

/// The capsule for words `text` big, in a window with `room` for it across.
fn geometry(text: NSSize, room: f64) -> Geometry {
    let fixed = PAD + ICON + GAP + PAD;
    let words = text.width.min((room - fixed).max(0.0)).ceil();
    let chip = NSSize::new(fixed + words, HEIGHT);
    let centred = |side: f64| ((HEIGHT - side) / 2.0).floor();
    Geometry {
        chip,
        icon: NSRect::new(NSPoint::new(PAD, centred(ICON)), NSSize::new(ICON, ICON)),
        text: NSRect::new(
            NSPoint::new(PAD + ICON + GAP, centred(text.height)),
            NSSize::new(words, text.height.ceil()),
        ),
    }
}

/// The SF Symbol for a glyph.
fn symbol_name(glyph: Glyph) -> &'static str {
    match glyph {
        Glyph::Copy => "doc.on.doc",
    }
}

/// One window's chip.
pub struct Chip {
    parent: Retained<NSWindow>,
    panel: Retained<NSPanel>,
    holder: Retained<NSView>,
    backdrop: Backdrop,
    label: Retained<NSView>,
    icon: Retained<NSImageView>,
    text: Retained<NSTextField>,
    /// The chip as it is now (its width follows the words).
    chip: NSSize,
    /// The title row's lower edge, from the window's top.
    inset: f64,
    entered: bool,
}

impl Chip {
    /// A panel for `window`'s chip, not yet on screen. `None` for a window without a native one.
    #[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
    pub fn new(window: &Window) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let parent = ns_window(window)?;
        // SAFETY: on the main thread; the panel is kept by `Retained`, so AppKit must not also release it
        // when it closes.
        let panel = unsafe {
            let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                NSRect::ZERO,
                NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                NSBackingStoreType::Buffered,
                false,
            );
            panel.setReleasedWhenClosed(false);
            panel
        };
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(false);
        panel.setIgnoresMouseEvents(true);
        panel.setHidesOnDeactivate(false);
        // Out of Mission Control and the window cycle, and over a full-screen Space like its parent.
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );

        let holder = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        holder.setWantsLayer(true);
        panel.setContentView(Some(&holder));

        let backdrop = Backdrop::make(material(), mtm);
        holder.addSubview(backdrop.view());

        let label = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        label.setWantsLayer(true);
        let icon = NSImageView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        icon.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        let text = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text.setFont(Some(&NSFont::systemFontOfSize(FONT)));
        text.setTextColor(Some(&NSColor::labelColor()));
        text.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.addSubview(&icon);
        label.addSubview(&text);
        backdrop.hold(&label);

        Some(Self {
            parent,
            panel,
            holder,
            backdrop,
            label,
            icon,
            text,
            chip: NSSize::ZERO,
            inset: 0.0,
            entered: false,
        })
    }

    /// Bring the chip in with `content`, `top_inset` points under the window's top edge.
    pub fn show(&mut self, content: &Content<'_>, top_inset: f32, entrance: Entrance) {
        self.inset = f64::from(top_inset);
        self.set_content(content);
        self.lay_out(false);
        let layer = self.layer();
        layer.removeAllAnimations();
        match entrance {
            Entrance::Drop => {
                rest(&layer);
                drop_in(&layer, self.travel());
            }
            Entrance::Fade => {
                rest(&layer);
                fade(&layer, 0.0, 1.0, FADE_IN, FADE_EASE);
            }
        }
        if !self.entered {
            // SAFETY: both windows are live and on the main thread.
            #[allow(unsafe_code)]
            unsafe {
                self.parent.addChildWindow_ordered(&self.panel, NSWindowOrderingMode::Above);
            }
            self.entered = true;
        }
    }

    /// New words in the chip that is up: they crossfade and the capsule's width follows.
    pub fn swap(&mut self, content: &Content<'_>) {
        // The old picture fades into the new: a transition on the label's layer snapshots both.
        if let Some(layer) = self.label.layer() {
            let fade = CATransition::animation();
            // SAFETY: a type name Core Animation defines, an immutable static.
            #[allow(unsafe_code)]
            fade.setType(unsafe { kCATransitionFade });
            fade.setDuration(SWAP.as_secs_f64());
            layer.addAnimation_forKey(&fade, Some(&NSString::from_str("notice-swap")));
        }
        self.set_content(content);
        self.lay_out(true);
    }

    /// Send the chip away; the panel stays until the chip is dropped.
    pub fn hide(&mut self, entrance: Entrance) {
        let layer = self.layer();
        layer.removeAllAnimations();
        match entrance {
            Entrance::Drop => {
                // The model is where it ends, so there is no flash between the animation and the panel's going.
                let away = self.travel();
                layer.setAffineTransform(translation(away));
                slide(&layer, 0.0, away, UP.as_secs_f64(), UP_EASE);
            }
            Entrance::Fade => {
                layer.setOpacity(0.0);
                fade(&layer, 1.0, 0.0, FADE_OUT, FADE_EASE);
            }
        }
    }

    /// Put the panel under the title row again: the window was resized, went full screen or came back.
    pub fn place(&mut self, top_inset: f32) {
        self.inset = f64::from(top_inset);
        self.lay_out(false);
    }

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::notice::Info> {
        let frame = self.parent.frame();
        let chip = self.chip;
        Some(crate::platform::notice::Info {
            material: self.backdrop.material(),
            frame: [
                ((frame.size.width - chip.width) / 2.0) as f32,
                (self.inset + f64::from(REST_GAP)) as f32,
                chip.width as f32,
                chip.height as f32,
            ],
            window_number: self.panel.windowNumber() as i64,
            parent_number: self.parent.windowNumber() as i64,
        })
    }

    fn layer(&self) -> Retained<CALayer> {
        // Layer-backed from the start (`new`), so it is there.
        self.holder.layer().unwrap_or_else(CALayer::new)
    }

    /// How far the chip goes to be out of sight above the panel's top edge.
    fn travel(&self) -> f64 {
        f64::from(travel(self.chip.height as f32))
    }

    fn set_content(&mut self, content: &Content<'_>) {
        let Content { glyph, text, appearance: theme } = *content;
        self.text.setStringValue(&NSString::from_str(text));
        self.icon.setImage(symbol(symbol_name(glyph), text).as_deref());
        // The theme's appearance on everything in the chip, whatever the system's is.
        appearance::set_on(&self.holder, theme);
    }

    /// Size the capsule to the words and the panel to the window, the capsule centred in it. `animated`:
    /// the capsule's width follows the words over the swap's time.
    fn lay_out(&mut self, animated: bool) {
        self.text.sizeToFit();
        let parent = self.parent.frame();
        let geometry = geometry(self.text.frame().size, parent.size.width - 2.0 * MARGIN);
        self.chip = geometry.chip;

        let height = geometry.chip.height + f64::from(REST_GAP) + SHADOW_ROOM;
        let panel = NSRect::new(
            NSPoint::new(parent.origin.x, parent.origin.y + parent.size.height - self.inset - height),
            NSSize::new(parent.size.width, height),
        );
        if self.panel.frame() != panel {
            self.panel.setFrame_display(panel, true);
        }
        self.holder.setFrame(NSRect::new(NSPoint::ZERO, panel.size));

        let at = NSRect::new(
            NSPoint::new(((panel.size.width - geometry.chip.width) / 2.0).floor(), SHADOW_ROOM),
            geometry.chip,
        );
        self.label.setFrame(NSRect::new(NSPoint::ZERO, geometry.chip));
        self.icon.setFrame(geometry.icon);
        self.text.setFrame(geometry.text);
        self.backdrop.round(geometry.chip);
        if animated {
            NSAnimationContext::beginGrouping();
            NSAnimationContext::currentContext().setDuration(SWAP.as_secs_f64());
            self.backdrop.view().animator().setFrame(at);
            NSAnimationContext::endGrouping();
        } else {
            self.backdrop.view().setFrame(at);
        }
    }
}

impl Drop for Chip {
    fn drop(&mut self) {
        self.parent.removeChildWindow(&self.panel);
        self.panel.orderOut(None);
        self.panel.close();
    }
}

/// Where the holder's layer sits when the chip is at rest and whole.
fn rest(layer: &CALayer) {
    layer.setAffineTransform(translation(0.0));
    layer.setOpacity(1.0);
}

/// Up by `y` points (a layer's y runs up in a view that isn't flipped).
fn translation(y: f64) -> CGAffineTransform {
    CGAffineTransform { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: y }
}

/// The drop: from `from` points above its place to it, on core's spring (mass 1).
#[allow(unsafe_code)]
fn drop_in(layer: &CALayer, from: f64) {
    let spring =
        CASpringAnimation::animationWithKeyPath(Some(&NSString::from_str("transform.translation.y")));
    spring.setMass(1.0);
    spring.setStiffness(f64::from(DROP.stiffness));
    spring.setDamping(f64::from(DROP.damping));
    set_range(&spring, from, 0.0);
    spring.setDuration(spring.settlingDuration());
    layer.addAnimation_forKey(&spring, Some(&NSString::from_str("notice")));
}

/// A slide along y from `from` to `to`.
fn slide(layer: &CALayer, from: f64, to: f64, seconds: f64, curve: CubicBezier) {
    let slide = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("transform.translation.y")));
    set_range(&slide, from, to);
    timed(&slide, seconds, curve);
    layer.addAnimation_forKey(&slide, Some(&NSString::from_str("notice")));
}

/// The opacity from `from` to `to`.
fn fade(layer: &CALayer, from: f64, to: f64, time: std::time::Duration, curve: CubicBezier) {
    let fade = CABasicAnimation::animationWithKeyPath(Some(&NSString::from_str("opacity")));
    set_range(&fade, from, to);
    timed(&fade, time.as_secs_f64(), curve);
    layer.addAnimation_forKey(&fade, Some(&NSString::from_str("notice")));
}

#[allow(unsafe_code)]
fn set_range(animation: &CABasicAnimation, from: f64, to: f64) {
    let (from, to) = (NSNumber::new_f64(from), NSNumber::new_f64(to));
    let (from, to): (&AnyObject, &AnyObject) = (&from, &to);
    // SAFETY: the animated key paths are numbers, which is what the values are.
    unsafe {
        animation.setFromValue(Some(from));
        animation.setToValue(Some(to));
    }
}

fn timed(animation: &CAAnimation, seconds: f64, CubicBezier(x1, y1, x2, y2): CubicBezier) {
    animation.setDuration(seconds);
    animation.setTimingFunction(Some(&CAMediaTimingFunction::functionWithControlPoints(x1, y1, x2, y2)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(width: f64) -> NSSize {
        NSSize::new(width, 16.0)
    }

    #[test]
    fn the_capsule_is_as_wide_as_its_words_and_the_padding() {
        let g = geometry(words(80.0), 800.0);
        assert_eq!(g.chip, NSSize::new(PAD + ICON + GAP + 80.0 + PAD, HEIGHT));
        assert_eq!(g.text.size.width, 80.0);
    }

    #[test]
    fn the_symbol_and_the_words_sit_in_the_middle_of_the_height() {
        let g = geometry(words(80.0), 800.0);
        assert_eq!(g.icon.origin.y, ((HEIGHT - ICON) / 2.0).floor());
        assert_eq!(g.text.origin.y, ((HEIGHT - 16.0) / 2.0).floor());
        assert!(g.icon.origin.x + g.icon.size.width + GAP <= g.text.origin.x + 1e-9);
    }

    #[test]
    fn words_wider_than_the_room_are_cut_not_the_window() {
        let g = geometry(words(2000.0), 300.0);
        assert!(g.chip.width <= 300.0 + 1.0, "{}", g.chip.width);
        assert_eq!(g.text.size.width, 300.0 - (PAD + ICON + GAP + PAD));
    }

    #[test]
    fn a_window_with_no_room_still_gets_a_chip() {
        let g = geometry(words(50.0), 0.0);
        assert_eq!(g.text.size.width, 0.0);
        assert_eq!(g.chip.height, HEIGHT);
    }
}
