//! The conversions between how other terminals count and how Wormhole does, and the ranges
//! Wormhole keeps its values in. A proposal carries the clamped value (`Settings::sanitized` would
//! clamp it on apply anyway); the source's own text stays in `Proposed::from`.

use crate::config::{LINE_HEIGHT, OPACITY, SCROLLBACK_LINES};
use crate::theme::fonts::TERMINAL_FONT_SIZES;

/// A finite number from text; `None` for anything else (`NaN`, `inf`, words).
pub(crate) fn number(text: &str) -> Option<f32> {
    text.trim().parse::<f32>().ok().filter(|value| value.is_finite())
}

/// A terminal font size in points, within what Wormhole's terminal can draw.
pub(crate) fn font_size(points: f32) -> f32 {
    points.clamp(*TERMINAL_FONT_SIZES.start(), *TERMINAL_FONT_SIZES.end())
}

/// A line height as a multiple of the font's own.
pub(crate) fn line_height(multiplier: f32) -> f32 {
    multiplier.clamp(*LINE_HEIGHT.start(), *LINE_HEIGHT.end())
}

/// A background opacity from 0 (clear) to 1 (solid).
pub(crate) fn opacity(value: f32) -> f32 {
    value.clamp(*OPACITY.start(), *OPACITY.end())
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

/// Words a config file uses for on and off.
pub(crate) fn truthy(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Where `text` splits into the words a command line is made of: whitespace, with `'…'`, `"…"` and
/// `\` quoting. Nothing is expanded and nothing runs.
pub(crate) fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => match chars.next() {
                Some(next @ ('"' | '\\')) => current.push(next),
                Some(next) => {
                    current.push('\\');
                    current.push(next);
                }
                None => current.push('\\'),
            },
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    started = true;
                }
            }
            (None, c) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (None, c) => {
                current.push(c);
                started = true;
            }
        }
    }
    if started || !current.is_empty() {
        out.push(current);
    }
    out
}
