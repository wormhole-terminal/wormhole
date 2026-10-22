//! Native background blur, and what the system says about it.
//!
//! GPUI's own `WindowBackgroundAppearance::Blurred` relies on private layer
//! tweaks that stopped working on macOS 27, so we insert a plain
//! `NSVisualEffectView` behind GPUI's view ourselves and keep the window
//! `Transparent`. Each window owns its [`Blur`], so the main, settings and
//! Theme Studio windows can blur independently.
//!
//! The material follows the theme, not the system: a light theme in Dark Mode sits on the
//! light HUD material. The appearance is set on the effect views alone, not on the
//! window, so the window keeps following the system and GPUI still reports its appearance
//! changes, which Match system listens to. The accessibility display options are read and
//! watched here: Reduce Transparency and Increase Contrast, which the settings store
//! turns into an opacity of 1 (hiding every blur) and the high-contrast tokens, and Reduce
//! Motion, which GPUI's animations follow.

use gpui_kit::Window;
use wormhole_core::tokens::Appearance;

/// The blur behind one window. Hidden until enabled; cheap to toggle. Under it lies a plain fill
/// (`set_backing`) for a window that has Liquid Glass but no blur: the glass is drawn below GPUI's
/// view, so what GPUI leaves see-through round it has to be painted by AppKit too.
#[derive(Default)]
pub struct Blur {
    #[cfg(target_os = "macos")]
    effect: Option<objc2::rc::Retained<objc2_app_kit::NSVisualEffectView>>,
    #[cfg(target_os = "macos")]
    backing: Option<objc2::rc::Retained<objc2_app_kit::NSBox>>,
    enabled: Option<bool>,
    backing_color: Option<Option<[u32; 4]>>,
}

impl Blur {
    #[cfg(target_os = "macos")]
    #[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
    pub fn install(window: &Window, enabled: bool) -> Self {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{
            NSAutoresizingMaskOptions, NSBox, NSBoxType, NSTitlePosition, NSView, NSVisualEffectBlendingMode,
            NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
        };

        let Some(mtm) = MainThreadMarker::new() else {
            return Self::default();
        };
        let Some(view) = super::macos::appkit::ns_view(window) else {
            return Self::default();
        };
        let view: &NSView = &view;
        // SAFETY: GPUI's view, on the main thread; AppKit only reads the hierarchy here.
        let Some(parent) = (unsafe { view.superview() }) else {
            return Self::default();
        };

        let effect = NSVisualEffectView::initWithFrame(mtm.alloc(), parent.bounds());
        effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::Active);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        effect.setHidden(!enabled);
        material::adopt(&effect);
        parent.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, Some(view));

        let backing = NSBox::initWithFrame(mtm.alloc(), parent.bounds());
        backing.setBoxType(NSBoxType::Custom);
        backing.setTitlePosition(NSTitlePosition::NoTitle);
        backing.setBorderWidth(0.0);
        backing.setContentViewMargins(objc2_foundation::NSSize::new(0.0, 0.0));
        backing.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        backing.setHidden(true);
        parent.addSubview_positioned_relativeTo(&backing, NSWindowOrderingMode::Below, Some(&effect));
        Self {
            effect: Some(effect),
            backing: Some(backing),
            enabled: Some(enabled),
            backing_color: Some(None),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn install(_window: &Window, enabled: bool) -> Self {
        Self { enabled: Some(enabled), backing_color: Some(None) }
    }

    /// Fill the window with `color` under the glass (`None`: no fill, the blur or nothing shows);
    /// a no-op when nothing changes.
    pub fn set_backing(&mut self, color: Option<gpui_kit::Hsla>) {
        let key = color.map(|c| {
            let c = c.to_rgb();
            [c.r, c.g, c.b, c.a].map(|v| (v * 1000.0).round() as u32)
        });
        if self.backing_color == Some(key) {
            return;
        }
        self.backing_color = Some(key);
        #[cfg(target_os = "macos")]
        if let Some(backing) = &self.backing {
            match color {
                Some(color) => {
                    let c = color.to_rgb();
                    let fill = objc2_app_kit::NSColor::colorWithSRGBRed_green_blue_alpha(
                        f64::from(c.r),
                        f64::from(c.g),
                        f64::from(c.b),
                        f64::from(c.a),
                    );
                    backing.setFillColor(&fill);
                    backing.setHidden(false);
                }
                None => backing.setHidden(true),
            }
        }
    }

    /// Show or hide the blur; a no-op when nothing changes, so views may call
    /// it on every render.
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == Some(enabled) {
            return;
        }
        self.enabled = Some(enabled);
        #[cfg(target_os = "macos")]
        if let Some(effect) = &self.effect {
            effect.setHidden(!enabled);
        }
    }
}

/// Liquid Glass (macOS 26): `NSGlassEffectView`s behind GPUI's view, one for each sidebar card,
/// each loose terminal, and each tab, chip and pile of the tab bars, floating over the window's
/// blur. GPUI paints those regions see-through and keeps only tints on top. Without the class
/// (before macOS 26) [`Glass::available`] is false and the windows keep today's blur.
///
/// A frame says where the regions are (`propose`, from the layout's own bounds, so the glass moves
/// with a sliding column, a scrolling list or a pile fanning out) and one last probe applies them
/// (`commit`); a region nobody proposed is hidden. Pieces proposed under one key (the stretches of
/// a card) join up. A region may name a clip (`clip_to`, a scrolling list's own room) it is cut
/// to. Glass views stack by key: a higher key lies over a lower one.
#[derive(Default)]
pub struct Glass {
    #[cfg(target_os = "macos")]
    native: std::cell::RefCell<Option<native::Views>>,
    pending: std::cell::RefCell<std::collections::BTreeMap<u64, (GlassRect, Option<u32>)>>,
    clips: std::cell::RefCell<std::collections::HashMap<u32, GlassRect>>,
    shown: std::cell::RefCell<Vec<GlassRect>>,
}

/// Where a glass sits, in the window's points from its top left, and how round it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radius: f32,
}

impl GlassRect {
    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn join(self, other: Self) -> Self {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        Self {
            x,
            y,
            w: self.right().max(other.right()) - x,
            h: self.bottom().max(other.bottom()) - y,
            radius: self.radius,
        }
    }

    fn clipped(self, to: Self) -> Option<Self> {
        let (x, y) = (self.x.max(to.x), self.y.max(to.y));
        let (w, h) = (self.right().min(to.right()) - x, self.bottom().min(to.bottom()) - y);
        (w > 0.0 && h > 0.0).then_some(Self { x, y, w, h, radius: self.radius })
    }
}

/// Whether this Mac has Liquid Glass (macOS 26 and later), for Settings, which has no window glass.
pub fn glass_available() -> bool {
    #[cfg(target_os = "macos")]
    return objc2::runtime::AnyClass::get(c"NSGlassEffectView").is_some();
    #[cfg(not(target_os = "macos"))]
    false
}

impl Glass {
    #[cfg(target_os = "macos")]
    pub fn install(window: &Window) -> Self {
        Self { native: std::cell::RefCell::new(native::Views::find(window)), ..Self::default() }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn install(_window: &Window) -> Self {
        Self::default()
    }

    /// Whether this Mac has Liquid Glass.
    pub fn available(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.native.borrow().is_some();
        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Clip `id` is `rect` this frame: the regions that name it are cut to it.
    pub fn clip_to(&self, id: u32, rect: GlassRect) {
        self.clips.borrow_mut().insert(id, rect);
    }

    pub fn propose(&self, key: u64, rect: GlassRect, clip: Option<u32>) {
        let mut pending = self.pending.borrow_mut();
        let rect = match pending.get(&key) {
            Some(&(have, _)) => have.join(rect),
            None => rect,
        };
        pending.insert(key, (rect, clip));
    }

    /// Move the glasses to what this frame proposed, and hide the rest.
    pub fn commit(&self) {
        let clips = std::mem::take(&mut *self.clips.borrow_mut());
        let next: Vec<GlassRect> = std::mem::take(&mut *self.pending.borrow_mut())
            .into_values()
            .filter_map(|(rect, clip)| match clip.and_then(|id| clips.get(&id)) {
                Some(&clip) => rect.clipped(clip),
                None => Some(rect),
            })
            .collect();
        if next == *self.shown.borrow() {
            return;
        }
        tracing::debug!(glasses = next.len(), "liquid glass placed");
        #[cfg(target_os = "macos")]
        if let Some(views) = self.native.borrow_mut().as_mut() {
            views.place(&next);
        }
        *self.shown.borrow_mut() = next;
    }
}

/// Every window's material follows `appearance` (the theme's) from now on, the windows open
/// now and the ones that open later.
pub fn set_material_appearance(appearance: Appearance) {
    #[cfg(target_os = "macos")]
    material::set(appearance);
    #[cfg(not(target_os = "macos"))]
    let _ = appearance;
}

#[cfg(target_os = "macos")]
mod native {
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2::runtime::AnyClass;
    use objc2_app_kit::{NSGlassEffectView, NSView, NSWindowOrderingMode};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    use super::{GlassRect, Window, material};

    /// GPUI's view, its parent, and the glass views made so far (a pool: the nth region uses the
    /// nth view, and the rest are hidden).
    pub(super) struct Views {
        mtm: MainThreadMarker,
        parent: Retained<NSView>,
        anchor: Retained<NSView>,
        glass: Vec<Retained<NSGlassEffectView>>,
    }

    impl Views {
        #[allow(unsafe_code)] // AppKit interop; GPUI hands out its live NSView on the main thread.
        pub(super) fn find(window: &Window) -> Option<Self> {
            // `NSGlassEffectView` only exists from macOS 26; naming the class on an older
            // system would abort, so ask the runtime first.
            AnyClass::get(c"NSGlassEffectView")?;
            let mtm = MainThreadMarker::new()?;
            let anchor = crate::platform::macos::appkit::ns_view(window)?;
            // SAFETY: see above.
            let parent = unsafe { anchor.superview() }?;
            Some(Self { mtm, parent, anchor, glass: Vec::new() })
        }

        pub(super) fn place(&mut self, rects: &[GlassRect]) {
            let base = self.anchor.frame();
            let flipped = self.parent.isFlipped();
            while self.glass.len() < rects.len() {
                let view = NSGlassEffectView::initWithFrame(self.mtm.alloc(), NSRect::ZERO);
                material::adopt(&view);
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
}

#[cfg(target_os = "macos")]
mod material {
    use std::cell::{Cell, RefCell};

    use objc2::rc::Weak;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSView,
    };
    use wormhole_core::tokens::Appearance;

    thread_local! {
        /// Every window's effect view (AppKit lives on the main thread), forgotten as windows close.
        static EFFECTS: RefCell<Vec<Weak<NSView>>> = const { RefCell::new(Vec::new()) };
        static APPEARANCE: Cell<Option<Appearance>> = const { Cell::new(None) };
    }

    /// A blur or glass view that follows the theme's appearance from now on.
    pub(super) fn adopt(effect: &NSView) {
        if let Some(appearance) = APPEARANCE.get() {
            apply(effect, appearance);
        }
        EFFECTS.with_borrow_mut(|effects| effects.push(Weak::new(effect)));
    }

    pub(super) fn set(appearance: Appearance) {
        if APPEARANCE.replace(Some(appearance)) == Some(appearance) {
            return;
        }
        EFFECTS.with_borrow_mut(|effects| {
            effects.retain(|weak| {
                let Some(effect) = weak.load() else { return false };
                apply(&effect, appearance);
                true
            });
        });
    }

    #[allow(unsafe_code)]
    fn apply(effect: &NSView, appearance: Appearance) {
        // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
        let name = unsafe {
            match appearance {
                Appearance::Light => NSAppearanceNameAqua,
                Appearance::Dark => NSAppearanceNameDarkAqua,
            }
        };
        effect.setAppearance(NSAppearance::appearanceNamed(name).as_deref());
    }
}

/// The system's appearance, light or dark. `None` where it can't be asked (off macOS, in tests).
pub fn system_appearance() -> Option<Appearance> {
    system::appearance()
}

/// Whether Reduce Transparency is on (System Settings → Accessibility → Display).
pub fn reduce_transparency() -> bool {
    system::reduce_transparency()
}

/// Whether Increase Contrast is on (System Settings → Accessibility → Display).
pub fn increase_contrast() -> bool {
    system::increase_contrast()
}

/// Whether Reduce Motion is on (System Settings → Accessibility → Display); `None` where it
/// can't be asked (off macOS, in tests), so GPUI's own flag is left as it is.
pub fn reduce_motion() -> Option<bool> {
    system::reduce_motion()
}

/// Calls `changed` on the main thread each time the system's display options change (Reduce
/// Transparency, Increase Contrast, Reduce Motion, …), for as long as the observer it returns
/// is kept.
pub fn observe_display_options(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
    system::observe(changed)
}

/// An observer of the display options; dropping it stops the calls.
pub struct DisplayObserver {
    #[cfg(all(target_os = "macos", not(test)))]
    token: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2::runtime::NSObjectProtocol>>,
}

#[cfg(all(target_os = "macos", not(test)))]
impl Drop for DisplayObserver {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        let center = objc2_app_kit::NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: `token` is the observer `addObserverForName…` returned on this center.
        unsafe { center.removeObserver(self.token.as_ref()) };
    }
}

#[cfg(all(target_os = "macos", not(test)))]
mod system {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSWorkspace,
        NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
    };
    use objc2_foundation::{NSArray, NSNotification, NSOperationQueue};
    use wormhole_core::tokens::Appearance;

    use super::DisplayObserver;

    #[allow(unsafe_code)]
    pub(super) fn appearance() -> Option<Appearance> {
        let mtm = MainThreadMarker::new()?;
        // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
        let (aqua, dark) = unsafe { (NSAppearanceNameAqua, NSAppearanceNameDarkAqua) };
        let names = NSArray::from_slice(&[aqua, dark]);
        // The app's own appearance is never set, so it is the system's. The best match folds
        // the high-contrast and vibrant variants into the two that matter here.
        let best = NSApplication::sharedApplication(mtm)
            .effectiveAppearance()
            .bestMatchFromAppearancesWithNames(&names)?;
        Some(if &*best == dark { Appearance::Dark } else { Appearance::Light })
    }

    pub(super) fn reduce_transparency() -> bool {
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
    }

    pub(super) fn increase_contrast() -> bool {
        NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast()
    }

    pub(super) fn reduce_motion() -> Option<bool> {
        Some(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
    }

    #[allow(unsafe_code)]
    pub(super) fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let block = RcBlock::new(move |_: NonNull<NSNotification>| changed());
        // SAFETY: a notification name AppKit defines, no sender filter, and the main queue, so
        // the block runs on the main thread; it is `Send + Sync` as the center requires.
        let token = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        Some(DisplayObserver { token })
    }
}

/// Off macOS, and in tests (which never ask the real system): nothing to ask or observe.
#[cfg(not(all(target_os = "macos", not(test))))]
mod system {
    use wormhole_core::tokens::Appearance;

    use super::DisplayObserver;

    pub(super) fn appearance() -> Option<Appearance> {
        None
    }

    pub(super) fn reduce_transparency() -> bool {
        false
    }

    pub(super) fn increase_contrast() -> bool {
        false
    }

    pub(super) fn reduce_motion() -> Option<bool> {
        None
    }

    pub(super) fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
        let _ = changed;
        None
    }
}
