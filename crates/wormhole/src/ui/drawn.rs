//! The line icons the design draws on its own 16-point grid (the settings sidebar's tiles and the like), as
//! assets, and the icon type a compiled component takes: one of the app's Lucide icons or one of these.
//! Their strokes are 1.6 wide as in the design; the colour is the text's.

use gpui_kit::SharedString;
use gpui_kit::assets::IconName;
use gpui_kit::component::IconNamed;

/// An icon of a compiled component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DesignIcon {
    /// One of the app's Lucide icons.
    Lucide(IconName),
    /// One of the design's own drawings (`ICONS`), by its asset path.
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
    use super::DesignIcon;

    pub(crate) const CHANGED: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/changed.svg");
    pub(crate) const TERMINAL: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/terminal.svg");
    pub(crate) const APPEARANCE: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/appearance.svg");
    pub(crate) const FONTS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/fonts.svg");
    pub(crate) const SESSIONS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/sessions.svg");
    pub(crate) const GIT: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/git.svg");
    pub(crate) const NOTIFICATIONS: DesignIcon =
        DesignIcon::Drawn("wormhole/glyphs/design/notifications.svg");
    pub(crate) const AGENTS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/agents.svg");
    pub(crate) const MENUBAR: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/menubar.svg");
    pub(crate) const IMPORT: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/import.svg");
    pub(crate) const UPDATES: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/updates.svg");
    pub(crate) const ADVANCED: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/advanced.svg");
    pub(crate) const MOTION: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/motion.svg");
    pub(crate) const PROJECTS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/projects.svg");
    pub(crate) const EXTENSIONS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/extensions.svg");
    pub(crate) const FLAGS: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/flags.svg");
    pub(crate) const SEARCH: DesignIcon = DesignIcon::Drawn("wormhole/glyphs/design/search.svg");
}

/// The drawings' bytes by asset path, for the app's asset source.
pub(crate) const ICONS: &[(&str, &[u8])] = &[
    ("wormhole/glyphs/design/changed.svg", include_bytes!("../../assets/glyphs/design/changed.svg")),
    ("wormhole/glyphs/design/terminal.svg", include_bytes!("../../assets/glyphs/design/terminal.svg")),
    ("wormhole/glyphs/design/appearance.svg", include_bytes!("../../assets/glyphs/design/appearance.svg")),
    ("wormhole/glyphs/design/fonts.svg", include_bytes!("../../assets/glyphs/design/fonts.svg")),
    ("wormhole/glyphs/design/sessions.svg", include_bytes!("../../assets/glyphs/design/sessions.svg")),
    ("wormhole/glyphs/design/git.svg", include_bytes!("../../assets/glyphs/design/git.svg")),
    (
        "wormhole/glyphs/design/notifications.svg",
        include_bytes!("../../assets/glyphs/design/notifications.svg"),
    ),
    ("wormhole/glyphs/design/agents.svg", include_bytes!("../../assets/glyphs/design/agents.svg")),
    ("wormhole/glyphs/design/menubar.svg", include_bytes!("../../assets/glyphs/design/menubar.svg")),
    ("wormhole/glyphs/design/import.svg", include_bytes!("../../assets/glyphs/design/import.svg")),
    ("wormhole/glyphs/design/updates.svg", include_bytes!("../../assets/glyphs/design/updates.svg")),
    ("wormhole/glyphs/design/advanced.svg", include_bytes!("../../assets/glyphs/design/advanced.svg")),
    ("wormhole/glyphs/design/motion.svg", include_bytes!("../../assets/glyphs/design/motion.svg")),
    ("wormhole/glyphs/design/projects.svg", include_bytes!("../../assets/glyphs/design/projects.svg")),
    ("wormhole/glyphs/design/extensions.svg", include_bytes!("../../assets/glyphs/design/extensions.svg")),
    ("wormhole/glyphs/design/flags.svg", include_bytes!("../../assets/glyphs/design/flags.svg")),
    ("wormhole/glyphs/design/search.svg", include_bytes!("../../assets/glyphs/design/search.svg")),
];

/// The bytes of the drawing at `path`.
pub(crate) fn bytes(path: &str) -> Option<&'static [u8]> {
    ICONS.iter().find(|(known, _)| *known == path).map(|(_, bytes)| *bytes)
}
