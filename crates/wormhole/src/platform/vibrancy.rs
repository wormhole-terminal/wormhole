//! Native background blur and Liquid Glass. (What the system says about how windows look, Reduce
//! Transparency and the rest, is `look`.)
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
//! changes, which Match system listens to.
//!
//! Where the system has no blur of its own (`fallback`), a window has none: GPUI's
//! `set_background_appearance` is the way a port would add it.

use gpui_kit::Window;
use wormhole_core::tokens::Appearance;

use super::os::vibrancy as os;

/// The blur behind one window. Hidden until enabled; cheap to toggle. Under it lies a plain fill
/// (`set_backing`) for a window that has Liquid Glass but no blur: the glass is drawn below GPUI's
/// view, so what GPUI leaves see-through round it has to be painted by AppKit too.
#[derive(Default)]
pub struct Blur {
    views: Option<os::BlurViews>,
    enabled: Option<bool>,
    backing_color: Option<Option<[u32; 4]>>,
}

impl Blur {
    pub fn install(window: &Window, enabled: bool) -> Self {
        match os::BlurViews::install(window, enabled) {
            Some(views) => Self { views: Some(views), enabled: Some(enabled), backing_color: Some(None) },
            None => Self::default(),
        }
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
        if let Some(views) = &self.views {
            views.set_backing(color);
        }
    }

    /// Show or hide the blur; a no-op when nothing changes, so views may call
    /// it on every render.
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == Some(enabled) {
            return;
        }
        self.enabled = Some(enabled);
        if let Some(views) = &self.views {
            views.set_enabled(enabled);
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
    native: std::cell::RefCell<Option<os::GlassViews>>,
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
    os::glass_available()
}

impl Glass {
    pub fn install(window: &Window) -> Self {
        Self { native: std::cell::RefCell::new(os::GlassViews::find(window)), ..Self::default() }
    }

    /// Whether this Mac has Liquid Glass.
    pub fn available(&self) -> bool {
        self.native.borrow().is_some()
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
        if let Some(views) = self.native.borrow_mut().as_mut() {
            views.place(&next);
        }
        *self.shown.borrow_mut() = next;
    }
}

/// Every window's material follows `appearance` (the theme's) from now on, the windows open
/// now and the ones that open later.
pub fn set_material_appearance(appearance: Appearance) {
    os::set_material_appearance(appearance);
}
