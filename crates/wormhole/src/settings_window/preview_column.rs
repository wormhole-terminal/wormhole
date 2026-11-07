//! The preview strip: at the top of a page that has one, the live preview that belongs to the page, always
//! [`PREVIEW_H`] tall, whatever the page and the window's size: a quiet head (the label, and the page's tabs
//! at its right), then one flat rounded stage (`style::preview_frame`, a group's fill) with the scene
//! centred on it, and under it a caption where the picture needs one. Nothing in the strip scrolls: every
//! scene is composed to fit its room whole (Appearance's window and the Terminals list are small
//! compositions of their own, not cut-off big ones), and a caption is one line that an ellipsis cuts where
//! the column is narrow. A scene draws only itself: the stage, the head and the caption are the strip's.
//!
//! - Appearance, and a search's results, show the window in miniature (`sections::draw_stage`):
//!   theme, primary, opacity, status bar and fonts as they are now.
//! - Fonts shows the specimen (`sections::draw_specimen`), on the tab of the font being picked (the tabs
//!   are in the head, `specimen_tabs`).
//! - Terminals & sidebar shows the Terminals list (`sections::draw_sidebar_preview`), beside the
//!   density under the pointer.
//! - Coding agents shows the Terminals list with agents at work in it (`draw_agents_preview`).
//! - Notifications shows two banners (`draw_notifications_preview`).
//! - Menu bar shows the menu bar item as the settings make it (`MenuBarPreview`), Hotkey the shortcut and
//!   what it opens (`HotkeyPreview`); both are views of their own (`BarPreview`).
//! - Motion shows every option's scene at once, four cells looping (`MotionStrip`).
//! - Changes & diffs shows the Changes panel's list, a diff or a binary file's diff (`draw_git_preview`),
//!   on the scene of the setting last changed, or the tab picked.
//! - Every other page has nothing to show there.
//!
//! The strip holds nothing to operate: no focus stop, so ⇥ and F6 pass over it, and one labelled
//! group for VoiceOver. A search result for a preview names where it went instead of drawing it.

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::config::settings_index::{self, SectionId as Section};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::{SettingsWindow, sections, ui};
use crate::style::{self, RowKeys, SettingParts, SettingState, TextRole};

/// Which preview the strip shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PreviewKind {
    /// The window in miniature on a plain backdrop.
    Window,
    /// The font specimen.
    Fonts,
    /// The Terminals list in the sidebar's own look.
    Sidebar,
    /// The Terminals list with coding agents at work.
    Agents,
    /// Notification banners.
    Notifications,
    /// The menu bar item in each of its states.
    MenuBar,
    /// The hotkey's keys and what they open.
    Hotkey,
    /// The Motion page's four scenes side by side.
    Motion,
    /// The Changes panel and the diff view.
    Git,
}

impl PreviewKind {
    /// What the strip's head says.
    fn title(self) -> &'static str {
        match self {
            Self::Window => text(SETTINGS_WINDOW_WINDOW),
            Self::Fonts => text(SETTINGS_WINDOW_FONTS),
            Self::Sidebar | Self::Agents => text(SETTINGS_WINDOW_TERMINALS_LIST),
            Self::Notifications => text(SETTINGS_WINDOW_NOTIFICATIONS),
            Self::MenuBar => text(SETTINGS_WINDOW_MENU_BAR),
            Self::Hotkey => text(SETTINGS_WINDOW_HOTKEY),
            Self::Git => text(SETTINGS_SECTION_GIT),
            Self::Motion => text(SETTINGS_SECTION_MOTION),
        }
    }

    /// A line at the head's right end: what to know of how the picture moves.
    fn aside(self) -> Option<&'static str> {
        match self {
            Self::Motion => Some(text(SETTINGS_MOTION_PREVIEW_CAPTION)),
            _ => None,
        }
    }

    /// The line under the stage, where the picture needs a word of how it differs from the app.
    fn caption(self) -> Option<&'static str> {
        match self {
            Self::Window => Some(text(SETTINGS_APPEARANCE_STAGE_CAPTION)),
            Self::Fonts => Some(text(SETTINGS_FONTS_HOVER_A_FONT_IN_A_LIST)),
            Self::Sidebar => Some(text(SETTINGS_SESSIONS_PREVIEW_CAPTION_SIDE_BY_SIDE)),
            Self::Agents => Some(text(SETTINGS_AGENTS_PREVIEW_CAPTION)),
            Self::Notifications => Some(text(SETTINGS_NOTIFICATIONS_PREVIEW_CAPTION)),
            _ => None,
        }
    }

    /// Whether the scene is made to stand in the stage's whole height and width (the window in miniature,
    /// the Changes panel's list, the motions side by side), so the stage gives it the least room round it
    /// ([`STAGE_INSET`]).
    fn fills_room(self) -> bool {
        matches!(self, Self::Window | Self::Git | Self::Motion)
    }

    /// What VoiceOver says for the strip: that it is a picture of the settings, not controls.
    fn spoken(self) -> &'static str {
        match self {
            Self::Window => text(SETTINGS_WINDOW_PREVIEW_WINDOW_LABEL),
            Self::Fonts => text(SETTINGS_WINDOW_PREVIEW_FONTS_LABEL),
            Self::Sidebar => text(SETTINGS_WINDOW_PREVIEW_TERMINALS_LABEL),
            Self::Agents => text(SETTINGS_WINDOW_PREVIEW_AGENTS_LABEL),
            Self::Notifications => text(SETTINGS_WINDOW_PREVIEW_NOTIFICATIONS_LABEL),
            Self::MenuBar => text(SETTINGS_WINDOW_PREVIEW_MENU_BAR_LABEL),
            Self::Hotkey => text(SETTINGS_WINDOW_PREVIEW_HOTKEY_LABEL),
            Self::Motion => text(SETTINGS_WINDOW_PREVIEW_MOTION_LABEL),
            Self::Git => text(SETTINGS_WINDOW_PREVIEW_GIT_LABEL),
        }
    }
}

/// The strip's height, head and scene together, on every page and at every window size (4-pt grid). Its
/// number is the smallest that holds the largest scene whole: Appearance's window (a 176 pt window with its
/// sidebar, title row, terminal and status bar, framed, with its caption) and the Changes panel's list
/// (about 217 pt) set it; the others are smaller. The page scrolls, so it stays reachable under it at
/// `WINDOW_MIN`'s 420 pt.
pub(super) const PREVIEW_H: f32 = 256.0;
/// The gap under the head (`gap_2`).
const HEAD_GAP: f32 = 8.0;
/// The room the scene has: the strip less its head and the gap under it.
pub(super) const SCENE_H: f32 = PREVIEW_H - style::CONTROL_H - HEAD_GAP;
/// What a caption under a scene takes: the gap (`gap_2`) and its one line (`text_caption`, 20 pt).
pub(super) const CAPTION_ROW: f32 = 28.0;
/// A scene's frame when a caption goes under it: the room less the caption.
pub(super) const FRAMED_H: f32 = SCENE_H - CAPTION_ROW;
/// What a stage that scenes fill (`p_2`: `fills_room`) leaves round them, at each side.
pub(super) const STAGE_INSET: f32 = 8.0;
/// The height inside such a stage, which a scene that fills it has to lay itself out in.
pub(super) const FILLED_INNER_H: f32 = SCENE_H - 2.0 * STAGE_INSET;

/// The rows the strip shows in place of the page: they are not drawn again in it.
const IN_COLUMN: [&str; 4] = ["appearance-preview", "font-specimen", "sidebar-preview", "changes-preview"];

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
            Section::Hotkey => Some(PreviewKind::Hotkey),
            Section::Motion => self.motion_strip.as_ref().map(|_| PreviewKind::Motion),
            Section::Git => Some(PreviewKind::Git),
            Section::Profile
            | Section::Changed
            | Section::Terminal
            | Section::Import
            | Section::Updates
            | Section::Advanced
            | Section::Projects
            | Section::Extensions
            | Section::FeatureFlags
            | Section::Components => None,
        }
    }

    /// Whether the strip shows the preview row `id` (so the page leaves it out).
    pub(super) fn in_column(&self, id: &str) -> bool {
        IN_COLUMN.contains(&id)
    }

    /// The strip: its head, then the page's preview on one flat stage, [`PREVIEW_H`] tall in all, with
    /// nothing that scrolls. `None` for a page without a preview.
    pub(super) fn render_preview_strip(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let ui = ui(cx);
        let kind = self.preview_kind()?;
        let preview = match kind {
            PreviewKind::Window => sections::draw_stage(self, cx),
            PreviewKind::Fonts => sections::draw_specimen(self, cx),
            PreviewKind::Sidebar => sections::draw_sidebar_preview(self, cx),
            PreviewKind::Agents => sections::draw_agents_preview(cx),
            PreviewKind::Notifications => sections::draw_notifications_preview(cx),
            PreviewKind::MenuBar | PreviewKind::Hotkey => self
                .bar_preview
                .as_ref()
                .map_or_else(|| div().into_any_element(), sections::BarPreview::element),
            PreviewKind::Motion => self.motion_strip.as_ref().map(|strip| strip.view())?,
            PreviewKind::Git => sections::draw_git_preview(self, cx),
        };
        // A preview with scenes puts their tabs at the head's right.
        let tabs = match kind {
            PreviewKind::Git => Some(sections::git_preview_tabs(self, cx)),
            PreviewKind::Fonts => Some(sections::specimen_tabs(self, cx)),
            _ => None,
        };
        // The stage is the same card on every page: a group's fill, `radius_lg`, no outline. A caption goes
        // under it, so the stage gives up the room the caption takes.
        let caption = kind.caption();
        let stage = style::preview_frame(ui)
            .flex_none()
            .w_full()
            .h(px(if caption.is_some() { FRAMED_H } else { SCENE_H }))
            .flex()
            .flex_col()
            .justify_center()
            .when_else(kind.fills_room(), |stage| stage.p_2(), |stage| stage.px_4().py_3())
            .child(preview);
        Some(
            div()
                .id("settings-preview")
                .role(Role::Group)
                .aria_label(kind.spoken())
                .flex_none()
                .w_full()
                .h(px(PREVIEW_H))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    // The head, as a group's: a quiet label from the card's inner edge, the page's tabs
                    // or a note at the other.
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .px_4()
                        .h(px(style::CONTROL_H))
                        .child(
                            div()
                                .flex_none()
                                .text_label()
                                .text_color(ui.muted)
                                .child(text(SETTINGS_WINDOW_PREVIEW)),
                        )
                        // What the picture is of, where the head has room for it: the page's tabs take it, and
                        // they name the scene themselves.
                        .when(tabs.is_none(), |head| {
                            head.child(
                                div()
                                    .ml_2()
                                    .min_w_0()
                                    .truncate()
                                    .text_label()
                                    .text_color(ui.faint)
                                    .child(kind.title()),
                            )
                        })
                        .children(kind.aside().map(|aside| {
                            div()
                                .flex_1()
                                .min_w_0()
                                .ml_4()
                                .truncate()
                                .text_right()
                                .text_caption()
                                .text_color(ui.faint)
                                .child(aside)
                        }))
                        .children(tabs.map(|tabs| div().flex_none().ml_auto().pl_4().child(tabs))),
                )
                // The room under the head, the same on every page. It is clipped, never scrolled: a scene is
                // made to fit (a font size far above the default can still outgrow the specimen's block,
                // which then cuts the text).
                .child(stage)
                .children(caption.map(|caption| {
                    div().flex_none().px_4().text_caption().text_color(ui.muted).truncate().child(caption)
                }))
                .into_any_element(),
        )
    }

    /// A preview row in the search's results: a note saying where it is, instead of a second copy.
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
                visible_hint: false,
                chip: None,
                keys: RowKeys::NONE,
                state: SettingState::Default,
                found: hit.and_then(|hit| hit.label_range.clone()),
                found_description: None,
                control: div().into_any_element(),
                tall: false,
                extra: None,
                on_reset: None,
                reset_arrow: crate::style::ResetArrow::OnHover,
            },
            cx,
        )
        .into_any_element()
    }
}
