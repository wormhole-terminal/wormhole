//! Liquid Glass behind the sidebar's cards and the tab bar's tabs (`vibrancy::Glass`): which
//! regions want it, the keys that tell them apart (and stack them), and the probes that tell it
//! where they are.

use super::*;
use crate::vibrancy::GlassRect;

/// How far a probe's glass is inset from its parent: left, top, right, bottom.
#[derive(Clone, Copy)]
pub(in crate::workspace) struct Insets(pub f32, pub f32, pub f32, pub f32);

impl Insets {
    pub(in crate::workspace) const NONE: Self = Self(0.0, 0.0, 0.0, 0.0);
}

/// The clips: the sidebar's list, and the tab bar's row of tabs (both cut what scrolls past them).
pub(in crate::workspace) const LIST_CLIP: u32 = 0;
pub(in crate::workspace) const TAB_ROW_CLIP: u32 = 1;

/// What a glass is for, in the key's top bits; the low ones tell the pieces of one kind apart. A
/// higher key lies over a lower one.
#[derive(Clone, Copy)]
pub(in crate::workspace) enum GlassKind {
    /// A sidebar card, by its band (1 up, so 0 stays free).
    Card = 0,
    /// A loose terminal in the sidebar, by its id.
    Loose = 1,
    /// A terminal's tab, by its id.
    Tab = 2,
}

pub(in crate::workspace) fn glass_key(kind: GlassKind, n: u64) -> u64 {
    ((kind as u64) << 40) | (n & ((1 << 40) - 1))
}

impl Workspace {
    /// Whether the sidebar's cards, the tab bars and the terminal's surface sit on glass: the
    /// Liquid Glass setting is on, the Mac has it and the system doesn't want windows opaque
    /// (Reduce Transparency, Increase Contrast), whatever the window's own blur and opacity are;
    /// without the blur the window gets a plain fill under the glass (`Blur::set_backing`). Else
    /// everything paints its own surface (the window's opacity and blur keep working as before).
    pub(in crate::workspace) fn glass_on(&self, cx: &App) -> bool {
        let store = store_in(&self.profile, cx);
        store.settings.glass_on(self.glass.available(), store.system().opaque())
    }

    /// A probe filling its parent that asks for glass under that region, inset, as the glass
    /// `key`: the stretches of one card all name it.
    pub(in crate::workspace) fn glass_probe(
        &self,
        key: u64,
        inset: Insets,
        radius: f32,
        clip: Option<u32>,
    ) -> AnyElement {
        Self::glass_probe_on(&self.glass, key, inset, radius, clip)
    }

    /// [`Self::glass_probe`] for a window's glass held apart from its workspace (a pile's cards
    /// ask for theirs while they animate).
    pub(in crate::workspace) fn glass_probe_on(
        glass: &std::rc::Rc<crate::vibrancy::Glass>,
        key: u64,
        inset: Insets,
        radius: f32,
        clip: Option<u32>,
    ) -> AnyElement {
        let glass = glass.clone();
        canvas(
            move |bounds, _, _| {
                let Insets(l, t, r, b) = inset;
                glass.propose(
                    key,
                    GlassRect {
                        x: f32::from(bounds.origin.x) + l,
                        y: f32::from(bounds.origin.y) + t,
                        w: (f32::from(bounds.size.width) - l - r).max(0.0),
                        h: (f32::from(bounds.size.height) - t - b).max(0.0),
                        radius,
                    },
                    clip,
                );
            },
            |_, _, _, _| {},
        )
        // Pinned to the corner: an absolute child otherwise starts after its parent's padding.
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }

    /// A probe filling its parent that makes it clip `id`: the glass of the regions that name it
    /// is cut to it (the sidebar's list, which scrolls under its header; a row of tabs).
    pub(in crate::workspace) fn glass_clip_probe(&self, id: u32) -> AnyElement {
        let glass = self.glass.clone();
        canvas(
            move |bounds, _, _| {
                glass.clip_to(
                    id,
                    GlassRect {
                        x: f32::from(bounds.origin.x),
                        y: f32::from(bounds.origin.y),
                        w: f32::from(bounds.size.width),
                        h: f32::from(bounds.size.height),
                        radius: 0.0,
                    },
                );
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }
}
