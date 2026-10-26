//! The conversions between how other terminals count and how Wormhole does, and the ranges
//! Wormhole keeps its values in. A proposal carries the clamped value (`Settings::sanitized` would
//! clamp it on apply anyway); the source's own text stays in `Proposed::from`.

use crate::config::{LINE_HEIGHT, OPACITY, SCROLLBACK_LINES, Settings};
use crate::theme::fonts::TERMINAL_FONT_SIZES;

/// A finite number from text; `None` for anything else (`NaN`, `inf`, words).
pub(crate) fn number(text: &str) -> Option<f32> {
    text.trim().parse::<f32>().ok().filter(|value| value.is_finite())
}

/// A finite number from a source's own number (a TOML float, a Lua number, a JSON number, a
/// property list real), as Wormhole's `f32`; `None` for `NaN`, `inf`, and a value too big for it.
pub(crate) fn finite(value: f64) -> Option<f32> {
    let narrowed = value as f32;
    narrowed.is_finite().then_some(narrowed)
}

/// A terminal font size in points, within what Wormhole's terminal can draw. A number that is
/// not one (`NaN`, `inf`) is Wormhole's default: `clamp` would keep a `NaN`.
pub(crate) fn font_size(points: f32) -> f32 {
    if points.is_finite() {
        points.clamp(*TERMINAL_FONT_SIZES.start(), *TERMINAL_FONT_SIZES.end())
    } else {
        Settings::default().font_size
    }
}

/// A line height as a multiple of the font's own.
pub(crate) fn line_height(multiplier: f32) -> f32 {
    if multiplier.is_finite() {
        multiplier.clamp(*LINE_HEIGHT.start(), *LINE_HEIGHT.end())
    } else {
        Settings::default().line_height
    }
}

/// A background opacity from 0 (clear) to 1 (solid).
pub(crate) fn opacity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(*OPACITY.start(), *OPACITY.end())
    } else {
        Settings::default().opacity
    }
}

/// `red` as `Red`, for the sources that write the ANSI colours' names (`theme::ANSI_NAMES`)
/// capitalised.
pub(crate) fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    chars.next().map(|first| first.to_ascii_uppercase().to_string() + chars.as_str()).unwrap_or_default()
}

/// A scrollback length in lines.
pub(crate) fn scrollback(lines: usize) -> usize {
    lines.clamp(*SCROLLBACK_LINES.start(), *SCROLLBACK_LINES.end())
}

/// Scrollback with no limit: as much as Wormhole keeps.
pub(crate) fn unlimited_scrollback() -> usize {
    *SCROLLBACK_LINES.end()
}

/// The height a font's line takes as a multiple of its size, to turn an added height into a
/// multiplier. A typical monospace font's natural cell is about this tall; for another it is
/// approximate, which is why a multiplier the source gives is always preferred.
const NATURAL_LINE: f32 = 1.2;

/// How many device pixels a Mac's screen draws to a point: the Retina screens all are two. A length a
/// source gives in pixels is converted with it, which is approximate for the rest.
pub(crate) const PIXELS_PER_POINT: f32 = 2.0;

/// A line height that adds `delta` points to a font of `size` points, as the multiplier Wormhole
/// takes (`adjust-cell-height = 2`, Alacritty's `font.offset.y`, Kitty's `cell_height 2px`).
pub(crate) fn added_height_multiplier(delta: f32, size: f32) -> f32 {
    NATURAL_LINE.mul_add(size, delta) / (NATURAL_LINE * size)
}

/// Bytes of Ghostty's scrollback as lines. Ghostty counts memory, not lines (`scrollback-limit` is
/// a byte count per terminal), so there is no exact answer: a line of 80 columns takes about
/// 640 bytes (8 bytes a cell, as in this engine's grid), and that is the estimate.
pub(crate) fn lines_from_bytes(bytes: u64) -> usize {
    const BYTES_PER_LINE: u64 = 80 * 8;
    usize::try_from(bytes / BYTES_PER_LINE).unwrap_or(usize::MAX)
}
