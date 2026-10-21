//! A Sixel decoder: the data of `DCS P1 ; P2 ; P3 q <data> ST` to RGBA pixels.
//!
//! Spec: DEC VT330/VT340 Programmer Reference Manual, chapter 14 ("Sixel Graphics"), and ECMA-48
//! style numeric parameters. A sixel is a six-pixel-tall column, written as one character `?`
//! to `~` (bit 0 the top pixel) painted in the current color. Around them:
//!
//! - `# Pc` picks a color register, `# Pc ; Pu ; Px ; Py ; Pz` defines it first: `Pu` 1 is
//!   hue, lightness, saturation (hue in degrees, blue at 0, red at 120, green at 240), 2 is red,
//!   green, blue in percent. The registers start as the VT340's sixteen colors, then black.
//! - `! Pn c` repeats the column `c` `Pn` times; `$` returns to the left edge of the band;
//!   `-` starts the next band, six pixels down.
//! - `" Pan ; Pad ; Ph ; Pv` (the raster attributes, first in the data) give the pixel aspect
//!   ratio `Pan:Pad` (vertical to horizontal) and the size of the image. Without them the aspect
//!   is `P1`'s: 0 and 1 mean 2:1, 2 is 5:1, 3 and 4 are 3:1, 5 and 6 are 2:1, 7 to 9 are 1:1
//!   (and omitted is 0). A pixel taller than wide is drawn as that many rows.
//! - Anything else, line breaks included, is skipped.
//!
//! Pixels never painted are transparent (`P2` asks for exactly that when 1; for 0 and 2 the
//! terminal would paint its background, which looks the same on a terminal that draws pictures
//! over its own background). The data is walked twice: once to measure, then to paint, so the
//! picture is allocated once at its final size. Both walks are bounded: the size, the pixels and
//! the work (a repeat count is arithmetic until it paints).

use super::transmit::{MAX_DIMENSION, MAX_PIXELS};

/// Most pixel writes one picture may take: an 8K square, fully painted.
const WORK_BUDGET: u64 = 64 * 1024 * 1024;

/// Why a Sixel was not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SixelError {
    /// More pixels than a picture may have.
    TooBig,
    /// More painting than the budget.
    TooMuchWork,
    /// Nothing was painted.
    Empty,
}

/// A decoded picture.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Decoded {
    pub width: u32,
    pub height: u32,
    /// `width * height` pixels, red, green, blue, alpha.
    pub rgba: Vec<u8>,
}

/// The VT340's default colors (percent), registers 0 to 15.
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

/// DEC's hue, lightness, saturation to RGB.
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

/// How many rows a pixel is drawn as, from `P1` (the DECSIXEL table above).
fn aspect_from_p1(p1: u16) -> u32 {
    match p1 {
        2 => 5,
        3 | 4 => 3,
        7..=9 => 1,
        _ => 2,
    }
}

/// What one walk of the data meets.
struct Walk<'a> {
    data: &'a [u8],
    at: usize,
    x: u32,
    /// The band's top row, in sixel pixels (before the aspect).
    y: u32,
    color: usize,
    palette: Vec<[u8; 3]>,
    /// Painted extent so far.
    width: u32,
    height: u32,
    /// Size and aspect from the raster attributes: `(pan, pad, width, height)`.
    raster: Option<(u32, u32, u32, u32)>,
    work: u64,
}

impl<'a> Walk<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut palette = vec![[0u8; 3]; 256];
        for (slot, [r, g, b]) in palette.iter_mut().zip(DEFAULT_COLORS) {
            *slot = [percent(r.into()), percent(g.into()), percent(b.into())];
        }
        Walk { data, at: 0, x: 0, y: 0, color: 0, palette, width: 0, height: 0, raster: None, work: 0 }
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

    /// Walk the whole data; `paint` is called with the bits and the repeat of each column run.
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
                    self.numbers(&mut params);
                    if self.raster.is_none() {
                        let at = |i: usize| params.get(i).copied().unwrap_or(0);
                        self.raster = Some((at(0), at(1), at(2), at(3)));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Decode `data`. `p1` is the DCS's first parameter, the aspect when the data has no raster
/// attributes of its own.
pub(super) fn decode(data: &[u8], p1: u16) -> Result<Decoded, SixelError> {
    // First walk: the size.
    let mut measure = Walk::new(data);
    measure.run(|walk, bits, repeat| {
        if bits != 0 {
            walk.width = walk.width.max(walk.x.saturating_add(repeat));
            walk.height = walk.height.max(walk.y.saturating_add(6));
            if walk.width > MAX_DIMENSION || walk.height > MAX_DIMENSION {
                return Err(SixelError::TooBig);
            }
        }
        Ok(())
    })?;
    let (declared, rows_per_pixel) = match measure.raster {
        // The numbers are the stream's, up to u32::MAX: rounding pan / pad must not overflow.
        Some((pan, pad, w, h)) => ((w, h), pan.saturating_add(pad.max(1) / 2) / pad.max(1)),
        None => ((0, 0), aspect_from_p1(p1)),
    };
    let rows_per_pixel = rows_per_pixel.clamp(1, 10);
    let (width, height) = (measure.width.max(declared.0), measure.height.max(declared.1));
    if width == 0 || measure.width == 0 || height == 0 {
        return Err(SixelError::Empty);
    }
    let out_height = height.saturating_mul(rows_per_pixel);
    if width > MAX_DIMENSION
        || out_height > MAX_DIMENSION
        || u64::from(width) * u64::from(out_height) > MAX_PIXELS
    {
        return Err(SixelError::TooBig);
    }
    let mut rgba = vec![0u8; width as usize * out_height as usize * 4];
    // Second walk: the pixels.
    let mut paint = Walk::new(data);
    paint.run(|walk, bits, repeat| {
        if bits == 0 || walk.x >= width {
            return Ok(());
        }
        let columns = repeat.min(width - walk.x);
        walk.work += u64::from(columns) * 6 * u64::from(rows_per_pixel);
        if walk.work > WORK_BUDGET {
            return Err(SixelError::TooMuchWork);
        }
        let [r, g, b] = walk.palette[walk.color];
        let pixel = [r, g, b, 255];
        let across = columns as usize;
        // Only the set bits, each as the pixel rows it stands for.
        let mut bits = bits;
        while bits != 0 {
            let bit = bits.trailing_zeros();
            bits &= bits - 1;
            let first = (walk.y + bit) * rows_per_pixel;
            for y in first..(first + rows_per_pixel).min(out_height) {
                let row = (y as usize * width as usize + walk.x as usize) * 4;
                if across == 1 {
                    rgba[row..row + 4].copy_from_slice(&pixel);
                } else {
                    for px in rgba[row..row + across * 4].as_chunks_mut::<4>().0 {
                        *px = pixel;
                    }
                }
            }
        }
        Ok(())
    })?;
    Ok(Decoded { width, height: out_height, rgba })
}

#[cfg(test)]
mod tests {
    use super::{Decoded, SixelError, aspect_from_p1, decode, hls_to_rgb};

    fn pixel(image: &Decoded, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * image.width + x) * 4) as usize;
        [image.rgba[at], image.rgba[at + 1], image.rgba[at + 2], image.rgba[at + 3]]
    }

    /// 1:1 pixels, so that rows are the sixels' own.
    fn square(data: &[u8]) -> Result<Decoded, SixelError> {
        decode(data, 9)
    }

    #[test]
    fn columns_colors_repeats_and_bands_paint_where_they_should() {
        // Red: a full column, then three columns with only the top pixel; next band in green, a
        // column with pixel 5 after a carriage return re-paints column 0 in blue on top.
        let data = b"#1;2;100;0;0~!3@-#2;2;0;100;0?A$#3;2;0;0;100_";
        let image = square(data).expect("decodes");
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
        let image = square(b"#2~").expect("decodes");
        assert_eq!(pixel(&image, 0, 0), [204, 33, 33, 255], "the VT340's red");
        assert_eq!(hls_to_rgb(120, 50, 100), [255, 0, 0], "DEC's hue 120 is red");
        assert_eq!(hls_to_rgb(0, 50, 100), [0, 0, 255]);
        assert_eq!(hls_to_rgb(240, 50, 100), [0, 255, 0]);
        assert_eq!(hls_to_rgb(0, 100, 0), [255, 255, 255]);
        // Defining a register selects it.
        let hls = square(b"#5;1;120;50;100~").expect("decodes");
        assert_eq!(pixel(&hls, 0, 0), [255, 0, 0, 255]);
        let rgb = square(b"#5;2;0;0;100#0~#5~").expect("decodes");
        assert_eq!(pixel(&rgb, 1, 0), [0, 0, 255, 255], "and it stays defined");
    }

    #[test]
    fn the_aspect_comes_from_the_raster_attributes_or_p1() {
        // 2:1 pixels are two rows each.
        let tall = decode(b"\"2;1;1;6~", 9).expect("decodes");
        assert_eq!((tall.width, tall.height), (1, 12));
        assert_eq!(pixel(&tall, 0, 11), [0, 0, 0, 255]);
        // Without them, P1: omitted (0) is 2:1, 9 is 1:1.
        assert_eq!(decode(b"~", 0).expect("decodes").height, 12);
        assert_eq!(decode(b"~", 9).expect("decodes").height, 6);
        assert_eq!(decode(b"~", 2).expect("decodes").height, 30);
        assert_eq!([0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map(aspect_from_p1), [2, 2, 5, 3, 3, 2, 2, 1, 1, 1]);
        // The raster attributes win over P1.
        assert_eq!(decode(b"\"1;1;1;6~", 0).expect("decodes").height, 6);
    }

    #[test]
    fn the_declared_size_is_the_least_the_picture_is() {
        let image = decode(b"\"1;1;10;12~", 9).expect("decodes");
        assert_eq!((image.width, image.height), (10, 12));
        assert_eq!(pixel(&image, 9, 11), [0, 0, 0, 0]);
        // Painting outside the declared size makes it larger, not an error.
        assert_eq!(decode(b"\"1;1;2;6!5~", 9).expect("decodes").width, 5);
    }

    #[test]
    fn nothing_painted_is_no_image_and_too_much_is_refused() {
        assert_eq!(square(b""), Err(SixelError::Empty));
        assert_eq!(square(b"\"1;1;10;10???"), Err(SixelError::Empty));
        assert_eq!(square(b"!101?"), Err(SixelError::Empty), "all blank");
        assert_eq!(square(b"!10001~"), Err(SixelError::TooBig), "wider than allowed");
        let mut tall = Vec::new();
        for _ in 0..1700 {
            tall.extend_from_slice(b"~-");
        }
        assert_eq!(square(&tall), Err(SixelError::TooBig), "taller than allowed");
        assert_eq!(square(b"!4000000000~"), Err(SixelError::TooBig));
        // Within the sides but over the pixels: 9,000 by 2,000 is 18 million.
        let mut wide = b"!9000~".to_vec();
        for _ in 0..333 {
            wide.extend_from_slice(b"-!9000~");
        }
        assert_eq!(square(&wide), Err(SixelError::TooBig));
        assert_eq!(
            decode(b"\"1;1;100000;6~", 9),
            Err(SixelError::TooBig),
            "a declared size is held to the same"
        );
    }

    #[test]
    fn hostile_data_never_panics_and_costs_bounded_work() {
        let mut rng = 0x9E37_79B9_u32;
        for len in [0usize, 1, 7, 100, 1000] {
            for _ in 0..50 {
                let data: Vec<u8> = (0..len)
                    .map(|_| {
                        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        b"#;!\"$-?~0123456789 @"[(rng >> 24) as usize % 20]
                    })
                    .collect();
                let _ = decode(&data, (rng >> 8) as u16);
            }
        }
        // A repeat of two billion across the whole width, band after band, runs out of budget.
        let mut heavy = Vec::new();
        for _ in 0..2000 {
            heavy.extend_from_slice(b"!9999~-");
        }
        assert!(matches!(square(&heavy), Err(SixelError::TooBig | SixelError::TooMuchWork)));
    }

    #[test]
    fn a_picture_as_wide_as_allowed_is_drawn() {
        let image = square(b"!10000~").expect("decodes");
        assert_eq!((image.width, image.height), (10_000, 6));
    }

    #[test]
    fn hls_colors_follow_lightness_and_saturation_and_the_hue_goes_round() {
        assert_eq!(hls_to_rgb(120, 50, 50), [191, 64, 64], "red, half saturated");
        assert_eq!(hls_to_rgb(120, 25, 100), [128, 0, 0], "a dark red");
        assert_eq!(hls_to_rgb(0, 50, 0), [128, 128, 128], "no saturation is gray");
        assert_eq!(hls_to_rgb(480, 50, 50), hls_to_rgb(120, 50, 50), "a turn and a third");
        assert_eq!(hls_to_rgb(u32::MAX, 50, 50), hls_to_rgb(u32::MAX % 360, 50, 50), "the biggest hue");
    }

    #[test]
    fn hues_between_the_primaries_mix_their_two_neighbours_in_every_sector_of_the_wheel() {
        // Full saturation at half lightness; DEC's degrees have blue at 0, red at 120, green at 240.
        assert_eq!(hls_to_rgb(150, 50, 100), [255, 128, 0], "from red toward green");
        assert_eq!(hls_to_rgb(210, 50, 100), [128, 255, 0], "from there to green");
        assert_eq!(hls_to_rgb(330, 50, 100), [0, 128, 255], "from cyan toward blue");
    }

    #[test]
    fn a_picture_may_be_as_tall_and_as_big_as_allowed_and_no_more() {
        // The size the raster attributes declare counts as the painted one.
        let tall = decode(b"\"1;1;1;10000~", 9).expect("decodes");
        assert_eq!((tall.width, tall.height), (1, 10_000));
        assert_eq!(decode(b"\"1;1;1;10001~", 9), Err(SixelError::TooBig));
        // 4,096 on both sides are the 16 Mi pixels of the most.
        let most = decode(b"\"1;1;4096;4096~", 9).expect("decodes");
        assert_eq!((most.width, most.height), (4096, 4096));
        assert_eq!(decode(b"\"1;1;4097;4096~", 9), Err(SixelError::TooBig));
    }

    #[test]
    fn blank_columns_cost_no_work() {
        // A band painted once, then 1,200 runs of 9,999 columns of nothing: 72 million of work if
        // they were counted, over the budget.
        let mut data = b"!9999~$".to_vec();
        for _ in 0..1200 {
            data.extend_from_slice(b"!9999?$");
        }
        let image = square(&data).expect("blank runs are free");
        assert_eq!((image.width, image.height), (9999, 6));
    }

    #[test]
    fn painting_over_and_over_is_held_to_the_work_budget() {
        // Ten rows to a pixel and 10,000 columns of one sixel row, each time: 600,000 of work for a
        // picture of 600,000 pixels. The budget is 64 Mi: 111 times is within it, 112 is not.
        let over = |times: usize| {
            let mut data = b"\"10;1;1;6".to_vec();
            for _ in 0..times {
                data.extend_from_slice(b"!10000@$");
            }
            decode(&data, 9)
        };
        let image = over(111).expect("66.6 million of work");
        assert_eq!((image.width, image.height), (10_000, 60));
        assert_eq!(over(112), Err(SixelError::TooMuchWork), "67.2 million");
    }

    /// Found by the fuzzer (`graphics`, `term`, `split`): an aspect ratio of two huge numbers
    /// overflowed the rounding of pan / pad.
    #[test]
    fn a_huge_aspect_ratio_is_clamped_not_overflowed() {
        for raster in
            ["\"4294967295;1;4;6", "\"1;4294967295;4;6", "\"4294967295;4294967295;4;6", "\"4294967295;0;4;6"]
        {
            let data = format!("{raster}#1;2;100;0;0~~~~");
            let decoded = decode(data.as_bytes(), 0).expect("a picture");
            assert_eq!(decoded.width, 4);
        }
    }
}
