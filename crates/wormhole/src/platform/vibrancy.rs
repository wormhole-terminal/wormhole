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
use wormhole_core::theme::tokens::Appearance;

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
///
/// A part of the window that GPUI replays instead of drawing (a cached view) proposes nothing in a
/// frame it is replayed. Such a part brackets what it proposes ([`begin_region`](Self::begin_region),
/// [`end_region`](Self::end_region)) and the glass keeps those proposals until the part proposes again
/// or is [forgotten](Self::forget_region).
#[derive(Default)]
pub struct Glass {
    native: std::cell::RefCell<Option<os::GlassViews>>,
    /// What this frame proposed outside any region.
    now: std::cell::RefCell<Proposals>,
    /// What each region proposed when it was last drawn, kept while it is replayed.
    kept: std::cell::RefCell<std::collections::HashMap<RegionId, Proposals>>,
    /// The region whose proposals are being made, between `begin_region` and `end_region`.
    recording: std::cell::Cell<Option<RegionId>>,
    shown: std::cell::RefCell<Vec<GlassRect>>,
}

/// A part of the window whose glass is kept while GPUI replays it ([`Glass::begin_region`]). Not a clip's
/// id ([`Glass::clip_to`]), which names a room a list scrolls in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegionId(u32);

impl RegionId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// The glasses and the clips a part of a frame asked for.
#[derive(Default, Clone)]
struct Proposals {
    glasses: std::collections::BTreeMap<u64, (GlassRect, Option<u32>)>,
    clips: std::collections::HashMap<u32, GlassRect>,
}

impl Proposals {
    fn propose(&mut self, key: u64, rect: GlassRect, clip: Option<u32>) {
        let rect = match self.glasses.get(&key) {
            Some(&(have, _)) => have.join(rect),
            None => rect,
        };
        self.glasses.insert(key, (rect, clip));
    }
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

    /// Where the glasses are now, for devctl.
    #[cfg(feature = "devctl")]
    pub fn placed(&self) -> Vec<GlassRect> {
        self.shown.borrow().clone()
    }

    /// Whether this Mac has Liquid Glass.
    pub fn available(&self) -> bool {
        self.native.borrow().is_some()
    }

    /// Clip `id` is `rect` this frame: the regions that name it are cut to it.
    pub fn clip_to(&self, id: u32, rect: GlassRect) {
        self.with_proposals(|proposals| {
            proposals.clips.insert(id, rect);
        });
    }

    pub fn propose(&self, key: u64, rect: GlassRect, clip: Option<u32>) {
        self.with_proposals(|proposals| proposals.propose(key, rect, clip));
    }

    /// `f` on the proposals of the region being recorded, else of this frame.
    fn with_proposals(&self, f: impl FnOnce(&mut Proposals)) {
        match self.recording.get() {
            Some(region) => f(self.kept.borrow_mut().entry(region).or_default()),
            None => f(&mut self.now.borrow_mut()),
        }
    }

    /// What follows, up to [`end_region`](Self::end_region), is `region` drawing itself: its earlier
    /// proposals are replaced by these.
    pub fn begin_region(&self, region: RegionId) {
        self.kept.borrow_mut().insert(region, Proposals::default());
        self.recording.set(Some(region));
    }

    pub fn end_region(&self) {
        self.recording.set(None);
    }

    /// `region` is not in the window now: its proposals no longer count.
    pub fn forget_region(&self, region: RegionId) {
        self.kept.borrow_mut().remove(&region);
    }

    /// Move the glasses to what this frame proposed, and hide the rest.
    pub fn commit(&self) {
        let mut all = std::mem::take(&mut *self.now.borrow_mut());
        for kept in self.kept.borrow().values() {
            all.clips.extend(kept.clips.iter().map(|(&id, &rect)| (id, rect)));
            for (&key, &(rect, clip)) in &kept.glasses {
                all.propose(key, rect, clip);
            }
        }
        let Proposals { glasses, clips } = all;
        let next: Vec<GlassRect> = glasses
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

#[cfg(test)]
mod tests {
    use super::{Glass, GlassRect, RegionId};

    fn rect(x: f32) -> GlassRect {
        GlassRect { x, y: 0.0, w: 10.0, h: 10.0, radius: 0.0 }
    }

    /// A part GPUI replays proposes nothing in that frame: its glass stays until it draws again or
    /// leaves the window.
    #[test]
    fn a_region_that_is_replayed_keeps_its_glass_until_it_draws_again_or_is_forgotten() {
        let glass = Glass::default();
        glass.begin_region(RegionId::new(1));
        glass.propose(10, rect(0.0), None);
        glass.end_region();
        glass.propose(20, rect(50.0), None);
        glass.commit();
        assert_eq!(*glass.shown.borrow(), [rect(0.0), rect(50.0)]);
        // A frame in which only a loose probe proposes: the region was replayed.
        glass.propose(20, rect(60.0), None);
        glass.commit();
        assert_eq!(*glass.shown.borrow(), [rect(0.0), rect(60.0)], "the replayed region's glass stays");
        // Drawn again, it replaces what it proposed before.
        glass.begin_region(RegionId::new(1));
        glass.propose(10, rect(5.0), None);
        glass.end_region();
        glass.commit();
        assert_eq!(*glass.shown.borrow(), [rect(5.0)], "and the loose one, not proposed, is hidden");
        glass.forget_region(RegionId::new(1));
        glass.commit();
        assert!(glass.shown.borrow().is_empty(), "a region that left the window shows no glass");
    }

    /// A clip named by a region's probe cuts that region's glass in frames it is replayed too.
    #[test]
    fn a_replayed_regions_clip_still_cuts_its_glass() {
        let glass = Glass::default();
        glass.begin_region(RegionId::new(1));
        glass.clip_to(0, GlassRect { x: 0.0, y: 0.0, w: 10.0, h: 5.0, radius: 0.0 });
        glass.propose(10, rect(0.0), Some(0));
        glass.end_region();
        glass.commit();
        glass.commit();
        assert_eq!(glass.shown.borrow()[0].h, 5.0);
    }
}
