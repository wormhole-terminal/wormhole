//! The table: what every section and setting is called and says. The sections' names are here; each
//! section's groups of settings are in a file of their own.

use wormhole_text::Key;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::{Entry, Group, NavGroup, SectionId, SectionInfo};
use crate::config::FeatureId;

mod advanced;
mod agents;
mod appearance;
mod import;
mod menu_bar;
mod motion;
mod notifications;
mod profile;
mod sessions;
mod terminal;
mod updates;

/// A setting whose words are in the catalogs: `label` is the message and the quiet line under it and the
/// search words are its attributes (`.with_description`, `.with_keywords`).
const fn entry(id: &'static str, label: Key, keys: &'static [&'static str]) -> Entry {
    Entry { id, label, description: None, keywords: None, keys, feature: None }
}

impl Entry {
    /// About the feature `feature`.
    const fn of(mut self, feature: FeatureId) -> Self {
        self.feature = Some(feature);
        self
    }

    /// The quiet line under the label.
    const fn with_description(mut self, description: Key) -> Self {
        self.description = Some(description);
        self
    }

    /// The words that find it besides its label and description.
    const fn with_keywords(mut self, keywords: Key) -> Self {
        self.keywords = Some(keywords);
        self
    }
}

/// A titled run of settings, the title a message of the catalogs.
const fn group(title: Key, entries: &'static [Entry]) -> Group {
    Group { title: Some(title), blurb: None, aside: None, entries }
}

/// Settings without a heading over them.
const fn untitled(entries: &'static [Entry]) -> Group {
    Group { title: None, blurb: None, aside: None, entries }
}

impl Group {
    /// The paragraph under the title.
    const fn with_blurb(mut self, blurb: Key) -> Self {
        self.blurb = Some(blurb);
        self
    }

    /// The quiet note at the right of the title.
    const fn with_aside(mut self, aside: Key) -> Self {
        self.aside = Some(aside);
        self
    }
}

pub(super) fn section(id: SectionId) -> &'static SectionInfo {
    match id {
        SectionId::Profile => &SectionInfo {
            title: SETTINGS_SECTION_PROFILE,
            intro: SETTINGS_SECTION_PROFILE_INTRO,
            nav: NavGroup::Profile,
        },
        SectionId::Appearance => &SectionInfo {
            title: SETTINGS_SECTION_APPEARANCE,
            intro: SETTINGS_SECTION_APPEARANCE_INTRO,
            nav: NavGroup::Look,
        },
        SectionId::Fonts => &SectionInfo {
            title: SETTINGS_SECTION_FONTS,
            intro: SETTINGS_SECTION_FONTS_INTRO,
            nav: NavGroup::Look,
        },
        SectionId::Motion => &SectionInfo {
            title: SETTINGS_SECTION_MOTION,
            intro: SETTINGS_SECTION_MOTION_INTRO,
            nav: NavGroup::Look,
        },
        SectionId::Terminal => &SectionInfo {
            title: SETTINGS_SECTION_TERMINAL,
            intro: SETTINGS_SECTION_TERMINAL_INTRO,
            nav: NavGroup::Work,
        },
        SectionId::Sessions => &SectionInfo {
            title: SETTINGS_SECTION_SESSIONS,
            intro: SETTINGS_SECTION_SESSIONS_INTRO,
            nav: NavGroup::Work,
        },
        SectionId::Git => &SectionInfo {
            title: SETTINGS_SECTION_GIT,
            intro: SETTINGS_SECTION_GIT_INTRO,
            nav: NavGroup::Work,
        },
        SectionId::Agents => &SectionInfo {
            title: SETTINGS_SECTION_AGENTS,
            intro: SETTINGS_SECTION_AGENTS_INTRO,
            nav: NavGroup::Agents,
        },
        SectionId::Projects => &SectionInfo {
            title: SETTINGS_SECTION_PROJECTS,
            intro: SETTINGS_SECTION_PROJECTS_INTRO,
            nav: NavGroup::Preview,
        },
        SectionId::FeatureFlags => &SectionInfo {
            title: SETTINGS_SECTION_FEATURE_FLAGS,
            intro: SETTINGS_SECTION_FEATURE_FLAGS_INTRO,
            nav: NavGroup::Preview,
        },
        SectionId::Import => &SectionInfo {
            title: SETTINGS_SECTION_IMPORT,
            intro: SETTINGS_SECTION_IMPORT_INTRO,
            nav: NavGroup::App,
        },
        SectionId::Notifications => &SectionInfo {
            title: SETTINGS_SECTION_NOTIFICATIONS,
            intro: SETTINGS_SECTION_NOTIFICATIONS_INTRO,
            nav: NavGroup::App,
        },
        SectionId::MenuBar => &SectionInfo {
            title: SETTINGS_SECTION_MENU_BAR,
            intro: SETTINGS_SECTION_MENU_BAR_INTRO,
            nav: NavGroup::App,
        },
        SectionId::Updates => &SectionInfo {
            title: SETTINGS_SECTION_UPDATES,
            intro: SETTINGS_SECTION_UPDATES_INTRO,
            nav: NavGroup::App,
        },
        SectionId::Advanced => &SectionInfo {
            title: SETTINGS_SECTION_ADVANCED,
            intro: SETTINGS_SECTION_ADVANCED_INTRO,
            nav: NavGroup::App,
        },
    }
}

pub(super) fn groups(id: SectionId) -> &'static [Group] {
    match id {
        SectionId::Profile => profile::PROFILE,
        SectionId::Appearance => appearance::APPEARANCE,
        SectionId::Fonts => appearance::FONTS,
        SectionId::Motion => motion::MOTION,
        SectionId::Terminal => terminal::TERMINAL,
        SectionId::Sessions => sessions::SESSIONS,
        SectionId::Git => sessions::GIT,
        SectionId::Agents => agents::AGENTS,
        SectionId::Projects => agents::PROJECTS,
        SectionId::Import => import::IMPORT,
        SectionId::Notifications => notifications::NOTIFICATIONS,
        SectionId::MenuBar => menu_bar::MENU_BAR,
        SectionId::FeatureFlags => agents::FEATURE_FLAGS,
        SectionId::Updates => updates::UPDATES,
        SectionId::Advanced => advanced::ADVANCED,
    }
}
