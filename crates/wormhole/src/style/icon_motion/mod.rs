//! Icons answering with their own drawing: a + that turns on a click, arrows that spin, a copy that
//! becomes a check, a split filling the pane it makes, corners that swap places, a chevron that
//! turns over with its menu, a panel toggle's strip filling with its column, and a right panel
//! tab's glyph playing as the tab becomes the active one. Each is the icon's own drawing
//! changing; the icon never moves, shakes or travels as a whole.
//!
//! Every motion ends at the icon's usual look for its new state, and `UiColors::icon_motion` (the
//! Animate icons setting) turns them all off. Under Reduce Motion GPUI shows an animation's last
//! frame, which is that look, so nothing else is needed for it.
//!
//! One concept to a file: [`trigger`] says what starts a motion, [`timeline`] and [`canvas`] are
//! the maths and the grid the drawings share, and each motion has its own file.

mod canvas;
mod copied;
mod pane_fill;
mod strip;
mod swap;
mod tab_glyph;
mod timeline;
mod trigger;
mod turn;

use gpui_kit::assets::IconName;
use gpui_kit::*;

pub use pane_fill::Half;
pub use strip::{StripFill, StripSide};
pub(crate) use tab_glyph::TabGlyph;
pub use trigger::MenuMark;
use trigger::{Clicks, watch};
pub use turn::Spin;

/// What an [`IconButton`](super::IconButton)'s icon does to show an answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconMotion {
    /// On a click, a quarter turn: a + looks the same after it.
    Quarter,
    /// On a click, a whole turn the way the arrows curl.
    Spin(Spin),
    /// On a click, the copy icon fills and turns into a check, which turns back.
    Copied,
    /// On a click, the half of a split icon the new pane takes fills and lets go.
    NewPane(Half),
    /// When the button's icon changes, the old one shrinks away and the new one grows in.
    Swap,
    /// While the button's menu is open (see `IconButton::menu_chevron`), the chevron points up.
    MenuChevron,
}

impl IconMotion {
    /// The motion an icon has wherever it is a button, so the same icon answers the same way
    /// everywhere: the +, the refresh and reset arrows, and copy.
    pub fn of(icon: IconName) -> Option<Self> {
        match icon {
            IconName::Plus => Some(IconMotion::Quarter),
            IconName::RefreshCw => Some(IconMotion::Spin(Spin::Clockwise)),
            IconName::RotateCcw => Some(IconMotion::Spin(Spin::CounterClockwise)),
            IconName::Copy => Some(IconMotion::Copied),
            _ => None,
        }
    }

    /// Gets the motion ready for one render of the button keyed `key`: what started it and how many
    /// times. `open` is whether the button's menu is open.
    pub(in crate::style) fn arm(
        self,
        key: &ElementId,
        icon: IconName,
        open: bool,
        enabled: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Armed {
        let key = ElementId::from((key.clone(), "icon-motion"));
        let mut armed = Armed { motion: self, run: 0, before: icon, open, clicks: None };
        match self {
            IconMotion::Swap => {
                let change = watch(key, icon, enabled, window, cx);
                (armed.run, armed.before) = (change.run, change.before);
            }
            IconMotion::MenuChevron => armed.run = watch(key, open, enabled, window, cx).run,
            _ => {
                let clicks = Clicks::keyed(key, enabled, window, cx);
                armed.run = clicks.run(cx);
                armed.clicks = Some(clicks);
            }
        }
        armed
    }
}

/// A motion ready to draw: see [`IconMotion::arm`].
pub(in crate::style) struct Armed {
    motion: IconMotion,
    run: u64,
    /// The icon before the last change: what [`IconMotion::Swap`] shrinks away.
    before: IconName,
    open: bool,
    clicks: Option<Clicks>,
}

impl Armed {
    /// What a click is to tell, for the motions a click starts.
    pub(in crate::style) fn clicks(&self) -> Option<Clicks> {
        self.clicks.clone()
    }

    /// `icon` drawn `size` points square, in its motion.
    pub(in crate::style) fn glyph(&self, icon: IconName, size: f32) -> AnyElement {
        let run = self.run;
        match self.motion {
            IconMotion::Quarter => turn::quarter(icon, size, run),
            IconMotion::Spin(way) => turn::spin(icon, size, way, run),
            IconMotion::Copied => copied::glyph(size, run),
            IconMotion::NewPane(half) => pane_fill::glyph(icon, size, half, run),
            IconMotion::Swap => swap::glyph(icon, self.before, size, run),
            IconMotion::MenuChevron => turn::chevron(icon, size, self.open, run),
        }
    }
}
