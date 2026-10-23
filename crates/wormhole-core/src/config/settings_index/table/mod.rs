//! The table: what every section and setting is called and says. The sections' names are here; each
//! section's groups of settings are in a file of their own.

use wormhole_text::Key;
use wormhole_text::keys::*;

use super::{Entry, Group, NavGroup, SectionId, SectionInfo, Word};
use crate::config::FeatureId;

mod advanced;
mod agents;
mod appearance;
mod menu_bar;
mod notifications;
mod profile;
mod sessions;
mod terminal;
mod updates;

/// A setting whose words are in the catalogs: `label` is the message and the quiet line under it and the
/// search words are its attributes (`.with_description`, `.with_keywords`).
const fn entry(id: &'static str, label: Key, keys: &'static [&'static str]) -> Entry {
    Entry { id, label: Word::Key(label), description: Word::NONE, keywords: Word::NONE, keys, feature: None }
}

/// A setting of a section whose words are still in Rust.
const fn literal_entry(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    keywords: &'static str,
    keys: &'static [&'static str],
) -> Entry {
    Entry {
        id,
        label: Word::Literal(label),
        description: Word::Literal(description),
        keywords: Word::Literal(keywords),
        keys,
        feature: None,
    }
}

impl Entry {
    /// About the feature `feature`.
    const fn of(mut self, feature: FeatureId) -> Self {
        self.feature = Some(feature);
        self
    }

    /// The quiet line under the label.
    const fn with_description(mut self, description: Key) -> Self {
        self.description = Word::Key(description);
        self
    }

    /// The words that find it besides its label and description.
    const fn with_keywords(mut self, keywords: Key) -> Self {
        self.keywords = Word::Key(keywords);
        self
    }
}

/// A titled run of settings, the title a message of the catalogs.
const fn group(title: Key, entries: &'static [Entry]) -> Group {
    Group { title: Word::Key(title), blurb: Word::NONE, aside: Word::NONE, entries }
}

/// Settings without a heading over them.
const fn untitled(entries: &'static [Entry]) -> Group {
    Group { title: Word::NONE, blurb: Word::NONE, aside: Word::NONE, entries }
}

/// A group of a section whose words are still in Rust.
const fn literal_group(title: &'static str, entries: &'static [Entry]) -> Group {
    Group { title: Word::Literal(title), blurb: Word::NONE, aside: Word::NONE, entries }
}

impl Group {
    /// The paragraph under the title.
    const fn with_blurb(mut self, blurb: Key) -> Self {
        self.blurb = Word::Key(blurb);
        self
    }

    /// The quiet note at the right of the title.
    const fn with_aside(mut self, aside: Key) -> Self {
        self.aside = Word::Key(aside);
        self
    }
}

pub(super) fn section(id: SectionId) -> &'static SectionInfo {
    match id {
        SectionId::Profile => &SectionInfo {
            title: Word::Literal("Profile"),
            intro: Word::Literal(
                "Every page here is this profile's. A new profile starts as a copy of one you have, \
                    without its terminals or agent logins.",
            ),
            nav: NavGroup::Profile,
        },
        SectionId::Appearance => &SectionInfo {
            title: Word::Literal("Appearance"),
            intro: Word::Literal("What every window looks like. Changes apply as you make them."),
            nav: NavGroup::Look,
        },
        SectionId::Fonts => &SectionInfo {
            title: Word::Literal("Fonts"),
            intro: Word::Literal(
                "Terminal first: it's what you read most. Code follows it and Interface uses the \
                    system font until you pick one.",
            ),
            nav: NavGroup::Look,
        },
        SectionId::Terminal => &SectionInfo {
            title: Word::Literal("Terminal"),
            intro: Word::Literal("How the terminal behaves. Its font is in Fonts."),
            nav: NavGroup::Work,
        },
        SectionId::Sessions => &SectionInfo {
            title: Word::Literal("Terminals & sidebar"),
            intro: Word::Literal("The Terminals list on the left: how it's grouped and what each row shows."),
            nav: NavGroup::Work,
        },
        SectionId::Git => &SectionInfo {
            title: Word::Literal("Changes & diffs"),
            intro: Word::Literal("The Changes panel and the diff view."),
            nav: NavGroup::Work,
        },
        SectionId::Agents => &SectionInfo {
            title: Word::Literal("Coding agents"),
            intro: Word::Literal(
                "Wormhole recognises the agent running in each terminal. With status hooks it \
                    knows exactly when it's working, waiting for you, or done.",
            ),
            nav: NavGroup::Agents,
        },
        SectionId::Projects => &SectionInfo {
            title: Word::Literal("Project folders"),
            intro: Word::Literal(
                "Change the defaults for every project, or pick one to set it up just for you.",
            ),
            nav: NavGroup::Preview,
        },
        SectionId::FeatureFlags => &SectionInfo {
            title: Word::Literal("Feature flags"),
            intro: Word::Literal(
                "Features still being built. They show here in a development build and on the \
                    Unstable update channel; otherwise a release has them off until they're finished.",
            ),
            nav: NavGroup::Preview,
        },
        SectionId::Notifications => &SectionInfo {
            title: Word::Literal("Notifications"),
            intro: Word::Literal("What Wormhole tells you about terminals you aren't looking at."),
            nav: NavGroup::App,
        },
        SectionId::MenuBar => &SectionInfo {
            title: Word::Literal("Menu bar & hotkey"),
            intro: Word::Literal("Wormhole in the macOS menu bar, and a shortcut that works from any app."),
            nav: NavGroup::App,
        },
        SectionId::Updates => &SectionInfo {
            title: Word::Key(SETTINGS_SECTION_UPDATES),
            intro: Word::Key(SETTINGS_SECTION_UPDATES_INTRO),
            nav: NavGroup::App,
        },
        SectionId::Advanced => &SectionInfo {
            title: Word::Literal("Advanced"),
            intro: Word::Literal("Files, folders and a way back to the defaults."),
            nav: NavGroup::App,
        },
    }
}

pub(super) fn groups(id: SectionId) -> &'static [Group] {
    match id {
        SectionId::Profile => profile::PROFILE,
        SectionId::Appearance => appearance::APPEARANCE,
        SectionId::Fonts => appearance::FONTS,
        SectionId::Terminal => terminal::TERMINAL,
        SectionId::Sessions => sessions::SESSIONS,
        SectionId::Git => sessions::GIT,
        SectionId::Agents => agents::AGENTS,
        SectionId::Projects => agents::PROJECTS,
        SectionId::Notifications => notifications::NOTIFICATIONS,
        SectionId::MenuBar => menu_bar::MENU_BAR,
        SectionId::FeatureFlags => agents::FEATURE_FLAGS,
        SectionId::Updates => updates::UPDATES,
        SectionId::Advanced => advanced::ADVANCED,
    }
}
