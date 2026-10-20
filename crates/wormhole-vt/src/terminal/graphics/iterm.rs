//! iTerm2's inline images: `OSC 1337 ; File = key=value;… : <base64> ST`, and its multipart form
//! (`MultipartFile=`, then `FilePart=` chunks, then `FileEnd`).
//!
//! Spec: iTerm2's "Inline Images Protocol" page. The arguments: `inline=1` shows the file where
//! it is (without it the file is a download, which the terminal does not do), `width` and
//! `height` are `N` (cells), `Npx` (pixels), `N%` (of the terminal) or `auto` (the picture's own),
//! `preserveAspectRatio` (default 1) keeps the picture's shape inside the box the sizes make, and
//! `doNotMoveCursor=1` leaves the cursor. `name`, `size` and the rest are not needed to show it.
//! The file's bytes are decoded by the host (any format its decoder reads); the engine reads the
//! size from the header to know the cells.

use crate::terminal::{Event, Terminal};
use crate::types::{Extent, SourceRect};

use super::apc::PAYLOAD_MAX;
use super::geometry::Geometry;
use super::put::{Motion, Placing};
use super::sniff;
use super::transmit::check_size;
use super::upload::{Compression, ImageFormat, ImageUpload};

/// Most base64 text one image may take: the payload cap's encoding.
const BASE64_MAX: usize = PAYLOAD_MAX / 3 * 4 + 4;

/// How wide or tall the picture is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Dimension {
    /// The picture's own size.
    Auto,
    Cells(u32),
    Pixels(u32),
    /// Of the terminal's width or height, 0 to 100.
    Percent(u32),
}

impl Dimension {
    /// `auto`, `N`, `Npx` or `N%`; anything else is `auto`.
    fn parse(value: &str) -> Dimension {
        let value = value.trim();
        let number = |digits: &str| digits.parse::<u32>().ok();
        if let Some(px) = value.strip_suffix("px") {
            number(px).map_or(Dimension::Auto, Dimension::Pixels)
        } else if let Some(percent) = value.strip_suffix('%') {
            number(percent).map_or(Dimension::Auto, |p| Dimension::Percent(p.min(100)))
        } else {
            number(value).map_or(Dimension::Auto, Dimension::Cells)
        }
    }

    /// The size in pixels, `None` for auto. `cell` is a cell's size along this axis and `extent`
    /// the terminal's size in cells.
    fn pixels(self, cell: u64, extent: usize) -> Option<u64> {
        match self {
            Dimension::Auto => None,
            Dimension::Cells(n) => Some(u64::from(n) * cell),
            Dimension::Pixels(n) => Some(u64::from(n)),
            Dimension::Percent(p) => Some(extent as u64 * cell * u64::from(p) / 100),
        }
        .map(|px| px.max(1))
    }
}

/// `File=`'s arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileArgs {
    pub width: Dimension,
    pub height: Dimension,
    pub preserve_aspect_ratio: bool,
    /// Shown in the terminal (otherwise iTerm2 downloads the file: nothing to show).
    pub inline: bool,
    pub do_not_move_cursor: bool,
}

impl Default for FileArgs {
    fn default() -> Self {
        FileArgs {
            width: Dimension::Auto,
            height: Dimension::Auto,
            preserve_aspect_ratio: true,
            inline: false,
            do_not_move_cursor: false,
        }
    }
}

impl FileArgs {
    /// The `key=value;…` before the colon.
    pub fn parse(args: &str) -> FileArgs {
        let mut out = FileArgs::default();
        for pair in args.split(';') {
            let Some((key, value)) = pair.split_once('=') else { continue };
            let value = value.trim();
            match key.trim() {
                "width" => out.width = Dimension::parse(value),
                "height" => out.height = Dimension::parse(value),
                "preserveAspectRatio" => out.preserve_aspect_ratio = value != "0",
                "inline" => out.inline = value == "1",
                "doNotMoveCursor" => out.do_not_move_cursor = value == "1",
                _ => {}
            }
        }
        out
    }

    /// How many cells the picture covers and how it is sized in them. `cell` is a cell's size in
    /// pixels, `screen` the terminal's columns and rows. A picture is never wider than the
    /// terminal: iTerm2 scales one down that would be.
    pub fn geometry(
        &self,
        (width, height): (u32, u32),
        cell: (u32, u32),
        screen: (usize, usize),
    ) -> Geometry {
        let (cell_w, cell_h) = (u64::from(cell.0.max(1)), u64::from(cell.1.max(1)));
        let (iw, ih) = (u64::from(width.max(1)), u64::from(height.max(1)));
        let screen_w = screen.0 as u64 * cell_w;
        let wanted = (self.width.pixels(cell_w, screen.0), self.height.pixels(cell_h, screen.1));
        let keep = self.preserve_aspect_ratio;
        let (w, h, extent) = match wanted {
            (None, None) => (iw, ih, None),
            (Some(w), None) if keep => (w, mul_div(ih, w, iw), None),
            (Some(w), None) => (w, ih, Some(Extent::Fill)),
            (None, Some(h)) if keep => (mul_div(iw, h, ih), h, None),
            (None, Some(h)) => (iw, h, Some(Extent::Fill)),
            (Some(w), Some(h)) => (w, h, Some(if keep { Extent::Fit } else { Extent::Fill })),
        };
        // Too wide for the terminal: scaled down with its shape.
        let (w, h) = if w > screen_w && screen_w > 0 { (screen_w, mul_div(h, screen_w, w)) } else { (w, h) };
        let extent = extent.unwrap_or(Extent::Pixels {
            width: w.min(u64::from(u32::MAX)) as u32,
            height: h.min(u64::from(u32::MAX)) as u32,
        });
        Geometry {
            cols: w.div_ceil(cell_w).max(1) as usize,
            rows: h.div_ceil(cell_h).max(1) as usize,
            source: SourceRect { x: 0, y: 0, width, height },
            extent,
            offset_x: 0,
            offset_y: 0,
        }
        .limited()
    }
}

/// A multipart image under way: its arguments and its base64 so far.
#[derive(Debug)]
pub(super) struct Multipart {
    args: FileArgs,
    base64: Vec<u8>,
}

impl Terminal {
    /// An OSC 1337 string (after the `1337;`). True when it was an image, which the engine took;
    /// false for anything else iTerm2's protocol carries, which the host gets as an event.
    pub(in crate::terminal) fn iterm_image(&mut self, rest: &[u8]) -> bool {
        if let Some(rest) = rest.strip_prefix(b"File=") {
            let (args, payload) = split_colon(rest);
            let args = FileArgs::parse(&String::from_utf8_lossy(args));
            self.graphics.multipart = None;
            self.show_iterm(args, payload);
        } else if let Some(args) = rest.strip_prefix(b"MultipartFile=") {
            let args = FileArgs::parse(&String::from_utf8_lossy(args));
            self.graphics.multipart = Some(Multipart { args, base64: Vec::new() });
        } else if let Some(part) = rest.strip_prefix(b"FilePart=") {
            if let Some(multipart) = &mut self.graphics.multipart {
                if multipart.base64.len() + part.len() <= BASE64_MAX {
                    multipart.base64.extend_from_slice(part);
                } else {
                    self.graphics.multipart = None;
                }
            }
        } else if rest == b"FileEnd" {
            if let Some(Multipart { args, base64 }) = self.graphics.multipart.take() {
                self.show_iterm(args, &base64);
            }
        } else {
            return false;
        }
        true
    }

    fn show_iterm(&mut self, args: FileArgs, payload: &[u8]) {
        if !args.inline || payload.len() > BASE64_MAX {
            return;
        }
        let Some(data) = crate::terminal::osc::base64::decode(payload) else { return };
        let Some((width, height)) = sniff::dimensions(&data) else { return };
        if check_size(width, height).is_err() {
            return;
        }
        let Ok((handle, _)) = self.add_image(0, 0, width, height, data.len()) else { return };
        self.push_event(Event::ImageData(ImageUpload {
            image: handle,
            format: ImageFormat::File,
            compression: Compression::None,
            width,
            height,
            data,
        }));
        let cell = self.cell_size();
        self.put_image(Placing {
            image: handle,
            image_id: 0,
            placement_id: 0,
            geometry: args.geometry((width, height), cell, (self.cols, self.rows)),
            z: 0,
            motion: if args.do_not_move_cursor { Motion::Stay } else { Motion::Right },
        });
    }
}

/// `a * b / c`, at least 1, without overflowing.
fn mul_div(a: u64, b: u64, c: u64) -> u64 {
    (u128::from(a) * u128::from(b) / u128::from(c.max(1))).clamp(1, u128::from(u64::MAX)) as u64
}

fn split_colon(bytes: &[u8]) -> (&[u8], &[u8]) {
    match bytes.iter().position(|&b| b == b':') {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[]),
    }
}

#[cfg(test)]
mod tests {
    use super::{Dimension, FileArgs};
    use crate::types::Extent;

    const CELL: (u32, u32) = (8, 16);
    const SCREEN: (usize, usize) = (80, 24);

    #[test]
    fn the_arguments_read_with_iterms_units() {
        let args = FileArgs::parse(
            "name=Y2F0LnBuZw==;size=1234;width=40;height=50%;preserveAspectRatio=0;inline=1;doNotMoveCursor=1",
        );
        assert_eq!(
            args,
            FileArgs {
                width: Dimension::Cells(40),
                height: Dimension::Percent(50),
                preserve_aspect_ratio: false,
                inline: true,
                do_not_move_cursor: true,
            }
        );
        assert_eq!(FileArgs::parse(""), FileArgs::default());
        assert!(FileArgs::default().preserve_aspect_ratio && !FileArgs::default().inline);
        assert_eq!(FileArgs::parse("inline=1;width=300px;height=auto").width, Dimension::Pixels(300));
        assert_eq!(FileArgs::parse("inline=1;width=300px;height=auto").height, Dimension::Auto);
        assert_eq!(FileArgs::parse("width=lots;x").width, Dimension::Auto);
        assert_eq!(FileArgs::parse("width=250%").width, Dimension::Percent(100));
        assert_eq!(
            FileArgs::parse(" width = 5 ; inline = 1").width,
            Dimension::Cells(5),
            "blanks are skipped"
        );
    }

    #[test]
    fn without_a_size_the_picture_is_shown_at_its_own() {
        let g = FileArgs::default().geometry((100, 40), CELL, SCREEN);
        assert_eq!((g.cols, g.rows), (13, 3));
        assert_eq!(g.extent, Extent::Pixels { width: 100, height: 40 });
        assert_eq!((g.source.width, g.source.height), (100, 40));
    }

    #[test]
    fn a_picture_wider_than_the_terminal_is_scaled_down_with_its_shape() {
        let g = FileArgs::default().geometry((1280, 640), CELL, SCREEN);
        assert_eq!(g.cols, 80);
        assert_eq!(g.extent, Extent::Pixels { width: 640, height: 320 });
        assert_eq!(g.rows, 20);
    }

    #[test]
    fn one_size_keeps_the_shape_unless_told_not_to() {
        let kept = FileArgs::parse("width=10").geometry((100, 50), CELL, SCREEN);
        assert_eq!((kept.cols, kept.rows), (10, 3));
        assert_eq!(kept.extent, Extent::Pixels { width: 80, height: 40 });
        let free = FileArgs::parse("width=10;preserveAspectRatio=0").geometry((100, 50), CELL, SCREEN);
        assert_eq!((free.cols, free.rows, free.extent), (10, 4, Extent::Fill));
        let tall = FileArgs::parse("height=4").geometry((100, 50), CELL, SCREEN);
        assert_eq!(tall.extent, Extent::Pixels { width: 128, height: 64 });
        assert_eq!((tall.cols, tall.rows), (16, 4));
    }

    #[test]
    fn both_sizes_make_a_box_the_picture_fits_or_fills() {
        let fit = FileArgs::parse("width=20;height=5").geometry((100, 50), CELL, SCREEN);
        assert_eq!((fit.cols, fit.rows, fit.extent), (20, 5, Extent::Fit));
        let fill =
            FileArgs::parse("width=20;height=5;preserveAspectRatio=0").geometry((100, 50), CELL, SCREEN);
        assert_eq!((fill.cols, fill.rows, fill.extent), (20, 5, Extent::Fill));
        let percent = FileArgs::parse("width=50%;height=25%").geometry((100, 50), CELL, SCREEN);
        assert_eq!((percent.cols, percent.rows), (40, 6));
        let pixels = FileArgs::parse("width=100px;height=33px").geometry((100, 50), CELL, SCREEN);
        assert_eq!((pixels.cols, pixels.rows), (13, 3));
    }

    #[test]
    fn nothing_overflows_or_divides_by_zero() {
        let hostile = FileArgs::parse("width=4294967295;height=4294967295");
        let g = hostile.geometry((u32::MAX, u32::MAX), CELL, SCREEN);
        assert!(g.cols <= 80 && g.rows >= 1);
        let g = FileArgs::default().geometry((0, 0), (0, 0), (0, 0));
        assert!(g.cols >= 1 && g.rows >= 1);
    }
}
