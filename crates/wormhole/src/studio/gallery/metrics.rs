//! The design language's numbers: sizes, insets, limits and motion, each with its
//! value, so a constant that is not a component still shows in the Studio.

use std::time::Duration;

use gpui_kit::*;
use style::motion::{self, CubicBezier};

use super::Env;
use super::view::GalleryView;
use crate::settings::UiColors;
use crate::style::{self, TextRole};

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    let pt = |v: f32| format!("{v}");
    let share = |v: f32| format!("{:.0} %", v * 100.0);
    let rem = |v: f32| format!("{v} rem");
    let sizes: Vec<(&str, String)> = vec![
        ("CONTROL_H · CONTROL_H_SM", format!("{} · {}", style::CONTROL_H, style::CONTROL_H_SM)),
        ("ROW_H · ROW_TWO_LINE_H", format!("{} · {}", style::ROW_H, style::ROW_TWO_LINE_H)),
        ("NAME_LINE · META_LINE", format!("{} · {}", rem(style::NAME_LINE), rem(style::META_LINE))),
        ("BAND_H · FOLDER_BAND_H", format!("{} · {}", style::BAND_H, style::FOLDER_BAND_H)),
        ("TITLE_ROW_H", pt(style::TITLE_ROW_H)),
        ("STATUS_BAR_H · TITLE_TAB_MAX_W", format!("{} · {}", style::STATUS_BAR_H, style::TITLE_TAB_MAX_W)),
        ("BADGE_H", pt(style::BADGE_H)),
        ("KEYCAP_H · KEYCAP_MIN_W", format!("{} · {}", style::KEYCAP_H, style::KEYCAP_MIN_W)),
        ("SETTING_ROW_MIN_H", pt(style::SETTING_ROW_MIN_H)),
        ("BANNER_MIN_H", pt(style::BANNER_MIN_H)),
        ("SIDEBAR_INSET · PANEL_INSET", format!("{} · {}", style::SIDEBAR_INSET, style::PANEL_INSET)),
        ("ITEM_LEAD_W · INDENT", format!("{} · {}", style::ITEM_LEAD_W, style::INDENT)),
        (
            "GLYPH · TITLE_GLYPH · EMPTY_GLYPH",
            format!("{} · {} · {}", style::GLYPH, style::TITLE_GLYPH, style::EMPTY_GLYPH),
        ),
        ("HIT_MIN", pt(style::HIT_MIN)),
        ("STATUS_DOT_SIZE · STATUS_HALO", format!("{} · {}", style::STATUS_DOT_SIZE, style::STATUS_HALO)),
        ("MARK_DOT", pt(style::MARK_DOT)),
        ("FOCUS_RING_OFFSET", pt(style::FOCUS_RING_OFFSET)),
        ("DRAG_LIFT_OPACITY", share(style::DRAG_LIFT_OPACITY)),
        ("DROP_LINE_W · RESIZE_HANDLE_W", format!("{} · {}", style::DROP_LINE_W, style::RESIZE_HANDLE_W)),
        ("BRANCH_CHIP_MAX · _MIN", format!("{} · {}", style::BRANCH_CHIP_MAX, style::BRANCH_CHIP_MIN)),
        (
            "FILE_NAME_MAX · FOLDER_NAME_MAX",
            format!("{} · {}", share(style::FILE_NAME_MAX), share(style::FOLDER_NAME_MAX)),
        ),
        ("FILTER_POPOVER_W", pt(style::FILTER_POPOVER_W)),
        ("SLIDER_W · compact", format!("{} · {}", style::SLIDER_W, style::SLIDER_W_COMPACT)),
        ("SWATCH · RECORDER_W", format!("{} · {}", style::SWATCH, style::RECORDER_W)),
        ("THEME_CARD", format!("{} × {}", style::THEME_CARD_W, style::THEME_CARD_H)),
        (
            "QUICK_PANEL_W · QP_SEARCH_H · QP_ROW_H",
            format!("{} · {} · {}", style::QUICK_PANEL_W, style::QP_SEARCH_H, style::QP_ROW_H),
        ),
        ("QP_TOP", share(style::QP_TOP)),
    ];
    let limits: Vec<(&str, String)> = vec![
        (
            "COUNT_BADGE_MAX",
            format!("{} → {}", style::COUNT_BADGE_MAX, style::count_label(style::COUNT_BADGE_MAX + 1)),
        ),
        ("FOLD_ABOVE · DONE_FOLD_ABOVE", format!("{} · {}", style::FOLD_ABOVE, style::DONE_FOLD_ABOVE)),
        ("NOTE_OPEN_MAX_LINES", style::NOTE_OPEN_MAX_LINES.to_string()),
        ("DIFF_SPLIT_MIN", pt(style::DIFF_SPLIT_MIN)),
        (
            "STATUS_BAR_WORDS_MIN · _SIZE_MIN",
            format!("{} · {}", style::STATUS_BAR_WORDS_MIN, style::STATUS_BAR_SIZE_MIN),
        ),
    ];
    let ms = |d: Duration| format!("{} ms", d.as_millis());
    let curve = |c: CubicBezier| format!("cubic-bezier({}, {}, {}, {})", c.0, c.1, c.2, c.3);
    let motion: Vec<(&str, String)> = vec![
        ("MOTION_INSTANT", ms(motion::MOTION_INSTANT)),
        ("MOTION_EXIT_FAST", ms(motion::MOTION_EXIT_FAST)),
        ("MOTION_FAST", ms(motion::MOTION_FAST)),
        ("MOTION_POPOVER", ms(motion::MOTION_POPOVER)),
        ("MOTION_BASE", ms(motion::MOTION_BASE)),
        ("MOTION_PANEL_EXIT", ms(motion::MOTION_PANEL_EXIT)),
        ("MOTION_PANEL", ms(motion::MOTION_PANEL)),
        ("EASE_OUT", curve(motion::EASE_OUT)),
        ("EASE_IN", curve(motion::EASE_IN)),
        ("EASE_IN_OUT", curve(motion::EASE_IN_OUT)),
        ("LINEAR", curve(motion::LINEAR)),
        (
            "SPRING SNAP · SOFT · BOUNCY",
            [motion::spring::SNAP, motion::spring::SOFT, motion::spring::BOUNCY]
                .map(|s| format!("{}/{}", s.stiffness, s.damping))
                .join(" · "),
        ),
        ("SPIN_PERIOD", ms(motion::SPIN_PERIOD)),
        (
            "STATUS_PULSE",
            format!("{} · {}–{}", ms(motion::STATUS_PULSE), motion::PULSE_RANGE.0, motion::PULSE_RANGE.1),
        ),
        ("TILE_ORBIT · TILE_BREATHE", format!("{} · {}", ms(motion::TILE_ORBIT), ms(motion::TILE_BREATHE))),
        ("ANIMATION_FPS", motion::ANIMATION_FPS.to_string()),
        ("TOOLTIP_DELAY · HINT_HOLD", format!("{} · {}", ms(motion::TOOLTIP_DELAY), ms(motion::HINT_HOLD))),
        ("RESIZE_HANDLE_HOVER_DELAY", ms(motion::RESIZE_HANDLE_HOVER_DELAY)),
    ];
    // The curves drawn: how far each has come at a few points in time.
    let ease_samples = [motion::EASE_OUT, motion::EASE_IN, motion::EASE_IN_OUT, motion::LINEAR].map(|c| {
        let ease = c.easing();
        [0.25, 0.5, 0.75].map(|t| format!("{:.2}", ease(t))).join(" ")
    });
    // The page builds one piece at a time (a piece is a stretch of the page).
    match part {
        0 => table(ui, "Sizes", sizes).into_any_element(),
        _ => div()
            .flex()
            .flex_col()
            .gap_4()
            .child(table(ui, "Limits", limits))
            .child(table(ui, "Motion", motion))
            .child(div().text_caption().text_color(ui.muted).child(format!(
                "at ¼ ½ ¾: out {} · in {} · in-out {} · linear {}",
                ease_samples[0], ease_samples[1], ease_samples[2], ease_samples[3]
            )))
            .into_any_element(),
    }
}

fn table(ui: UiColors, title: &'static str, rows: Vec<(&'static str, String)>) -> Div {
    div().flex().flex_col().gap_1().child(style::section_label(ui, title)).children(rows.into_iter().map(
        |(name, value)| {
            div()
                .flex()
                .gap_4()
                .text_caption()
                .child(div().w(px(NAME_W)).text_color(ui.muted).child(name))
                .child(div().tabular_nums().text_color(ui.text).child(value))
        },
    ))
}

/// The name column of a table, wide enough for the longest constant name.
const NAME_W: f32 = 240.0;
