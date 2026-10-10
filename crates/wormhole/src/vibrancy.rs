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

/// The blur behind one window. Hidden until enabled; cheap to toggle.
#[derive(Default)]
pub struct Blur {
    #[cfg(target_os = "macos")]
    effect: Option<objc2::rc::Retained<objc2_app_kit::NSVisualEffectView>>,
    enabled: Option<bool>,
}

impl Blur {
    #[cfg(target_os = "macos")]
    #[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
    pub fn install(window: &Window, enabled: bool) -> Self {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{
            NSAutoresizingMaskOptions, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial,
            NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
        };
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let Some(mtm) = MainThreadMarker::new() else {
            return Self::default();
        };
        let Ok(handle) = HasWindowHandle::window_handle(window) else {
            return Self::default();
        };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Self::default();
        };
        // SAFETY: GPUI hands out its live NSView and we are on the main thread.
        let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
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
        Self { effect: Some(effect), enabled: Some(enabled) }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn install(_window: &Window, enabled: bool) -> Self {
        Self { enabled: Some(enabled) }
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

/// Every window's material follows `appearance` (the theme's) from now on, the windows open
/// now and the ones that open later.
pub fn set_material_appearance(appearance: Appearance) {
    #[cfg(target_os = "macos")]
    material::set(appearance);
    #[cfg(not(target_os = "macos"))]
    let _ = appearance;
}

#[cfg(target_os = "macos")]
mod material {
    use std::cell::{Cell, RefCell};

    use objc2::rc::Weak;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSVisualEffectView,
    };
    use wormhole_core::tokens::Appearance;

    thread_local! {
        /// Every window's effect view (AppKit lives on the main thread), forgotten as windows close.
        static EFFECTS: RefCell<Vec<Weak<NSVisualEffectView>>> = const { RefCell::new(Vec::new()) };
        static APPEARANCE: Cell<Option<Appearance>> = const { Cell::new(None) };
    }

    pub(super) fn adopt(effect: &objc2::rc::Retained<NSVisualEffectView>) {
        if let Some(appearance) = APPEARANCE.get() {
            apply(effect, appearance);
        }
        EFFECTS.with_borrow_mut(|effects| effects.push(Weak::from_retained(effect)));
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
    fn apply(effect: &NSVisualEffectView, appearance: Appearance) {
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
