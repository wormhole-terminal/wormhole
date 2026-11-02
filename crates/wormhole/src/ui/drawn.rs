//! The icon type a compiled component takes: one of the app's Lucide icons or one of the design's own line
//! drawings (`icons::design`, drawn on the design's 16-point grid).

use gpui_kit::SharedString;
use gpui_kit::assets::IconName;
use gpui_kit::component::IconNamed;

use crate::icons;

/// An icon of a compiled component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DesignIcon {
    /// One of the app's Lucide icons.
    Lucide(IconName),
    /// One of the design's own drawings, by its asset path.
    Drawn(&'static str),
}

impl From<IconName> for DesignIcon {
    fn from(icon: IconName) -> Self {
        Self::Lucide(icon)
    }
}

impl IconNamed for DesignIcon {
    fn path(self) -> SharedString {
        match self {
            Self::Lucide(icon) => icon.path(),
            Self::Drawn(path) => path.into(),
        }
    }
}

/// The settings sidebar's tile icons and the search field's.
pub(crate) mod design {
    use super::{DesignIcon, icons};

    pub(crate) const CHANGED: DesignIcon = DesignIcon::Drawn(icons::design::CHANGED);
    pub(crate) const TERMINAL: DesignIcon = DesignIcon::Drawn(icons::design::TERMINAL);
    pub(crate) const APPEARANCE: DesignIcon = DesignIcon::Drawn(icons::design::APPEARANCE);
    pub(crate) const FONTS: DesignIcon = DesignIcon::Drawn(icons::design::FONTS);
    pub(crate) const SESSIONS: DesignIcon = DesignIcon::Drawn(icons::design::SESSIONS);
    pub(crate) const GIT: DesignIcon = DesignIcon::Drawn(icons::design::GIT);
    pub(crate) const NOTIFICATIONS: DesignIcon = DesignIcon::Drawn(icons::design::NOTIFICATIONS);
    pub(crate) const AGENTS: DesignIcon = DesignIcon::Drawn(icons::design::AGENTS);
    pub(crate) const MENUBAR: DesignIcon = DesignIcon::Drawn(icons::design::MENUBAR);
    pub(crate) const IMPORT: DesignIcon = DesignIcon::Drawn(icons::design::IMPORT);
    pub(crate) const UPDATES: DesignIcon = DesignIcon::Drawn(icons::design::UPDATES);
    pub(crate) const ADVANCED: DesignIcon = DesignIcon::Drawn(icons::design::ADVANCED);
    pub(crate) const MOTION: DesignIcon = DesignIcon::Drawn(icons::design::MOTION);
    pub(crate) const PROJECTS: DesignIcon = DesignIcon::Drawn(icons::design::PROJECTS);
    pub(crate) const EXTENSIONS: DesignIcon = DesignIcon::Drawn(icons::design::EXTENSIONS);
    pub(crate) const FLAGS: DesignIcon = DesignIcon::Drawn(icons::design::FLAGS);
    pub(crate) const SEARCH: DesignIcon = DesignIcon::Drawn(icons::design::SEARCH);
}
