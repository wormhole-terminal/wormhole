//! Liquid Glass behind the sidebar's cards and the tab bar's tabs (`vibrancy::Glass`): the keys that
//! tell the regions apart (and stack them), and the probes that tell the glass where they are. Whether
//! the window wants glass at all is the window's (`window::glass`).

use gpui_kit::*;

use crate::platform::vibrancy::GlassRect;

/// How far a probe's glass is inset from its parent: left, top, right, bottom.
#[derive(Clone, Copy)]
pub(super) struct Insets(pub f32, pub f32, pub f32, pub f32);

impl Insets {
    pub(super) const NONE: Self = Self(0.0, 0.0, 0.0, 0.0);
}

/// The clips: the sidebar's list, and the tab bar's row of tabs (both cut what scrolls past them).
pub(super) const LIST_CLIP: u32 = 0;
pub(super) const TAB_ROW_CLIP: u32 = 1;

/// What a glass is for, in the key's top bits; the low ones tell the pieces of one kind apart. A
/// higher key lies over a lower one.
#[derive(Clone, Copy)]
pub(super) enum GlassKind {
    /// A sidebar card, by its band (1 up, so 0 stays free).
    Card = 0,
    /// A loose terminal in the sidebar, by its id.
    Loose = 1,
    /// A terminal's tab, by its id.
    Tab = 2,
}

pub(super) fn glass_key(kind: GlassKind, n: u64) -> u64 {
    ((kind as u64) << 40) | (n & ((1 << 40) - 1))
}

/// A probe filling its parent that asks for glass under that region, inset, as the glass `key`, on a
/// window's glass held apart from its workspace (the sidebar's cards, a pile's cards).
pub(super) fn probe(
    glass: &std::rc::Rc<crate::platform::vibrancy::Glass>,
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

/// A probe filling its parent that makes it clip `id`: the glass of the regions that name it is cut to
/// it (the sidebar's list, which scrolls under its header; a row of tabs).
pub(super) fn clip_probe(glass: &std::rc::Rc<crate::platform::vibrancy::Glass>, id: u32) -> AnyElement {
    let glass = glass.clone();
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
