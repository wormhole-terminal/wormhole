//! The live previews of Menu bar and Hotkey, which run a clock each: one exists while its page is up
//! and not otherwise, as Motion's does (`MotionStrip`).

use gpui_kit::*;
use wormhole_core::config::settings_index::SectionId as Section;

use super::hotkey_preview::HotkeyPreview;
use super::menu_bar_preview::MenuBarPreview;

/// The preview of the page in front, if it is one of the two.
pub(in crate::settings_window) enum BarPreview {
    MenuBar(Entity<MenuBarPreview>),
    Hotkey(Entity<HotkeyPreview>),
}

impl BarPreview {
    /// The preview `section` has: the one already up if it is that page's, a new one for a page that
    /// has one, none for any other (the old one, and its clock, are dropped).
    pub(in crate::settings_window) fn for_section(
        section: Section,
        up: Option<BarPreview>,
        cx: &mut App,
    ) -> Option<BarPreview> {
        match (section, up) {
            (Section::MenuBar, Some(up @ BarPreview::MenuBar(_)))
            | (Section::Hotkey, Some(up @ BarPreview::Hotkey(_))) => Some(up),
            (Section::MenuBar, _) => Some(BarPreview::MenuBar(cx.new(MenuBarPreview::new))),
            (Section::Hotkey, _) => Some(BarPreview::Hotkey(cx.new(HotkeyPreview::new))),
            _ => None,
        }
    }

    /// The preview, to put in the strip.
    pub(in crate::settings_window) fn element(&self) -> AnyElement {
        match self {
            Self::MenuBar(view) => view.clone().into_any_element(),
            Self::Hotkey(view) => view.clone().into_any_element(),
        }
    }
}
