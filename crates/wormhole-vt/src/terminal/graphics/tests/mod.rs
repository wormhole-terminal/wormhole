//! Tests of the picture protocols through the whole terminal: kitty's graphics commands (`kitty`,
//! `place`, `delete`), Sixel (`sixel`), iTerm2's inline images (`iterm`), what happens to
//! placements as the screen scrolls, is erased and resized (`screen`), and limits and hostile
//! input (`limits`). Each cites the section of the protocol document it checks.

mod delete;
mod iterm;
mod kitty;
mod limits;
mod place;
mod rare;
mod screen;
mod sixel;

use crate::terminal::base64;
use crate::terminal::tests::Rig;
use crate::terminal::{Event, ImageUpload};
use crate::types::{ImageHandle, ImagePlacement};

/// The size of a cell in the rigs: 8 by 16 pixels, the terminal's default.
pub(super) const CELL: (u16, u16) = (8, 16);

/// A rig whose terminal is `cols` by `rows`.
pub(super) fn rig(cols: usize, rows: usize) -> Rig {
    let mut rig = Rig::new(cols, rows);
    rig.term.set_cell_size(CELL.0, CELL.1);
    rig
}

/// `ESC _ G control ; payload ESC \` with the payload base64 encoded.
pub(super) fn apc(control: &str, payload: &[u8]) -> String {
    format!("\x1b_G{control};{}\x1b\\", base64::encode(payload))
}

/// A command without a payload.
pub(super) fn apc_bare(control: &str) -> String {
    format!("\x1b_G{control}\x1b\\")
}

/// `w` by `h` raw RGB pixels of one color.
pub(super) fn rgb(w: usize, h: usize) -> Vec<u8> {
    vec![0x7F; w * h * 3]
}

/// The first bytes of a PNG of this size: the signature and the IHDR chunk, which is all the
/// engine reads (the host decodes the rest).
pub(super) fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    out.extend_from_slice(&w.to_be_bytes());
    out.extend_from_slice(&h.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    out
}

/// The uploads among the events since the last call.
pub(super) fn uploads(rig: &mut Rig) -> Vec<ImageUpload> {
    rig.events()
        .into_iter()
        .filter_map(|event| match event {
            Event::ImageData(upload) => Some(upload),
            _ => None,
        })
        .collect()
}

/// The handles freed among the events since the last call.
pub(super) fn freed(rig: &mut Rig) -> Vec<ImageHandle> {
    rig.events()
        .into_iter()
        .filter_map(|event| match event {
            Event::ImageFree(handle) => Some(handle),
            _ => None,
        })
        .collect()
}

/// What is drawn, in the view.
pub(super) fn placed(rig: &Rig) -> Vec<ImagePlacement> {
    let mut out = Vec::new();
    rig.term.placements(&mut out);
    out
}

/// (picture id, line, column, cols, rows) of each placement in the view.
pub(super) fn spots(rig: &Rig) -> Vec<(u32, i32, usize, usize, usize)> {
    placed(rig).iter().map(|p| (p.image_id, p.line.0, p.column, p.cols, p.rows)).collect()
}
