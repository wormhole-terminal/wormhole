//! Color math for design tokens: hex, blending, compositing, OKLab lightness
//! and WCAG contrast. Re-exported as `tokens::Color`.

/// Straight (non-premultiplied) sRGB color with components in 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const BLACK: Color = Color::rgba(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Color = Color::rgba(1.0, 1.0, 1.0, 1.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// From packed `0xRRGGBB`, the format used by [`crate::theme::Theme`].
    pub fn from_rgb_u32(c: u32) -> Self {
        let ch = |s: u32| ((c >> s) & 0xff) as f32 / 255.0;
        Self::rgba(ch(16), ch(8), ch(0), 1.0)
    }

    /// Packed `0xRRGGBB`; alpha is dropped.
    pub fn to_rgb_u32(self) -> u32 {
        let [r, g, b, _] = self.to_bytes();
        (r as u32) << 16 | (g as u32) << 8 | b as u32
    }

    /// The 8-bit channels a hex string would show; equal bytes mean an equal `to_hex()`.
    pub(crate) fn to_bytes(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    /// The color its `to_hex()` reads back as.
    pub(crate) fn quantized(self) -> Self {
        let [r, g, b, a] = self.to_bytes().map(|v| v as f32 / 255.0);
        Self::rgba(r, g, b, a)
    }

    /// Parse `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` (the `#` is optional).
    pub fn from_hex(s: &str) -> Result<Self, String> {
        let h = s.trim();
        let h = h.strip_prefix('#').unwrap_or(h);
        let bad = || format!("invalid hex color {s:?} (expected #rgb, #rrggbb or #rrggbbaa)");
        if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let digits: Vec<u8> = h
            .bytes()
            .map(|b| match b {
                b'0'..=b'9' => b - b'0',
                _ => (b | 0x20) - b'a' + 10,
            })
            .collect();
        let bytes: Vec<u8> = match digits.len() {
            3 | 4 => digits.iter().map(|d| d * 17).collect(),
            6 | 8 => digits.chunks(2).map(|p| p[0] * 16 + p[1]).collect(),
            _ => return Err(bad()),
        };
        let f = |i: usize| bytes.get(i).map_or(1.0, |&v| v as f32 / 255.0);
        Ok(Self::rgba(f(0), f(1), f(2), f(3)))
    }

    /// `#rrggbb`, or `#rrggbbaa` when not fully opaque.
    pub fn to_hex(self) -> String {
        let [r, g, b, a] = self.to_bytes();
        if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Self { a: a.clamp(0.0, 1.0), ..self }
    }

    /// Blend in gamma-encoded sRGB (like CSS `color-mix(in srgb)`): `t` = 0 is
    /// `self`, 1 is `other`. Alpha blends too.
    pub fn mix(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let l = |a: f32, b: f32| a + (b - a) * t;
        Self::rgba(l(self.r, other.r), l(self.g, other.g), l(self.b, other.b), l(self.a, other.a))
    }

    /// Source-over compositing onto `bg`, as a browser would draw it.
    pub fn over(self, bg: Color) -> Self {
        let a = self.a + bg.a * (1.0 - self.a);
        if a <= 0.0 {
            return Self::rgba(0.0, 0.0, 0.0, 0.0);
        }
        let c = |f: f32, b: f32| (f * self.a + b * bg.a * (1.0 - self.a)) / a;
        Self::rgba(c(self.r, bg.r), c(self.g, bg.g), c(self.b, bg.b), a)
    }

    /// Raise OKLab lightness by `t` (absolute, lightness is 0..1). OKLab is
    /// used rather than HSL so that equal steps look equal across hues (HSL
    /// "lighten 10%" is a big jump for blue and barely visible for yellow).
    pub fn lighten(self, t: f32) -> Self {
        let (l, a, b) = self.to_oklab();
        Self::from_oklab((l + t).clamp(0.0, 1.0), a, b, self.a)
    }

    /// Lower OKLab lightness by `t`; see [`Color::lighten`].
    pub fn darken(self, t: f32) -> Self {
        self.lighten(-t)
    }

    /// WCAG 2.x relative luminance (alpha ignored; composite first).
    pub fn luminance(self) -> f32 {
        fn lin(v: f32) -> f32 {
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        }
        0.2126 * lin(self.r) + 0.7152 * lin(self.g) + 0.0722 * lin(self.b)
    }

    /// WCAG contrast ratio, 1..21, symmetric. Alpha is ignored, so composite
    /// translucent colors with [`Color::over`] first.
    pub fn contrast_ratio(a: Color, b: Color) -> f32 {
        let (la, lb) = (a.luminance(), b.luminance());
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Black or white, whichever contrasts more with `self`.
    pub fn text_on(self) -> Color {
        let bg = self.over(Color::BLACK);
        if Self::contrast_ratio(Color::WHITE, bg) >= Self::contrast_ratio(Color::BLACK, bg) {
            Color::WHITE
        } else {
            Color::BLACK
        }
    }

    /// `self` moved along OKLab lightness toward white or black (whichever
    /// contrasts better with `bg`) just far enough to reach `ratio` against
    /// opaque `bg`. Keeps alpha; contrast is measured on the composite.
    pub fn readable_on(self, bg: Color, ratio: f32) -> Color {
        let ratio_of = |c: Color| Self::contrast_ratio(c.over(bg), bg);
        if ratio_of(self) >= ratio {
            return self;
        }
        let toward_white = Self::contrast_ratio(Color::WHITE, bg) >= Self::contrast_ratio(Color::BLACK, bg);
        let step = |t: f32| if toward_white { self.lighten(t) } else { self.darken(t) };
        if ratio_of(step(1.0)) < ratio {
            // Unreachable for opaque text (black or white always reach 4.58:1),
            // but a very translucent color may never get there.
            return step(1.0);
        }
        // Lightness vs. contrast is monotonic, so bisect for the smallest move.
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..24 {
            let mid = (lo + hi) / 2.0;
            if ratio_of(step(mid)) >= ratio { hi = mid } else { lo = mid }
        }
        step(hi)
    }

    /// OKLab `(L, a, b)`.
    pub(crate) fn to_oklab(self) -> (f32, f32, f32) {
        fn lin(v: f32) -> f32 {
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        }
        let (r, g, b) = (lin(self.r), lin(self.g), lin(self.b));
        let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
        let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
        let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
        (
            0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        )
    }

    /// OKLCH `(L, chroma, hue in degrees 0..360)`: OKLab's polar form, where a hue is a number.
    pub fn to_oklch(self) -> (f32, f32, f32) {
        let (l, a, b) = self.to_oklab();
        (l, a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0))
    }

    /// The colour of OKLCH `(l, chroma, hue)`, opaque; one the sRGB gamut can't hold loses chroma,
    /// keeping its lightness and hue (see `from_oklab`).
    pub fn from_oklch(l: f32, chroma: f32, hue: f32) -> Self {
        let hue = hue.to_radians();
        Self::from_oklab(l, chroma * hue.cos(), chroma * hue.sin(), 1.0)
    }

    /// Converts back to sRGB. Out-of-gamut results keep lightness and hue and
    /// shed chroma, which looks far better than per-channel clipping (that
    /// shifts hue, e.g. a lightened blue turning cyan).
    fn from_oklab(l: f32, a: f32, b: f32, alpha: f32) -> Self {
        fn raw(l: f32, a: f32, b: f32) -> [f32; 3] {
            let l_ = (l + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
            let m_ = (l - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
            let s_ = (l - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
            [
                4.076_741_7 * l_ - 3.307_711_6 * m_ + 0.230_969_94 * s_,
                -1.268_438 * l_ + 2.609_757_4 * m_ - 0.341_319_38 * s_,
                -0.004_196_086_3 * l_ - 0.703_418_6 * m_ + 1.707_614_7 * s_,
            ]
        }
        fn in_gamut(c: [f32; 3]) -> bool {
            c.iter().all(|v| (-1e-4..=1.0 + 1e-4).contains(v))
        }
        // Chroma must vanish at the extremes; without this, rounding leaves a
        // faint tint on "black" and "white".
        if l <= 0.0 {
            return Self::rgba(0.0, 0.0, 0.0, alpha);
        }
        if l >= 1.0 {
            return Self::rgba(1.0, 1.0, 1.0, alpha);
        }
        let mut rgb = raw(l, a, b);
        if !in_gamut(rgb) {
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..20 {
                let mid = (lo + hi) / 2.0;
                if in_gamut(raw(l, a * mid, b * mid)) { lo = mid } else { hi = mid }
            }
            rgb = raw(l, a * lo, b * lo);
        }
        fn gamma(v: f32) -> f32 {
            let v = v.clamp(0.0, 1.0);
            if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
        }
        Self::rgba(gamma(rgb[0]), gamma(rgb[1]), gamma(rgb[2]), alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    fn hex(s: &str) -> Color {
        Color::from_hex(s).unwrap()
    }

    #[test]
    fn hex_parsing() {
        assert_eq!(hex("#ff0000"), Color::rgba(1.0, 0.0, 0.0, 1.0));
        assert_eq!(hex("#f00"), hex("#ff0000"));
        assert_eq!(hex("00ff0080").to_hex(), "#00ff0080");
        assert_eq!(hex("#f008").to_hex(), "#ff000088");
        assert_eq!(hex("#ABCDEF").to_hex(), "#abcdef");
        for bad in ["", "#", "#12", "#12345", "#1234567", "#gggggg", "#12 345"] {
            assert!(Color::from_hex(bad).is_err(), "{bad}");
        }
        for s in ["#000000", "#ffffff", "#1a1b26", "#7aa2f7cc"] {
            assert_eq!(hex(s).to_hex(), s);
        }
        assert_eq!(Color::from_rgb_u32(0x1a1b26).to_rgb_u32(), 0x1a1b26);
    }

    #[test]
    fn mix_alpha_over() {
        let (a, b) = (hex("#000000"), hex("#ffffff"));
        assert_eq!(a.mix(b, 0.0), a);
        assert_eq!(a.mix(b, 1.0), b);
        assert_eq!(a.mix(b, 0.5).to_hex(), "#808080");
        assert_eq!(a.mix(b, 7.0), b, "t is clamped");
        assert_eq!(b.with_alpha(0.5).to_hex(), "#ffffff80");
        assert_eq!(b.with_alpha(0.5).over(a).to_hex(), "#808080");
        assert_eq!(b.with_alpha(0.0).over(a), a);
        assert_eq!(a.over(b), a);
        let half = hex("#ff000080").over(hex("#0000ff80"));
        assert!((half.a - 0.75).abs() < 0.01);
    }

    #[test]
    fn lighten_darken_keep_hue() {
        let blue = hex("#3366cc");
        let (light, dark) = (blue.lighten(0.1), blue.darken(0.1));
        assert!(light.luminance() > blue.luminance());
        assert!(dark.luminance() < blue.luminance());
        // Hue (OKLab a/b direction) stays put.
        let angle = |c: Color| {
            let (_, a, b) = c.to_oklab();
            b.atan2(a)
        };
        assert!((angle(light) - angle(blue)).abs() < 0.05);
        assert!((angle(dark) - angle(blue)).abs() < 0.05);
        assert_eq!(blue.lighten(1.0).to_hex(), "#ffffff");
        assert_eq!(blue.darken(1.0).to_hex(), "#000000");
        // Round trip through OKLab is lossless at 8 bits.
        for s in ["#1a1b26", "#f7768e", "#9ece6a", "#ffffff", "#000000", "#0969da"] {
            assert_eq!(hex(s).lighten(0.0).to_hex(), s);
        }
        assert_eq!(blue.with_alpha(0.5).lighten(0.1).a, 0.5);
    }

    #[test]
    fn oklch_is_oklab_in_polar_form() {
        for s in ["#e5484d", "#2bb3a3", "#7aa2f7", "#f5d90a"] {
            let (l, c, h) = hex(s).to_oklch();
            assert!((0.0..360.0).contains(&h) && c > 0.05, "{s}: {c} {h}");
            assert_eq!(Color::from_oklch(l, c, h).to_hex(), s);
        }
        // Out of the gamut it loses chroma, and keeps its hue.
        let (_, chroma, hue) = Color::from_oklch(0.5, 0.4, 260.0).to_oklch();
        assert!(chroma < 0.4 && (hue - 260.0).abs() < 3.0, "{chroma} {hue}");
        assert_eq!(Color::from_oklch(0.5, 0.1, 30.0).a, 1.0);
        // Gray has no chroma to speak of.
        assert!(hex("#808080").to_oklch().1 < 0.01);
    }

    #[test]
    fn wcag_luminance_and_contrast() {
        assert!((Color::WHITE.luminance() - 1.0).abs() < 1e-5);
        assert!(Color::BLACK.luminance().abs() < 1e-6);
        assert!((Color::contrast_ratio(Color::BLACK, Color::WHITE) - 21.0).abs() < 1e-3);
        assert_eq!(Color::contrast_ratio(Color::WHITE, Color::WHITE), 1.0);
        // Known reference values.
        assert!((Color::contrast_ratio(hex("#777777"), Color::WHITE) - 4.48).abs() < 0.01);
        assert!((Color::contrast_ratio(hex("#767676"), Color::WHITE) - 4.54).abs() < 0.01);
        assert!((Color::contrast_ratio(hex("#0969da"), Color::WHITE) - 5.19).abs() < 0.01);
        let (a, b) = (hex("#1a1b26"), hex("#c0caf5"));
        assert_eq!(Color::contrast_ratio(a, b), Color::contrast_ratio(b, a));
    }

    #[test]
    fn readable_and_text_on() {
        let bg = hex("#1a1b26");
        let dim = hex("#3b4261");
        let fixed = dim.readable_on(bg, 4.5);
        assert!(Color::contrast_ratio(fixed, bg) >= 4.5);
        // Minimal move: not far past the target.
        assert!(Color::contrast_ratio(fixed, bg) < 4.7);
        assert!(fixed.luminance() > dim.luminance());
        // Already readable colors are untouched.
        assert_eq!(Color::WHITE.readable_on(bg, 4.5), Color::WHITE);
        // On light backgrounds it darkens.
        let pale = hex("#df8e1d").readable_on(hex("#eff1f5"), 4.5);
        assert!(Color::contrast_ratio(pale, hex("#eff1f5")) >= 4.5);
        assert!(pale.luminance() < hex("#df8e1d").luminance());
        assert_eq!(hex("#0969da").text_on(), Color::WHITE);
        assert_eq!(hex("#7aa2f7").text_on(), Color::BLACK);
        // Either black or white always reaches ~4.58:1.
        for s in ["#777777", "#7aa2f7", "#ff0000", "#00ff00", "#0000ff", "#808080"] {
            let c = hex(s);
            assert!(Color::contrast_ratio(c.text_on(), c) >= 4.5, "{s}");
        }
    }
}
