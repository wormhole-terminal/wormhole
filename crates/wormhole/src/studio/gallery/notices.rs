//! The Settings window's notice, of each kind, with the buttons that sit on its fill.

use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::{self, NoticeKind};

/// As wide as the Settings content column.
const COLUMN_W: f32 = 560.0;

pub(super) fn notices(ui: UiColors, cx: &App) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .max_w(px(COLUMN_W))
        .child(style::notice(
            ui,
            NoticeKind::Info,
            "Development build: updates only work in released builds.", // text: sample (the gallery's specimen)
        ))
        .child(
            style::notice(
                ui,
                NoticeKind::Warning,
                "Off in System Settings: Wormhole can’t show banners until you allow them.", // text: sample (the gallery's specimen)
            )
            .child(style::notice_button("g-notice-warning", ui, NoticeKind::Warning, cx).label("Open")), // text: sample (the gallery's specimen)
        )
        .child(
            style::notice(
                ui,
                NoticeKind::Danger,
                "Reset every setting to its default?", // text: sample (the gallery's specimen)
            )
            .child(style::notice_button_quiet("g-notice-cancel", ui, NoticeKind::Danger, cx).label("Cancel")) // text: sample (the gallery's specimen)
            .child(style::notice_button("g-notice-reset", ui, NoticeKind::Danger, cx).label("Reset")), // text: sample (the gallery's specimen)
        )
}
