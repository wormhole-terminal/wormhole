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
    let rem = |v: f32| format!("{v} rem"); // text: sample (the gallery's specimen)
    let sizes: Vec<(&str, String)> = vec![
        ("CONTROL_H · CONTROL_H_SM", format!("{} · {}", style::CONTROL_H, style::CONTROL_H_SM)), // text: sample (the gallery's specimen)
        ("ROW_H · ROW_TWO_LINE_H", format!("{} · {}", style::ROW_H, style::ROW_TWO_LINE_H)), // text: sample (the gallery's specimen)
        ("NAME_LINE · META_LINE", format!("{} · {}", rem(style::NAME_LINE), rem(style::META_LINE))), // text: sample (the gallery's specimen)
        ("BAND_H · FOLDER_BAND_H", format!("{} · {}", style::BAND_H, style::FOLDER_BAND_H)), // text: sample (the gallery's specimen)
        ("TITLE_ROW_H", pt(style::TITLE_ROW_H)), // text: sample (the gallery's specimen)
        ("STATUS_BAR_H · TITLE_TAB_MAX_W", format!("{} · {}", style::STATUS_BAR_H, style::TITLE_TAB_MAX_W)), // text: sample (the gallery's specimen)
        ("BADGE_H", pt(style::BADGE_H)), // text: sample (the gallery's specimen)
        ("KEYCAP_H · KEYCAP_MIN_W", format!("{} · {}", style::KEYCAP_H, style::KEYCAP_MIN_W)), // text: sample (the gallery's specimen)
        ("SETTING_ROW_MIN_H", pt(style::SETTING_ROW_MIN_H)), // text: sample (the gallery's specimen)
        ("BANNER_MIN_H", pt(style::BANNER_MIN_H)),           // text: sample (the gallery's specimen)
        ("SIDEBAR_INSET · PANEL_INSET", format!("{} · {}", style::SIDEBAR_INSET, style::PANEL_INSET)), // text: sample (the gallery's specimen)
        ("ITEM_LEAD_W · INDENT", format!("{} · {}", style::ITEM_LEAD_W, style::INDENT)), // text: sample (the gallery's specimen)
        (
            "GLYPH · TITLE_GLYPH · EMPTY_GLYPH", // text: sample (the gallery's specimen)
            format!("{} · {} · {}", style::GLYPH, style::TITLE_GLYPH, style::EMPTY_GLYPH),
        ),
        ("HIT_MIN", pt(style::HIT_MIN)), // text: sample (the gallery's specimen)
        ("STATUS_DOT_SIZE · STATUS_HALO", format!("{} · {}", style::STATUS_DOT_SIZE, style::STATUS_HALO)), // text: sample (the gallery's specimen)
        ("MARK_DOT", pt(style::MARK_DOT)), // text: sample (the gallery's specimen)
        ("FOCUS_RING_OFFSET", pt(style::FOCUS_RING_OFFSET)), // text: sample (the gallery's specimen)
        ("DRAG_LIFT_OPACITY", share(style::DRAG_LIFT_OPACITY)), // text: sample (the gallery's specimen)
        ("DROP_LINE_W · RESIZE_HANDLE_W", format!("{} · {}", style::DROP_LINE_W, style::RESIZE_HANDLE_W)), // text: sample (the gallery's specimen)
        ("BRANCH_CHIP_MAX · _MIN", format!("{} · {}", style::BRANCH_CHIP_MAX, style::BRANCH_CHIP_MIN)), // text: sample (the gallery's specimen)
        (
            "FILE_NAME_MAX · FOLDER_NAME_MAX", // text: sample (the gallery's specimen)
            format!("{} · {}", share(style::FILE_NAME_MAX), share(style::FOLDER_NAME_MAX)),
        ),
        ("FILTER_POPOVER_W", pt(style::FILTER_POPOVER_W)), // text: sample (the gallery's specimen)
        ("SLIDER_W · compact", format!("{} · {}", style::SLIDER_W, style::SLIDER_W_COMPACT)), // text: sample (the gallery's specimen)
        (
            "SWATCH · RECORDER_W · SELECT_W", // text: sample (the gallery's specimen)
            format!("{} · {} · {}", style::SWATCH, style::RECORDER_W, style::SELECT_W),
        ),
        ("THEME_CARD", format!("{} × {}", style::THEME_CARD_W, style::THEME_CARD_H)), // text: sample (the gallery's specimen)
        (
            "QUICK_PANEL_W · QP_SEARCH_H · QP_ROW_H", // text: sample (the gallery's specimen)
            format!("{} · {} · {}", style::QUICK_PANEL_W, style::QP_SEARCH_H, style::QP_ROW_H),
        ),
        ("QP_TOP", share(style::QP_TOP)), // text: sample (the gallery's specimen)
    ];
    let limits: Vec<(&str, String)> = vec![
        (
            "COUNT_BADGE_MAX", // text: sample (the gallery's specimen)
            format!("{} → {}", style::COUNT_BADGE_MAX, style::count_label(style::COUNT_BADGE_MAX + 1)),
        ),
        ("FOLD_ABOVE · DONE_FOLD_ABOVE", format!("{} · {}", style::FOLD_ABOVE, style::DONE_FOLD_ABOVE)), // text: sample (the gallery's specimen)
        ("NOTE_OPEN_MAX_LINES", style::NOTE_OPEN_MAX_LINES.to_string()), // text: sample (the gallery's specimen)
        ("DIFF_SPLIT_MIN", pt(style::DIFF_SPLIT_MIN)), // text: sample (the gallery's specimen)
        (
            "STATUS_BAR_WORDS_MIN · _SIZE_MIN", // text: sample (the gallery's specimen)
            format!("{} · {}", style::STATUS_BAR_WORDS_MIN, style::STATUS_BAR_SIZE_MIN),
        ),
    ];
    let ms = |d: Duration| format!("{} ms", d.as_millis()); // text: sample (the gallery's specimen)
    let curve = |c: CubicBezier| format!("cubic-bezier({}, {}, {}, {})", c.0, c.1, c.2, c.3); // text: sample (the gallery's specimen)
    let motion: Vec<(&str, String)> = vec![
        ("MOTION_INSTANT", ms(motion::MOTION_INSTANT)), // text: sample (the gallery's specimen)
        ("MOTION_EXIT_FAST", ms(motion::MOTION_EXIT_FAST)), // text: sample (the gallery's specimen)
        ("MOTION_FAST", ms(motion::MOTION_FAST)),       // text: sample (the gallery's specimen)
        ("MOTION_POPOVER", ms(motion::MOTION_POPOVER)), // text: sample (the gallery's specimen)
        ("MOTION_BASE", ms(motion::MOTION_BASE)),       // text: sample (the gallery's specimen)
        ("MOTION_PANEL_EXIT", ms(motion::MOTION_PANEL_EXIT)), // text: sample (the gallery's specimen)
        ("MOTION_PANEL", ms(motion::MOTION_PANEL)),     // text: sample (the gallery's specimen)
        ("EASE_OUT", curve(motion::EASE_OUT)),          // text: sample (the gallery's specimen)
        ("EASE_IN", curve(motion::EASE_IN)),            // text: sample (the gallery's specimen)
        ("EASE_IN_OUT", curve(motion::EASE_IN_OUT)),    // text: sample (the gallery's specimen)
        ("LINEAR", curve(motion::LINEAR)),              // text: sample (the gallery's specimen)
        (
            "SPRING SNAP · SOFT · BOUNCY", // text: sample (the gallery's specimen)
            [motion::SPRING_SNAP, motion::SPRING_SOFT, motion::SPRING_BOUNCY]
                .map(|s| format!("{}/{}", s.stiffness, s.damping))
                .join(" · "),
        ),
        ("SPIN_PERIOD", ms(motion::SPIN_PERIOD)), // text: sample (the gallery's specimen)
        (
            "STATUS_PULSE", // text: sample (the gallery's specimen)
            format!("{} · {}–{}", ms(motion::STATUS_PULSE), motion::PULSE_RANGE.0, motion::PULSE_RANGE.1),
        ),
        ("TILE_ORBIT · TILE_BREATHE", format!("{} · {}", ms(motion::TILE_ORBIT), ms(motion::TILE_BREATHE))), // text: sample (the gallery's specimen)
        ("ANIMATION_FPS", motion::ANIMATION_FPS.to_string()), // text: sample (the gallery's specimen)
        ("TOOLTIP_DELAY · HINT_HOLD", format!("{} · {}", ms(motion::TOOLTIP_DELAY), ms(motion::HINT_HOLD))), // text: sample (the gallery's specimen)
        ("RESIZE_HANDLE_HOVER_DELAY", ms(motion::RESIZE_HANDLE_HOVER_DELAY)), // text: sample (the gallery's specimen)
    ];
    // The curves drawn: how far each has come at a few points in time.
    let ease_samples = [motion::EASE_OUT, motion::EASE_IN, motion::EASE_IN_OUT, motion::LINEAR].map(|c| {
        let ease = c.easing();
        [0.25, 0.5, 0.75].map(|t| format!("{:.2}", ease(t))).join(" ")
    });
    // The page builds one piece at a time (a piece is a stretch of the page).
    match part {
        0 => table(ui, "Sizes", sizes).into_any_element(), // text: sample (the gallery's specimen)
        _ => div()
            .flex()
            .flex_col()
            .gap_4()
            .child(table(ui, "Limits", limits)) // text: sample (the gallery's specimen)
            .child(table(ui, "Motion", motion)) // text: sample (the gallery's specimen)
            .child(div().text_caption().text_color(ui.muted).child(format!(
                "at ¼ ½ ¾: out {} · in {} · in-out {} · linear {}", // text: sample (the gallery's specimen)
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
