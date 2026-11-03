//! Settings: a native macOS window drawn by SwiftUI (`macos/settings/`). The app keeps the settings and gives the
//! window a snapshot of them; the window answers each change of the person's as an event. Everything of that
//! exchange is in `native`; this is how the rest of the app opens the window.
//!
//! Above the window and the shared state: it may use them; the app root, the palette and the status item use it.

#[cfg(feature = "devctl")]
pub(crate) mod devctl;
mod native;

use gpui_kit::App;
use gpui_kit::assets::IconName;
use wormhole_core::config::settings_index;

use crate::settings::settings;

pub(crate) use wormhole_core::config::settings_index::SectionId as Section;

/// Bring up the Settings window (showing `section`), or bring it forward if it is open.
pub(crate) fn open_settings(section: Option<Section>, cx: &mut App) {
    native::open(section, cx);
}

/// Bring up Settings at the row `id` itself (the command palette): its page, scrolled to the row.
pub(crate) fn reveal_row(id: &str, cx: &mut App) {
    let Some((section, _, _)) = settings_index::entry(id) else { return };
    // Never open a section this build hides (a Preview section whose flag is off): a stale recent id could still
    // name one.
    if !section.is_visible(settings(cx)) {
        return;
    }
    // A setting the window has no row for is shown by its page alone.
    let row = native::row_of(id);
    native::show(
        native::Show {
            page: Some(
                row.and_then(|key| key.split(':').next())
                    .map_or_else(|| format!("{section:?}"), str::to_string),
            ),
            scroll: row.map(str::to_string),
            search: None,
        },
        cx,
    );
}

/// The "New profile" sheet over the Settings window, from anywhere (the File menu, the palette).
pub(crate) fn open_new_profile_sheet(cx: &mut App) {
    native::open_new_profile(cx);
}

/// The page the Settings window was last asked to show; `None` until it was opened at all.
#[cfg(test)]
pub(crate) fn shown_page(cx: &App) -> Option<String> {
    native::page(cx)
}

/// The icon the palette gives a section's settings.
pub(crate) fn section_icon(section: Section) -> IconName {
    match section {
        Section::Profile => IconName::CircleUser,
        Section::Changed => IconName::RotateCcw,
        Section::Appearance => IconName::Palette,
        Section::Fonts => IconName::ALargeSmall,
        Section::Motion => IconName::Play,
        Section::Terminal => IconName::SquareTerminal,
        Section::Sessions => IconName::PanelLeft,
        Section::Git => IconName::GitCompare,
        Section::Agents => IconName::Sparkles,
        Section::Import => IconName::ArrowDownToLine,
        Section::Projects => IconName::Folder,
        Section::Extensions => IconName::Puzzle,
        Section::FeatureFlags => IconName::FlaskConical,
        Section::Notifications => IconName::Bell,
        Section::MenuBar => IconName::Command,
        Section::Updates => IconName::Download,
        Section::Advanced => IconName::SlidersHorizontal,
    }
}
