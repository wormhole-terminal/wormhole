//! The table: what every section and setting is called and says. The sections' names are here; each
//! section's groups of settings are in a file of their own.

use super::{Entry, Group, NavGroup, SectionId, SectionInfo};
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

const fn entry(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    keywords: &'static str,
    keys: &'static [&'static str],
) -> Entry {
    Entry { id, label, description, keywords, keys, feature: None }
}

impl Entry {
    /// About the feature `feature`.
    const fn of(mut self, feature: FeatureId) -> Self {
        self.feature = Some(feature);
        self
    }
}

const fn group(title: &'static str, entries: &'static [Entry]) -> Group {
    Group { title, blurb: "", aside: "", entries }
}

pub(super) fn section(id: SectionId) -> &'static SectionInfo {
    match id {
        SectionId::Profile => &SectionInfo {
            title: "Profile",
            intro: "Every page here is this profile's. A new profile starts as a copy of one you have, \
                    without its terminals or agent logins.",
            nav: NavGroup::Profile,
        },
        SectionId::Appearance => &SectionInfo {
            title: "Appearance",
            intro: "What every window looks like. Changes apply as you make them.",
            nav: NavGroup::Look,
        },
        SectionId::Fonts => &SectionInfo {
            title: "Fonts",
            intro: "Terminal first: it's what you read most. Code follows it and Interface uses the \
                    system font until you pick one.",
            nav: NavGroup::Look,
        },
        SectionId::Terminal => &SectionInfo {
            title: "Terminal",
            intro: "How the terminal behaves. Its font is in Fonts.",
            nav: NavGroup::Work,
        },
        SectionId::Sessions => &SectionInfo {
            title: "Terminals & sidebar",
            intro: "The Terminals list on the left: how it's grouped and what each row shows.",
            nav: NavGroup::Work,
        },
        SectionId::Git => &SectionInfo {
            title: "Changes & diffs",
            intro: "The Changes panel and the diff view.",
            nav: NavGroup::Work,
        },
        SectionId::Agents => &SectionInfo {
            title: "Coding agents",
            intro: "Wormhole recognises the agent running in each terminal. With status hooks it \
                    knows exactly when it's working, waiting for you, or done.",
            nav: NavGroup::Agents,
        },
        SectionId::Projects => &SectionInfo {
            title: "Project folders",
            intro: "Change the defaults for every project, or pick one to set it up just for you.",
            nav: NavGroup::Preview,
        },
        SectionId::FeatureFlags => &SectionInfo {
            title: "Feature flags",
            intro: "Features still being built. They show here in a development build and on the \
                    Unstable update channel; otherwise a release has them off until they're finished.",
            nav: NavGroup::Preview,
        },
        SectionId::Notifications => &SectionInfo {
            title: "Notifications",
            intro: "What Wormhole tells you about terminals you aren't looking at.",
            nav: NavGroup::App,
        },
        SectionId::MenuBar => &SectionInfo {
            title: "Menu bar & hotkey",
            intro: "Wormhole in the macOS menu bar, and a shortcut that works from any app.",
            nav: NavGroup::App,
        },
        SectionId::Updates => &SectionInfo {
            title: "Updates",
            intro: "Signed releases, checked in the background.",
            nav: NavGroup::App,
        },
        SectionId::Advanced => &SectionInfo {
            title: "Advanced",
            intro: "Files, folders and a way back to the defaults.",
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
