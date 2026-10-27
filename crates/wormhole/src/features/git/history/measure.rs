//! How wide text is in the panel's own font, for the two questions History asks of it: whether a subject
//! is cut by its row, and how many characters a line of the message holds.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// The type roles the tab draws with, in rems: body (a commit's subject) and caption (the details).
pub(super) const BODY_REMS: f32 = 0.875;
pub(super) const CAPTION_REMS: f32 = 0.75;

/// The width of `text` in the interface font at `size_rems`. The text system keeps what it laid out,
/// so asking again for the same text costs a lookup.
pub(super) fn text_width(text: &str, size_rems: f32, weight: FontWeight, window: &Window, cx: &App) -> f32 {
    let family = cx.theme().font_family.clone();
    let run = TextRun { len: text.len(), font: Font { weight, ..font(family) }, ..Default::default() };
    let size = rems(size_rems).to_pixels(window.rem_size());
    f32::from(window.text_system().layout_line(text, size, &[run], None).width)
}

/// About how many characters of caption text fit in `room` points: the width of a line of ordinary
/// words, divided out. A little under it, so that what is cut or offered as "more" errs on the side of
/// offering.
pub(super) fn caption_chars(room: f32, window: &Window, cx: &App) -> usize {
    const SAMPLE: &str = "The quick brown fox jumps over";
    let per_char = text_width(SAMPLE, CAPTION_REMS, FontWeight::NORMAL, window, cx) / SAMPLE.len() as f32;
    ((room / per_char.max(1.0)) * 0.95).max(16.0) as usize
}
