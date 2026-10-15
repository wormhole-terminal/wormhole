//! Color specifications (OSC 4, 10, 11, 12) and the palette queries answer with.
//!
//! Spec: xterm's ctlseqs, "OSC Ps ; Pt": the color is `rgb:<r>/<g>/<b>` with 1 to 4 hex digits per
//! component (X11's color strings, X(7)), or `#` and 3, 6, 9 or 12 hex digits. In `rgb:` a
//! component with fewer than 4 digits is scaled to its range (`f` is full, `ff` is full); in `#`
//! the digits are the most significant bits (`#3a7` is `#3000a0007000`). A component is kept as
//! its top byte, and replies use 4 digits, the byte twice. Color names (`red`) and the
//! device-independent forms (`rgbi:`, `CIEXYZ:`, `CIELab:`, `TekHVC:`, …) are not read: they depend
//! on the display's color characterization, and a program that wants a color can send its numbers.

use crate::types::{COLOR_COUNT, Rgb};

/// xterm's default palette (the 16 named colors, the 6x6x6 cube, the 24 grays) and light gray on
/// black for the foreground, background and cursor.
pub(super) fn xterm_defaults() -> [Rgb; COLOR_COUNT] {
    const NAMED: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00),
        (0xcd, 0x00, 0x00),
        (0x00, 0xcd, 0x00),
        (0xcd, 0xcd, 0x00),
        (0x00, 0x00, 0xee),
        (0xcd, 0x00, 0xcd),
        (0x00, 0xcd, 0xcd),
        (0xe5, 0xe5, 0xe5),
        (0x7f, 0x7f, 0x7f),
        (0xff, 0x00, 0x00),
        (0x00, 0xff, 0x00),
        (0xff, 0xff, 0x00),
        (0x5c, 0x5c, 0xff),
        (0xff, 0x00, 0xff),
        (0x00, 0xff, 0xff),
        (0xff, 0xff, 0xff),
    ];
    let mut palette = [Rgb::default(); COLOR_COUNT];
    for (i, slot) in palette.iter_mut().enumerate().take(256) {
        let (r, g, b) = match i {
            0..=15 => NAMED[i],
            16..=231 => {
                let n = (i - 16) as u8;
                let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
                (level(n / 36), level(n / 6 % 6), level(n % 6))
            }
            _ => {
                let v = 8 + 10 * (i - 232) as u8;
                (v, v, v)
            }
        };
        *slot = Rgb { r, g, b };
    }
    palette[256] = Rgb { r: 0xe5, g: 0xe5, b: 0xe5 };
    palette[257] = Rgb { r: 0, g: 0, b: 0 };
    palette[258] = Rgb { r: 0xe5, g: 0xe5, b: 0xe5 };
    palette
}

/// Hex digits `text` as a number and how many digits it had (1..=4).
fn hex(text: &[u8]) -> Option<(u32, u32)> {
    if text.is_empty() || text.len() > 4 {
        return None;
    }
    let mut value = 0u32;
    for &b in text {
        value = value << 4 | char::from(b).to_digit(16)?;
    }
    Some((value, text.len() as u32))
}

/// A component of `digits` hex digits as the top byte of the 16-bit value X11 scales it to
/// (`rgb:` specs: `f`, `ff`, `fff` and `ffff` are all full).
fn scale(value: u32, digits: u32) -> u8 {
    let max = (1u32 << (4 * digits)) - 1;
    ((value * 0xFFFF / max) >> 8) as u8
}

/// A component of a `#` spec: its digits are the top bits of the value, the rest zero (X11:
/// `#3a7` is `#3000a0007000`).
fn left_aligned(value: u32, digits: u32) -> u8 {
    ((value << (16 - 4 * digits)) >> 8) as u8
}

/// The color a specification names, if it is one of the forms above.
pub(super) fn parse_color(spec: &[u8]) -> Option<Rgb> {
    if let Some(rest) = spec.strip_prefix(b"rgb:") {
        let mut parts = rest.split(|&b| b == b'/');
        let (r, g, b) = (hex(parts.next()?)?, hex(parts.next()?)?, hex(parts.next()?)?);
        if parts.next().is_some() {
            return None;
        }
        return Some(Rgb { r: scale(r.0, r.1), g: scale(g.0, g.1), b: scale(b.0, b.1) });
    }
    let digits = spec.strip_prefix(b"#")?;
    if digits.is_empty() || digits.len() % 3 != 0 || digits.len() > 12 {
        return None;
    }
    let each = digits.len() / 3;
    let (r, g, b) = (hex(&digits[..each])?, hex(&digits[each..2 * each])?, hex(&digits[2 * each..])?);
    Some(Rgb { r: left_aligned(r.0, r.1), g: left_aligned(g.0, g.1), b: left_aligned(b.0, b.1) })
}

/// `rgb:rrrr/gggg/bbbb`, each 8-bit component repeated to 16 bits as xterm does, for a reply.
pub(super) struct ColorSpec(pub(super) Rgb);

impl std::fmt::Display for ColorSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Rgb { r, g, b } = self.0;
        write!(f, "rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}")
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorSpec, parse_color, xterm_defaults};
    use crate::types::Rgb;

    fn rgb(r: u8, g: u8, b: u8) -> Option<Rgb> {
        Some(Rgb { r, g, b })
    }

    #[test]
    fn every_form_of_a_color_spec() {
        assert_eq!(parse_color(b"rgb:ff/00/80"), rgb(255, 0, 128));
        assert_eq!(parse_color(b"rgb:f/0/8"), rgb(255, 0, 136));
        assert_eq!(parse_color(b"rgb:ffff/0000/8080"), rgb(255, 0, 128));
        assert_eq!(parse_color(b"#ff0080"), rgb(255, 0, 128));
        assert_eq!(parse_color(b"#f08"), rgb(0xf0, 0, 0x80));
        assert_eq!(parse_color(b"#fff000800"), rgb(255, 0, 128));
        assert_eq!(parse_color(b"#ffff00008080"), rgb(255, 0, 128));
        assert_eq!(parse_color(b"rgb:ff/00"), None);
        assert_eq!(parse_color(b"rgb:ff/00/80/00"), None);
        assert_eq!(parse_color(b"#ff00"), None);
        assert_eq!(parse_color(b"red"), None);
        assert_eq!(parse_color(b"rgb:gg/00/00"), None);
    }

    #[test]
    fn replies_repeat_the_byte() {
        assert_eq!(ColorSpec(Rgb { r: 0x1e, g: 0xff, b: 0 }).to_string(), "rgb:1e1e/ffff/0000");
    }

    #[test]
    fn the_default_palette_is_xterms() {
        let palette = xterm_defaults();
        assert_eq!(palette[196], Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(palette[232], Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(palette[1], Rgb { r: 0xcd, g: 0, b: 0 });
    }
}
