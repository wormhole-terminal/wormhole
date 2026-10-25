//! The preview column: the right-hand strip of a wide Settings window that always shows the live
//! preview that belongs to the page. The window opens wider to the right by the column's width,
//! so the page keeps the width it always had; narrower than [`COLUMN_SWITCH_W`] the column goes
//! away and the same previews sit inline in their pages, as before.
//!
//! - Appearance, and a search's results, show the window in miniature (`sections::draw_stage`):
//!   theme, primary, opacity, status bar and fonts as they are now.
//! - Fonts shows the specimen (`sections::draw_specimen`), on the tab of the font being picked.
//! - Terminals & sidebar shows the Terminals list (`sections::draw_sidebar_preview`), beside the
//!   density under the pointer.
//! - Coding agents shows the Terminals list with agents at work in it (`draw_agents_preview`).
//! - Notifications shows two banners (`draw_notifications_preview`).
//! - Menu bar & hotkey shows the menu bar item and the quick panel (`draw_menu_bar_preview`).
//! - Every other page has nothing to show there: no column, and the page takes its width.
//!
//! The column holds nothing to operate: no focus stop, so ⇥ and F6 pass over it, and one labelled
//! group for VoiceOver. A search result for a preview names where it went instead of drawing it.

use gpui_kit::*;
use wormhole_core::config::settings_index::{self, SectionId as Section};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{PAGE_TOP, SettingsWindow, nav, sections, ui};
use crate::style::{self, Edges, SettingParts, SettingState, TextRole};

/// The column's width, border to border: a 16 pt inset round the miniature window.
pub(super) const PREVIEW_W: f32 = 448.0 + 2.0 * 16.0;
/// The window is at least this wide for the column to show; the page keeps 600 and more.
pub(super) const COLUMN_SWITCH_W: f32 = nav::NAV_W + 600.0 + PREVIEW_W;

/// Which preview the column shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PreviewKind {
    /// The window in miniature over a wallpaper.
    Window,
    /// The font specimen.
    Fonts,
    /// The Terminals list in the sidebar's own look.
    Sidebar,
    /// The Terminals list with coding agents at work.
    Agents,
    /// Notification banners.
    Notifications,
    /// The menu bar item and the quick panel.
    MenuBar,
}

impl PreviewKind {
    /// What the column's head says.
    fn title(self) -> &'static str {
        match self {
            Self::Window => text(SETTINGS_WINDOW_WINDOW),
            Self::Fonts => text(SETTINGS_WINDOW_FONTS),
            Self::Sidebar | Self::Agents => text(SETTINGS_WINDOW_TERMINALS_LIST),
            Self::Notifications => text(SETTINGS_WINDOW_NOTIFICATIONS),
            Self::MenuBar => text(SETTINGS_WINDOW_MENU_BAR),
        }
    }

    /// What VoiceOver says for the column: that it is a picture of the settings, not controls.
    fn spoken(self) -> &'static str {
        match self {
            Self::Window => text(SETTINGS_WINDOW_PREVIEW_WINDOW_LABEL),
            Self::Fonts => text(SETTINGS_WINDOW_PREVIEW_FONTS_LABEL),
            Self::Sidebar => text(SETTINGS_WINDOW_PREVIEW_TERMINALS_LABEL),
            Self::Agents => text(SETTINGS_WINDOW_PREVIEW_AGENTS_LABEL),
            Self::Notifications => text(SETTINGS_WINDOW_PREVIEW_NOTIFICATIONS_LABEL),
            Self::MenuBar => text(SETTINGS_WINDOW_PREVIEW_MENU_BAR_LABEL),
        }
    }
}

/// The rows that the column shows in place of the page: they are not drawn again inline while it
/// is there.
const IN_COLUMN: [&str; 3] = ["appearance-preview", "font-specimen", "sidebar-preview"];

impl SettingsWindow {
    /// The preview of the page in front, if it has one. A search's results are not a page: they
    /// show the window.
    pub(super) fn preview_kind(&self) -> Option<PreviewKind> {
        if self.show_results {
            return Some(PreviewKind::Window);
        }
        match self.section {
            Section::Appearance => Some(PreviewKind::Window),
            Section::Fonts => Some(PreviewKind::Fonts),
            Section::Sessions => Some(PreviewKind::Sidebar),
            Section::Agents => Some(PreviewKind::Agents),
            Section::Notifications => Some(PreviewKind::Notifications),
            Section::MenuBar => Some(PreviewKind::MenuBar),
            Section::Profile
            | Section::Terminal
            | Section::Git
            | Section::Updates
            | Section::Advanced
            | Section::Projects
            | Section::FeatureFlags => None,
        }
    }

    /// Whether the column is drawn: the window is wide enough and the page has a preview.
    pub(super) fn column_shown(&self) -> bool {
        self.preview_column && self.preview_kind().is_some()
    }

    /// Whether the column shows the preview row `id` (so the page leaves it out).
    pub(super) fn in_column(&self, id: &str) -> bool {
        self.preview_column && IN_COLUMN.contains(&id)
    }

    /// The column: its head, then the page's preview, scrolling on its own when a window is short.
    /// `None` for a page without a preview.
    pub(super) fn render_preview_column(&self, fill: Hsla, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ui = ui(cx);
        let kind = self.preview_kind()?;
        let preview = match kind {
            PreviewKind::Window => sections::draw_stage(self, cx),
            PreviewKind::Fonts => sections::draw_specimen(self, cx),
            PreviewKind::Sidebar => sections::draw_sidebar_preview(self, cx),
            PreviewKind::Agents => sections::draw_agents_preview(cx),
            PreviewKind::Notifications => sections::draw_notifications_preview(cx),
            PreviewKind::MenuBar => sections::draw_menu_bar_preview(cx),
        };
        Some(
            div()
                .id("settings-preview")
                .role(Role::Group)
                .aria_label(kind.spoken())
                .flex_none()
                .w(px(PREVIEW_W))
                .h_full()
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .px_4()
                .pt(px(PAGE_TOP))
                .pb_6()
                .gap_2()
                .bg(fill)
                .rule_left(ui)
                .child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .h(px(style::CONTROL_H))
                        .child(style::section_label(ui, text(SETTINGS_WINDOW_PREVIEW)))
                        .child(
                            div()
                                .ml_2()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(format!("· {}", kind.title())),
                        ),
                )
                .child(preview)
                .into_any_element(),
        )
    }

    /// A preview row in the search's results while the column is showing it: a note saying where it
    /// is, instead of a second copy.
    pub(super) fn render_preview_note(
        &self,
        entry: &'static settings_index::Entry,
        hit: Option<&settings_index::Hit>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        style::setting(
            SharedString::from(format!("setting-{}", entry.id)),
            ui,
            SettingParts {
                label: entry.label().into(),
                description: Some(text(SETTINGS_WINDOW_PREVIEW_COLUMN_HINT).into()),
                chip: None,
                key: "",
                state: SettingState::Default,
                found: hit.and_then(|hit| hit.label_range.clone()),
                found_description: None,
                control: div().into_any_element(),
                tall: false,
                extra: None,
                on_reset: None,
            },
            cx,
        )
        .into_any_element()
    }
}
