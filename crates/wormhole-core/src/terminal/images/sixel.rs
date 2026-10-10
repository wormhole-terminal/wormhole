//! A Sixel decoder: `DCS P1 ; P2 ; P3 q <data> ST` to RGBA.
//!
//! The data is a stream of six-pixel-tall columns (`?`..`~`, bit 0 the top pixel) in the current
//! color, with `#n` picking and `#n;2;r;g;b` (percent) or `#n;1;h;l;s` defining colors, `!n`
//! repeating the next column, `$` returning to the left edge of the band and `-` starting the
//! next band. Pixels never set stay transparent (`P2=1` asks for that; otherwise they would be
//! the background, which looks the same over the terminal's). The data is walked twice: once to
//! measure, once to paint, so the image is allocated once at its final size. Every walk is
//! bounded: the dimensions, the pixels and the work (a repeat count is arithmetic until it paints).

use std::time::Instant;

use super::{Image, Limits};

/// Most pixel writes one image may take: an 8K square, fully painted.
const WORK_BUDGET: u64 = 64 * 1024 * 1024;
/// How long a decode may run before it gives up (the PTY thread runs it).
const TIME_BUDGET_MS: u128 = 500;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SixelError {
    TooBig,
    /// Took longer than the budget, or more work.
    TooMuchWork,
    Empty,
}

/// The VT340's default colors (percent), as libsixel has them.
const DEFAULT_COLORS: [[u8; 3]; 16] = [
    [0, 0, 0],
    [20, 20, 80],
    [80, 13, 13],
    [20, 80, 20],
    [80, 20, 80],
    [20, 80, 80],
    [80, 80, 20],
    [53, 53, 53],
    [26, 26, 26],
    [33, 33, 60],
    [60, 26, 26],
    [33, 60, 33],
    [60, 33, 60],
    [33, 60, 60],
    [60, 60, 33],
    [80, 80, 80],
];

fn percent(v: u32) -> u8 {
    (v.min(100) * 255 / 100) as u8
}

/// DEC's HLS (hue 0 is blue, 120 red, 240 green) to RGB.
fn hls_to_rgb(h: u32, l: u32, s: u32) -> [u8; 3] {
    let h = ((h % 360) as f32 + 240.0) % 360.0;
    let l = l.min(100) as f32 / 100.0;
    let s = s.min(100) as f32 / 100.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let byte = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(r), byte(g), byte(b)]
}

/// What one walk of the data meets.
struct Walk<'a> {
    data: &'a [u8],
    at: usize,
    x: u32,
    /// The band's top pixel row.
    y: u32,
    color: usize,
    palette: Vec<[u8; 3]>,
    width: u32,
    height: u32,
    work: u64,
    started: Instant,
}

impl<'a> Walk<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut palette = vec![[0u8; 3]; 256];
        for (slot, [r, g, b]) in palette.iter_mut().zip(DEFAULT_COLORS) {
            *slot = [percent(r.into()), percent(g.into()), percent(b.into())];
        }
        Self {
            data,
            at: 0,
            x: 0,
            y: 0,
            color: 0,
            palette,
            width: 0,
            height: 0,
            work: 0,
            started: Instant::now(),
        }
    }

    fn number(&mut self) -> u32 {
        let mut n: u32 = 0;
        while let Some(&c) = self.data.get(self.at) {
            if !c.is_ascii_digit() {
                break;
            }
            n = n.saturating_mul(10).saturating_add(u32::from(c - b'0'));
            self.at += 1;
        }
        n
    }

    /// `;`-separated numbers after an introducer.
    fn numbers(&mut self, out: &mut Vec<u32>) {
        out.clear();
        loop {
            out.push(self.number());
            if self.data.get(self.at) == Some(&b';') {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    /// Walk the whole data, `paint` called with (x, y, bits, repeat) for each column run.
    fn run(
        &mut self,
        mut paint: impl FnMut(&mut Self, u8, u32) -> Result<(), SixelError>,
    ) -> Result<(), SixelError> {
        let mut params = Vec::new();
        let mut repeat = 1u32;
        while self.at < self.data.len() {
            let c = self.data[self.at];
            self.at += 1;
            match c {
                b'?'..=b'~' => {
                    paint(self, c - b'?', repeat)?;
                    self.x = self.x.saturating_add(repeat);
                    repeat = 1;
                }
                b'!' => repeat = self.number().max(1),
                b'$' => self.x = 0,
                b'-' => {
                    self.x = 0;
                    self.y = self.y.saturating_add(6);
                }
                b'#' => {
                    self.numbers(&mut params);
                    let index = params.first().copied().unwrap_or(0) as usize % 256;
                    self.color = index;
                    if let [_, system, a, b, c2, ..] = params[..] {
                        self.palette[index] = match system {
                            1 => hls_to_rgb(a, b, c2),
                            _ => [percent(a), percent(b), percent(c2)],
                        };
                    }
                }
                b'"' => {
                    // Raster attributes: the size they announce is a hint; the data decides.
                    self.numbers(&mut params);
                }
                _ => {}
            }
            if self.at & 0xFFF == 0 && self.started.elapsed().as_millis() > TIME_BUDGET_MS {
                return Err(SixelError::TooMuchWork);
            }
        }
        Ok(())
    }
}

/// Decode `data` within `limits`.
pub fn decode(data: &[u8], limits: &Limits) -> Result<Image, SixelError> {
    // First walk: the size.
    let mut measure = Walk::new(data);
    let max = limits.max_dimension;
    measure.run(|walk, bits, repeat| {
        if bits != 0 {
            walk.width = walk.width.max(walk.x.saturating_add(repeat));
            walk.height = walk.height.max(walk.y.saturating_add(6));
            if walk.width > max || walk.height > max {
                return Err(SixelError::TooBig);
            }
        }
        Ok(())
    })?;
    let (width, height) = (measure.width, measure.height);
    if width == 0 || height == 0 {
        return Err(SixelError::Empty);
    }
    if u64::from(width) * u64::from(height) > limits.max_pixels {
        return Err(SixelError::TooBig);
    }
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    // Second walk: the pixels.
    let mut paint = Walk::new(data);
    paint.run(|walk, bits, repeat| {
        if bits == 0 || walk.x >= width {
            return Ok(());
        }
        let columns = repeat.min(width - walk.x);
        walk.work += u64::from(columns) * 6;
        if walk.work > WORK_BUDGET {
            return Err(SixelError::TooMuchWork);
        }
        let [r, g, b] = walk.palette[walk.color];
        for bit in 0..6 {
            if bits & (1 << bit) == 0 {
                continue;
            }
            let y = walk.y + bit;
            if y >= height {
                break;
            }
            let row = (y * width + walk.x) as usize * 4;
            for px in rgba[row..row + columns as usize * 4].as_chunks_mut::<4>().0 {
                *px = [r, g, b, 255];
            }
        }
        Ok(())
    })?;
    Ok(Image { width, height, rgba })
}

#[cfg(test)]
mod tests {
    use super::{SixelError, decode, hls_to_rgb};
    use crate::terminal::images::Limits;

    fn pixel(image: &super::Image, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * image.width + x) * 4) as usize;
        [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2], image.rgba[at + 3]]
    }

    #[test]
    fn columns_colors_repeats_and_bands_paint_where_they_should() {
        // Red: a full column, then three columns with only the top pixel; next band in green, a
        // column with pixel 5 after a carriage return re-paints column 0 in blue on top.
        let data = b"#1;2;100;0;0~!3@-#2;2;0;100;0?A$#3;2;0;0;100_";
        let image = decode(data, &Limits::default()).expect("decodes");
        assert_eq!((image.width, image.height), (4, 12));
        assert_eq!(pixel(&image, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 0, 5), [255, 0, 0, 255]);
        assert_eq!(pixel(&image, 3, 0), [255, 0, 0, 255], "the repeat");
        assert_eq!(pixel(&image, 3, 1), [0, 0, 0, 0], "unset pixels are transparent");
        assert_eq!(pixel(&image, 1, 7), [0, 255, 0, 255], "the second band, `A` is bit 1");
        assert_eq!(pixel(&image, 1, 6), [0, 0, 0, 0]);
        assert_eq!(pixel(&image, 0, 11), [0, 0, 255, 255], "`_` is bit 5, after `$`");
        assert_eq!(pixel(&image, 0, 6), [0, 0, 0, 0]);
    }

    #[test]
    fn the_default_palette_and_hls_colors() {
        let image = decode(b"#2~", &Limits::default()).expect("decodes");
        assert_eq!(pixel(&image, 0, 0), [204, 33, 33, 255], "the VT340's red");
        assert_eq!(hls_to_rgb(120, 50, 100), [255, 0, 0], "DEC's hue 120 is red");
        assert_eq!(hls_to_rgb(0, 50, 100), [0, 0, 255]);
        assert_eq!(hls_to_rgb(240, 50, 100), [0, 255, 0]);
        assert_eq!(hls_to_rgb(0, 100, 0), [255, 255, 255]);
    }

    #[test]
    fn nothing_painted_is_no_image_and_too_much_is_refused() {
        assert_eq!(decode(b"", &Limits::default()), Err(SixelError::Empty));
        assert_eq!(decode(b"\"1;1;10;10???", &Limits::default()), Err(SixelError::Empty));
        let limits = Limits { max_dimension: 100, max_pixels: 1000, max_bytes: 1 << 20 };
        assert_eq!(decode(b"!101~", &limits), Err(SixelError::TooBig), "wider than allowed");
        let mut tall = Vec::new();
        for _ in 0..20 {
            tall.extend_from_slice(b"~-");
        }
        assert_eq!(decode(&tall, &limits), Err(SixelError::TooBig), "taller than allowed");
        // A repeat beyond the width costs nothing: the image is the width the data paints.
        let image = decode(b"!50~", &limits).expect("decodes");
        assert_eq!((image.width, image.height), (50, 6));
        assert_eq!(decode(b"!4000000000~", &Limits::default()), Err(SixelError::TooBig));
    }
}
