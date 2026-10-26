//! A right panel tab's glyph playing once as its tab becomes the active one: each icon does what it
//! is about, drawn from pieces of the icon itself (`icons::part`) laid over it. Git-compare's
//! two commits fill in turn, the commit's dot fills as its lines draw out, the gauge's needle
//! sweeps up, the page's lines are written, and the folder flashes full. None plays when the
//! window opens or the panel comes back, only on the tab becoming active, and each ends on the
//! plain icon.
//!
//! Which one an icon plays is decided by the icon, so the tab bar needn't know which feature a tab
//! belongs to, and an icon means the same motion wherever it is a tab.

mod commit;
mod compare;
mod folder;
mod gauge;
mod notes;

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable, Size};
use gpui_kit::*;

use super::canvas::Canvas;
use super::trigger::watch;

/// The tab icons that play something on becoming active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Glyph {
    Compare,
    Commit,
    Gauge,
    Notes,
    Folder,
}

impl Glyph {
    fn of(icon: IconName) -> Option<Self> {
        Some(match icon {
            IconName::GitCompare => Glyph::Compare,
            IconName::GitCommitHorizontal => Glyph::Commit,
            IconName::Gauge => Glyph::Gauge,
            IconName::FileText => Glyph::Notes,
            IconName::Folder => Glyph::Folder,
            _ => return None,
        })
    }

    fn glyph(self, canvas: Canvas, run: u64) -> AnyElement {
        match self {
            Glyph::Compare => compare::glyph(canvas, run),
            Glyph::Commit => commit::glyph(canvas, run),
            Glyph::Gauge => gauge::glyph(canvas, run),
            Glyph::Notes => notes::glyph(canvas, run),
            Glyph::Folder => folder::glyph(canvas, run),
        }
    }
}

/// The icon of a right panel tab (`style::icon_tab` draws it): the tab's text color, 1 rem
/// square, and its motion when `active` has just become true.
#[derive(IntoElement)]
pub(crate) struct TabGlyph {
    key: ElementId,
    icon: IconName,
    active: bool,
    enabled: bool,
}

impl TabGlyph {
    /// `key` names the tab, `enabled` is the Animate icons setting.
    pub(crate) fn new(key: impl Into<ElementId>, icon: IconName, active: bool, enabled: bool) -> Self {
        TabGlyph { key: key.into(), icon, active, enabled }
    }
}

impl RenderOnce for TabGlyph {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let rest = Icon::new(self.icon).with_size(Size::Medium);
        let Some(glyph) = Glyph::of(self.icon) else { return rest.into_any_element() };
        let change = watch((self.key, "icon-motion"), self.active, self.enabled, window, cx);
        // Only a tab that has become active plays; one that has stopped being it is at rest.
        if !self.active || change.run == 0 {
            return rest.into_any_element();
        }
        // `Size::Medium` is a rem.
        glyph.glyph(Canvas::new(f32::from(window.rem_size())), change.run)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::assets::IconName;

    use super::Glyph;

    #[test]
    fn the_icons_of_the_right_panels_tabs_each_have_a_motion_and_the_others_none() {
        for icon in [
            IconName::GitCompare,
            IconName::GitCommitHorizontal,
            IconName::Gauge,
            IconName::FileText,
            IconName::Folder,
        ] {
            assert!(Glyph::of(icon).is_some(), "{icon:?}");
        }
        assert!(Glyph::of(IconName::Inbox).is_none());
    }
}
