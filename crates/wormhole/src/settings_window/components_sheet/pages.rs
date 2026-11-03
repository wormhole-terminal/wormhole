//! The component sheet's pieces that the pages draw themselves (colours, theme cards, title row parts and
//! the preview cards): the page's own functions, called with the window the sheet is drawn in, so the sheet
//! shows what a page shows and not a copy of it.

use gpui_kit::*;

use super::SPOTS;
use crate::settings_window::SettingsWindow;
use crate::settings_window::sections::{
    draw_agents_preview, draw_diff_preview, draw_menu_bar_preview, draw_notifications_preview, draw_parts,
    draw_primary, draw_sidebar_preview, draw_specimen, draw_split_preview, draw_stage, draw_theme,
};

/// What the pages draw, for the sheet to place.
pub(super) struct Pages {
    pub colours: AnyElement,
    pub themes: AnyElement,
    pub title_parts: AnyElement,
    /// Each preview card with the board's place for it (x, y).
    pub previews: Vec<(f32, f32, AnyElement)>,
}

impl Pages {
    pub(super) fn of(window: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Self {
        let _ = SPOTS;
        let previews = vec![
            (40.0, 2168.0, draw_stage(window, cx)),
            (616.0, 2168.0, draw_split_preview(window, cx)),
            (40.0, 2470.0, draw_sidebar_preview(window, cx)),
            (616.0, 2470.0, draw_agents_preview(cx)),
            (40.0, 2773.0, draw_diff_preview(window, cx)),
            (616.0, 2773.0, draw_notifications_preview(cx)),
            (40.0, 3049.0, draw_specimen(window, cx)),
            (616.0, 3049.0, draw_menu_bar_preview(cx)),
        ];
        Self {
            colours: draw_primary(window, cx),
            themes: draw_theme(window, cx),
            title_parts: draw_parts(window, cx).unwrap_or_else(|| div().into_any_element()),
            previews,
        }
    }
}
