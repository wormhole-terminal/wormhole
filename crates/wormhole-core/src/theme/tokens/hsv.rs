//! A colour as hue, saturation and value: the model of a colour picker's area and hue slider, where
//! dragging along one axis changes one number. [`Color`] stays the stored form (a hex code); this is
//! only the picker's working copy, so a hue survives while the colour passes through a grey or black
//! (which a hex code can't remember).

use super::Color;

/// Hue in degrees (`0.0..360.0`), saturation and value in `0.0..=1.0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    /// A colour from its three numbers. Out-of-range input is brought in (a pointer past the edge of
    /// an area asks for the edge): a hue wraps, the other two stop at 0 and 1. That limits the numbers
    /// of the control, never the colour a person can have: every colour of the sRGB cube is some
    /// `Hsv`.
    pub fn new(h: f32, s: f32, v: f32) -> Self {
        Self { h: h.rem_euclid(360.0), s: s.clamp(0.0, 1.0), v: v.clamp(0.0, 1.0) }
    }

    /// The hue, saturation and value of `color` (its alpha is not part of it).
    pub fn from_color(color: Color) -> Self {
        let (r, g, b) = (color.r, color.g, color.b);
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        Self::new(h, if max == 0.0 { 0.0 } else { delta / max }, max)
    }

    /// The opaque colour.
    pub fn to_color(self) -> Color {
        let Self { h, s, v } = self;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Color::rgba(r + m, g + m, b + m, 1.0)
    }

    /// The fully saturated, fully bright colour of this hue: what the area's top right corner is.
    pub fn pure_hue(self) -> Color {
        Self::new(self.h, 1.0, 1.0).to_color()
    }
}

#[cfg(test)]
mod tests {
    use super::{Color, Hsv};

    #[test]
    fn every_colour_comes_back_as_the_same_hex() {
        // Every channel byte on a coarse grid, and the extremes: the hex a picker writes is the one it read.
        let steps = [0u32, 1, 17, 64, 127, 128, 200, 254, 255];
        for r in steps {
            for g in steps {
                for b in steps {
                    let color = Color::from_rgb_u32(r << 16 | g << 8 | b);
                    assert_eq!(Hsv::from_color(color).to_color().to_hex(), color.to_hex(), "{r} {g} {b}");
                }
            }
        }
    }

    #[test]
    fn the_primaries_are_where_the_wheel_puts_them() {
        let hue = |hex: &str| Color::from_hex(hex).map(|c| Hsv::from_color(c).h);
        assert_eq!(hue("#ff0000"), Ok(0.0));
        assert_eq!(hue("#00ff00"), Ok(120.0));
        assert_eq!(hue("#0000ff"), Ok(240.0));
        assert_eq!(hue("#ffff00"), Ok(60.0));
    }

    #[test]
    fn grey_and_black_have_no_saturation_but_keep_the_hue_they_are_given() {
        let grey = Hsv::from_color(Color::from_rgb_u32(0x808080));
        assert_eq!((grey.h, grey.s), (0.0, 0.0));
        // A picker that dragged to black keeps its hue: the colour is black, the numbers are not.
        let black = Hsv::new(200.0, 0.7, 0.0);
        assert_eq!(black.to_color().to_hex(), "#000000");
        assert_eq!(Hsv::new(200.0, 0.7, 0.5).h, 200.0);
    }

    #[test]
    fn a_pointer_past_an_edge_is_brought_to_it() {
        assert_eq!(Hsv::new(-30.0, 2.0, -1.0), Hsv { h: 330.0, s: 1.0, v: 0.0 });
        assert_eq!(Hsv::new(360.0, 0.5, 0.5).h, 0.0);
    }

    #[test]
    fn the_pure_hue_is_the_corner_of_the_area() {
        assert_eq!(Hsv::new(0.0, 0.2, 0.3).pure_hue().to_hex(), "#ff0000");
        assert_eq!(Hsv::new(240.0, 0.2, 0.3).pure_hue().to_hex(), "#0000ff");
    }
}
