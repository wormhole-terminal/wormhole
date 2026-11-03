//! The controls of the fonts page the native window writes through: each setting's menu or switch over the settings.

use super::{Current, MenuOption, Setting, toggle};

/// The line heights the menu lists, in hundredths.
const LINE_HEIGHTS: [u32; 4] = [120, 130, 140, 150];

/// "1.30", from hundredths.
fn ratio(hundredths: u32) -> String {
    format!("{:.2}", hundredths as f32 / 100.0)
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::menu_of_values(
        "line-height",
        &[
            MenuOption::counted(ratio, LINE_HEIGHTS[0]),
            MenuOption::counted(ratio, LINE_HEIGHTS[1]),
            MenuOption::counted(ratio, LINE_HEIGHTS[2]),
            MenuOption::counted(ratio, LINE_HEIGHTS[3]),
        ],
        // A line height written in config.json (or left by the slider this menu replaced) shows as it is.
        |s| match LINE_HEIGHTS.iter().position(|&h| (s.line_height * 100.0).round() as u32 == h) {
            Some(at) => Current::At(at),
            None => Current::Other(ratio((s.line_height * 100.0).round() as u32)),
        },
        |s, ix| s.line_height = LINE_HEIGHTS[ix.min(LINE_HEIGHTS.len() - 1)] as f32 / 100.0,
    ),
    toggle!("font-ligatures", font_ligatures),
];
