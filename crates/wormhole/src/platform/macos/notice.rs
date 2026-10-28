//! AppKit's side of `platform::notice`: a borderless, non-activating panel that is a child of the app window's
//! `NSWindow`, wide as that window (but see `Chip::set_clickable`: it ignores the mouse, and while a sticky
//! message waits for a click it shrinks to the capsule) and as tall as the chip with the gap it rests at and room below for the
//! glass's shadow, whose top edge sits on the top edge of the terminal the chip is about (core's `place` says
//! where the chip is in it). The panel's own edge is the clip: the chip drops in from above it and goes back up
//! behind it.
//!
//! The chip is a view tree, `holder` (the layer Core Animation moves) > the material (Liquid Glass, the HUD's
//! frosted blur or a plain box) > `label` (an SF Symbol and the words). The drop is a `CASpringAnimation` on
//! the holder's layer, the way up a basic animation, and Reduce Motion an opacity animation, so GPUI draws
//! nothing while it plays. The theme's appearance goes on the holder (never on an `NSWindow`: GPUI reads that
//! one as the system's).

use std::rc::Rc;

#[cfg(feature = "devctl")]
use std::cell::Cell;

use gpui_kit::{App, Window};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2_app_kit::{
    NSAccessibility as _, NSAnimatablePropertyContainer as _, NSAnimationContext, NSBackingStoreType,
    NSBezierPath, NSBox, NSBoxType, NSColor, NSFont, NSGlassEffectView, NSImage, NSImageResizingMode,
    NSImageScaling, NSImageView, NSLineBreakMode, NSPanel, NSTextField, NSTitlePosition, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowCollectionBehavior, NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_core_foundation::CGAffineTransform;
use objc2_foundation::{NSEdgeInsets, NSNumber, NSPoint, NSRect, NSSize, NSString};
use objc2_quartz_core::{
    CAAnimation, CABasicAnimation, CALayer, CAMediaTiming as _, CAMediaTimingFunction, CASpringAnimation,
    CATransition, kCATransitionFade,
};
use wormhole_core::view::motion::CubicBezier;
use wormhole_core::view::motion::notice::{DROP, Entrance, FADE_EASE, FADE_IN, FADE_OUT, SWAP, UP, UP_EASE};
use wormhole_core::view::notice::{Glyph, Placed, Spot, place};
use wormhole_core::view::panes::Rect;

use super::appearance;
use super::appkit::{ns_window, symbol};
use super::notice_click::{ClickView, click_view};
use crate::platform::notice::{Clicks, Content, Material};

/// The chip's own metrics, AppKit's native ones for a small capsule (the app's style scale is for what
/// GPUI draws; this is system chrome, like a menu row in `rows`).
const HEIGHT: f64 = 34.0;
/// Beside the symbol and the words.
const PAD: f64 = 14.0;
const ICON: f64 = 15.0;
const GAP: f64 = 7.0;
/// Under the chip, inside the panel: where the glass's shadow falls.
const SHADOW_ROOM: f64 = 14.0;
/// How far the panel reaches past each side of the capsule while it is clickable: where the glass's
/// shadow falls. A click there is the panel's, and does nothing.
const SIDE_ROOM: f64 = 10.0;
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

/// What the chip is made of now: `opaque` (Reduce Transparency or Increase Contrast, as the app reads them)
/// asks for a plain fill, otherwise
/// Liquid Glass where the system has it and the HUD's blur where it hasn't.
fn material(opaque: bool) -> Material {
    #[cfg(feature = "devctl")]
    if let Some(forced) = FORCED.get() {
        return forced;
    }
    if opaque {
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
            Material::Glass => {
                let glass = NSGlassEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                glass.setCornerRadius(HEIGHT / 2.0);
                Self::Glass(glass)
            }
            Material::Frosted => {
                let view = NSVisualEffectView::initWithFrame(mtm.alloc(), NSRect::ZERO);
                view.setMaterial(NSVisualEffectMaterial::HUDWindow);
                view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
                // Active, so the blur doesn't go flat while the app is in the background.
                view.setState(NSVisualEffectState::Active);
                // The blur behind a window is cut by the view's mask image, not by its layer: a capsule whose
                // middle stretches, so the cut follows the view's width when the words change.
                view.setMaskImage(Some(&capsule()));
                Self::Frosted(view)
            }
            Material::Solid => {
                let fill = NSBox::initWithFrame(mtm.alloc(), NSRect::ZERO);
                fill.setBoxType(NSBoxType::Custom);
                fill.setTitlePosition(NSTitlePosition::NoTitle);
                fill.setBorderWidth(EDGE);
                fill.setCornerRadius(HEIGHT / 2.0);
                fill.setContentViewMargins(NSSize::new(0.0, 0.0));
                fill.setFillColor(&NSColor::windowBackgroundColor());
                fill.setBorderColor(&NSColor::separatorColor());
                Self::Solid(fill)
            }
        }
    }

    #[cfg(feature = "devctl")]
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
}

/// An opaque capsule as tall as the chip, with a stretching middle.
fn capsule() -> Retained<NSImage> {
    let radius = HEIGHT / 2.0;
    let handler = block2::RcBlock::new(move |bounds: NSRect| -> Bool {
        NSColor::blackColor().set();
        NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(bounds, radius, radius).fill();
        Bool::YES
    });
    let image =
        NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(HEIGHT + 2.0, HEIGHT), false, &handler);
    image.setCapInsets(NSEdgeInsets { top: 0.0, left: radius, bottom: 0.0, right: radius });
    image.setResizingMode(NSImageResizingMode::Stretch);
    image
}

/// Where everything sits in a capsule `chip_w` wide, for words `text` big.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    /// In the label's own space.
    icon: NSRect,
    text: NSRect,
}

/// What the capsule needs beside its words.
const FIXED: f64 = PAD + ICON + GAP + PAD;

fn geometry(text: NSSize, chip_w: f64) -> Geometry {
    // A capsule too narrow for them cuts the words (the label truncates the tail); the symbol stays.
    let words = text.width.min((chip_w - FIXED).max(0.0)).ceil();
    let centred = |side: f64| ((HEIGHT - side) / 2.0).floor();
    Geometry {
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
        Glyph::Eye => "eye",
        Glyph::Keyboard => "keyboard",
        Glyph::Warning => "exclamationmark.triangle",
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
    /// Lies over the capsule and takes the click, while the chip is clickable.
    press: Retained<ClickView>,
    /// The chip takes a click now: the panel is only as wide as the capsule (`set_clickable`).
    clickable: bool,
    /// Where the chip is, as last laid out.
    placed: Placed,
    /// The chip is on its way out (or gone): the holder is mid-animation and is left alone until it comes back.
    leaving: bool,
    /// The panel has been given to the parent window.
    entered: bool,
    /// The panel has been taken down (`close`), so dropping the chip has nothing left to do.
    taken_down: bool,
}

impl Chip {
    /// A panel for `window`'s chip, not yet on screen: `opaque` for a plain fill (the person asked the
    /// system for no transparency or more contrast). A click on the chip, while it takes one, is sent to
    /// `clicks`. `None` for a window without a native one.
    pub fn new(window: &Window, opaque: bool, clicks: Clicks) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let parent = ns_window(window)?;
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc(),
            NSRect::ZERO,
            NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
            NSBackingStoreType::Buffered,
            false,
        );
        // SAFETY: on the main thread (`mtm`), and the panel is kept by `Retained`, so AppKit must not also
        // release it when it closes.
        #[allow(unsafe_code)]
        unsafe {
            panel.setReleasedWhenClosed(false);
        }
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

        let backdrop = Backdrop::make(material(opaque), mtm);
        holder.addSubview(backdrop.view());

        let label = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        label.setWantsLayer(true);
        let icon = NSImageView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        icon.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        icon.setContentTintColor(Some(&NSColor::secondaryLabelColor()));
        let text = NSTextField::labelWithString(&NSString::from_str(""), mtm);
        text.setFont(Some(&NSFont::systemFontOfSize(NSFont::systemFontSize())));
        text.setTextColor(Some(&NSColor::labelColor()));
        text.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
        label.addSubview(&icon);
        label.addSubview(&text);
        backdrop.hold(&label);

        // Over the capsule, above the glass, and out of the way until the chip is clickable. The channel is
        // the app's to read: a press can come at any moment of the event loop, outside any GPUI update.
        let press = click_view(
            mtm,
            "",
            Rc::new(move || {
                // Unbounded: it never waits, and it only fails when the app has stopped listening.
                clicks.try_send(()).ok();
            }),
        );
        press.setHidden(true);
        holder.addSubview(&press);

        let placed = Placed { top: 0.0, chip: Rect::new(0.0, 0.0, 0.0, 0.0), cut: false };
        Some(Self {
            parent,
            panel,
            holder,
            backdrop,
            label,
            icon,
            text,
            press,
            clickable: false,
            placed,
            leaving: false,
            entered: false,
            taken_down: false,
        })
    }

    /// Bring the chip in with `content`, at `spot`.
    pub fn show(
        &mut self,
        content: &Content<'_>,
        spot: Spot,
        entrance: Entrance,
        clickable: bool,
        cx: &mut App,
    ) {
        self.leaving = false;
        self.set_content(content);
        self.set_clickable(clickable);
        self.lay_out(spot, false);
        let layer = self.layer();
        layer.removeAllAnimations();
        rest(&layer);
        match entrance {
            Entrance::Drop => drop_in(&layer, self.travel()),
            Entrance::Fade => fade(&layer, 0.0, 1.0, FADE_IN, FADE_EASE),
        }
        if !self.entered {
            self.entered = true;
            // Ordering a child window makes AppKit tell the parent, whose view tells GPUI: done from a task,
            // outside this update (as `panel::set_height_keeping_top` does its resize), so GPUI is free to hear.
            let (parent, panel) = (self.parent.clone(), self.panel.clone());
            cx.spawn(async move |_| adopt(&parent, &panel)).detach();
        }
    }

    /// New words in the chip that is up: they crossfade and the capsule's width follows.
    pub fn swap(&mut self, content: &Content<'_>, spot: Spot, clickable: bool) {
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
        // The capsule's width follows the words only inside a panel as wide as the window: a panel that
        // changes its own width (a copy over a sticky message, or the sticky one back) has it snap.
        let snap = self.clickable || clickable;
        self.set_clickable(clickable);
        self.lay_out(spot, !snap);
    }

    /// Send the chip away; the panel stays until the chip is closed.
    pub fn hide(&mut self, entrance: Entrance) {
        self.leaving = true;
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

    /// Whether the chip takes a click on its capsule (and shows the pointing hand over it). A copy's chip
    /// ignores the mouse, so every click goes through the panel to the window under it. The panel spans the
    /// window, though, and AppKit gives a window that takes the mouse every click in its frame, so while the
    /// chip is clickable the panel is cut down to the capsule and a little room around it (`lay_out`), and the
    /// clear view over the capsule takes the click. A click anywhere else is not over the panel at all.
    pub fn set_clickable(&mut self, clickable: bool) {
        self.clickable = clickable;
        self.press.setHidden(!clickable);
        self.panel.setIgnoresMouseEvents(!clickable);
    }

    /// Put the chip at `spot` again: the window was resized, went full screen or came back, or the terminal it
    /// is about moved. A chip that is leaving is left to go.
    pub fn place(&mut self, spot: Spot) {
        if !self.leaving {
            self.lay_out(spot, false);
        }
    }

    /// Take the panel down. Closing a window tells its parent and AppKit may tell GPUI, so it is done from a
    /// task, outside whatever update this is called in.
    pub fn close(mut self, cx: &mut App) {
        self.taken_down = true;
        let (parent, panel) = (self.parent.clone(), self.panel.clone());
        cx.spawn(async move |_| take_down(&parent, &panel)).detach();
    }

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::notice::Info> {
        let chip = self.placed.chip;
        Some(crate::platform::notice::Info {
            material: self.backdrop.material(),
            frame: [chip.x, chip.y, chip.w, chip.h],
            takes_mouse: !self.panel.ignoresMouseEvents(),
            window_number: self.panel.windowNumber() as i64,
            parent_number: self.parent.windowNumber() as i64,
        })
    }

    fn layer(&self) -> Retained<CALayer> {
        // Layer-backed from the start (`new`), so it is there.
        self.holder.layer().unwrap_or_default()
    }

    /// How far the chip goes to be out of sight above the panel's top edge: its height and the gap it rests at.
    fn travel(&self) -> f64 {
        f64::from(self.placed.chip.h + self.gap())
    }

    /// How far under the panel's top edge the chip rests.
    fn gap(&self) -> f32 {
        self.placed.chip.y - self.placed.top
    }

    fn set_content(&mut self, content: &Content<'_>) {
        let Content { glyph, text, appearance: theme } = *content;
        self.text.setStringValue(&NSString::from_str(text));
        self.press.setAccessibilityLabel(Some(&NSString::from_str(text)));
        self.icon.setImage(symbol(symbol_name(glyph), text).as_deref());
        // The theme's appearance on everything in the chip, whatever the system's is.
        appearance::set_on(&self.holder, theme);
    }

    /// Place the chip at `spot` and the panel, which is as wide as the window, over it. `animated`: the
    /// capsule's width follows the words over the swap's time.
    fn lay_out(&mut self, spot: Spot, animated: bool) {
        self.text.sizeToFit();
        let words = self.text.frame().size;
        let natural = ((FIXED + words.width.ceil()) as f32, HEIGHT as f32);
        self.placed = place(spot, natural);
        let Placed { top, chip, .. } = self.placed;
        let geometry = geometry(words, f64::from(chip.w));

        let parent = self.parent.frame();
        let height = f64::from(chip.h + self.gap()) + SHADOW_ROOM;
        // Wide as the window, or, while it takes a click, as the capsule and its room: see `set_clickable`.
        let (left, width, inset) = if self.clickable {
            (f64::from(chip.x) - SIDE_ROOM, f64::from(chip.w) + 2.0 * SIDE_ROOM, SIDE_ROOM)
        } else {
            (0.0, parent.size.width, f64::from(chip.x))
        };
        let panel = NSRect::new(
            NSPoint::new(
                parent.origin.x + left,
                parent.origin.y + parent.size.height - f64::from(top) - height,
            ),
            NSSize::new(width, height),
        );
        if self.panel.frame() != panel {
            self.panel.setFrame_display(panel, true);
        }
        let whole = NSRect::new(NSPoint::ZERO, panel.size);
        // The holder's layer is AppKit's own, and is being moved by an animation: its frame is set only when
        // it has changed.
        if self.holder.frame() != whole {
            self.holder.setFrame(whole);
        }

        let capsule = NSSize::new(f64::from(chip.w), f64::from(chip.h));
        let at = NSRect::new(NSPoint::new(inset, SHADOW_ROOM), capsule);
        let inside = NSRect::new(NSPoint::ZERO, capsule);
        self.icon.setFrame(geometry.icon);
        self.text.setFrame(geometry.text);
        self.press.setFrame(at);
        if animated {
            // The capsule and the label's frame move together, so the words are never ahead of the glass.
            NSAnimationContext::beginGrouping();
            NSAnimationContext::currentContext().setDuration(SWAP.as_secs_f64());
            self.backdrop.view().animator().setFrame(at);
            if !matches!(self.backdrop, Backdrop::Glass(_)) {
                self.label.animator().setFrame(inside);
            }
            NSAnimationContext::endGrouping();
        } else {
            self.backdrop.view().setFrame(at);
            self.label.setFrame(inside);
        }
    }
}

impl Drop for Chip {
    fn drop(&mut self) {
        if !self.taken_down {
            take_down(&self.parent, &self.panel);
        }
    }
}

/// Give `panel` to `parent` as a child, above it.
#[allow(unsafe_code)]
fn adopt(parent: &NSWindow, panel: &NSPanel) {
    // SAFETY: both windows are live and on the main thread (the executor's).
    unsafe { parent.addChildWindow_ordered(panel, NSWindowOrderingMode::Above) };
}

/// Take `panel` off `parent` and close it. AppKit orders a child out with its parent, but the chip doesn't
/// rely on that: it goes with its window.
fn take_down(parent: &NSWindow, panel: &NSPanel) {
    parent.removeChildWindow(panel);
    panel.orderOut(None);
    panel.close();
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
    fn the_words_get_what_the_capsule_has_beside_the_symbol() {
        let g = geometry(words(80.0), FIXED + 80.0);
        assert_eq!(g.text.size.width, 80.0);
    }

    #[test]
    fn the_symbol_and_the_words_sit_in_the_middle_of_the_height() {
        let g = geometry(words(80.0), FIXED + 80.0);
        assert_eq!(g.icon.origin.y, ((HEIGHT - ICON) / 2.0).floor());
        assert_eq!(g.text.origin.y, ((HEIGHT - 16.0) / 2.0).floor());
        assert!(g.icon.origin.x + g.icon.size.width + GAP <= g.text.origin.x + 1e-9);
    }

    #[test]
    fn words_wider_than_the_capsule_are_cut_and_the_symbol_stays() {
        let g = geometry(words(2000.0), 300.0);
        assert_eq!(g.text.size.width, 300.0 - FIXED);
        assert_eq!(g.icon.size.width, ICON);
    }

    #[test]
    fn a_capsule_with_no_room_for_words_has_none() {
        let g = geometry(words(50.0), 0.0);
        assert_eq!(g.text.size.width, 0.0);
    }
}
