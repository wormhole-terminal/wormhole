//! Drawing surfaces that turn into escape sequences: pixels as half blocks (two per cell), dots as
//! braille (eight per cell), and plain cells. Each writes only the color changes it needs, so the
//! bytes per frame measure the picture, not the encoder.

use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const BLACK: Rgb = Rgb(0, 0, 0);
    pub const WHITE: Rgb = Rgb(255, 255, 255);

    /// From components in 0..=1, clamped.
    pub fn f(r: f64, g: f64, b: f64) -> Rgb {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        Rgb(c(r), c(g), c(b))
    }

    pub fn lerp(self, other: Rgb, t: f64) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t + 0.5) as u8;
        Rgb(m(self.0, other.0), m(self.1, other.1), m(self.2, other.2))
    }

    pub fn scale(self, k: f64) -> Rgb {
        let k = k.max(0.0);
        let m = |a: u8| (a as f64 * k).min(255.0) as u8;
        Rgb(m(self.0), m(self.1), m(self.2))
    }

    pub fn add(self, o: Rgb) -> Rgb {
        Rgb(self.0.saturating_add(o.0), self.1.saturating_add(o.1), self.2.saturating_add(o.2))
    }

    /// Hue, saturation and value, each in 0..=1 (hue wraps).
    pub fn hsv(h: f64, s: f64, v: f64) -> Rgb {
        let h = h.rem_euclid(1.0) * 6.0;
        let i = h.floor();
        let f = h - i;
        let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
        let (r, g, b) = match i as u8 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Rgb::f(r, g, b)
    }

    /// Inigo Quilez's cosine palette: `a + b·cos(2π(c·t + d))`, per channel.
    pub fn cosine(t: f64, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) -> Rgb {
        let ch = |i: usize| a[i] + b[i] * (TAU * (c[i] * t + d[i])).cos();
        Rgb::f(ch(0), ch(1), ch(2))
    }
}

/// Appends a number in decimal, without going through `fmt`.
#[inline]
pub fn push_num(out: &mut Vec<u8>, mut n: u32) {
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&buf[i..]);
}

/// Moves the cursor to a 0-based column and row.
#[inline]
pub fn goto(out: &mut Vec<u8>, col: u16, row: u16) {
    out.extend_from_slice(b"\x1b[");
    push_num(out, row as u32 + 1);
    out.push(b';');
    push_num(out, col as u32 + 1);
    out.push(b'H');
}

#[inline]
pub fn fg(out: &mut Vec<u8>, c: Rgb) {
    out.extend_from_slice(b"\x1b[38;2;");
    rgb_args(out, c);
}

#[inline]
pub fn bg(out: &mut Vec<u8>, c: Rgb) {
    out.extend_from_slice(b"\x1b[48;2;");
    rgb_args(out, c);
}

#[inline]
fn rgb_args(out: &mut Vec<u8>, c: Rgb) {
    push_num(out, c.0 as u32);
    out.push(b';');
    push_num(out, c.1 as u32);
    out.push(b';');
    push_num(out, c.2 as u32);
    out.push(b'm');
}

pub fn reset(out: &mut Vec<u8>) {
    out.extend_from_slice(b"\x1b[0m");
}

/// Pixels drawn as `▀`: the cell's foreground is the upper pixel, its background the lower, so a
/// terminal of `cols × rows` is a picture of `cols × 2·rows`.
pub struct Pixels {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
}

impl Pixels {
    pub fn new() -> Self {
        Self { w: 0, h: 0, px: Vec::new() }
    }

    /// Fits the picture to the terminal; returns whether the size changed (the contents are then
    /// black).
    pub fn fit(&mut self, cols: u16, rows: u16) -> bool {
        let (w, h) = (cols as usize, rows as usize * 2);
        if (w, h) == (self.w, self.h) {
            return false;
        }
        self.w = w;
        self.h = h;
        self.px = vec![Rgb::BLACK; w * h];
        true
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, c: Rgb) {
        if x < self.w && y < self.h {
            self.px[y * self.w + x] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Rgb {
        self.px[y * self.w + x]
    }

    /// Runs `f(x, y)` for every pixel, `x` and `y` in 0..1 across the width and height (`y` grows
    /// down), with the aspect ratio in the third argument (`w / h`), so shapes stay round.
    pub fn shade(&mut self, mut f: impl FnMut(f64, f64, f64) -> Rgb) {
        let (w, h) = (self.w, self.h);
        let aspect = w as f64 / h.max(1) as f64;
        for y in 0..h {
            let v = (y as f64 + 0.5) / h as f64;
            for x in 0..w {
                let u = (x as f64 + 0.5) / w as f64;
                self.px[y * w + x] = f(u, v, aspect);
            }
        }
    }

    /// The escape sequences for the whole picture.
    pub fn render(&self, out: &mut Vec<u8>) {
        let mut cur_fg: Option<Rgb> = None;
        let mut cur_bg: Option<Rgb> = None;
        for row in 0..self.h / 2 {
            goto(out, 0, row as u16);
            let top = &self.px[row * 2 * self.w..(row * 2 + 1) * self.w];
            let bottom = &self.px[(row * 2 + 1) * self.w..(row * 2 + 2) * self.w];
            for x in 0..self.w {
                let (t, b) = (top[x], bottom[x]);
                if t == b {
                    // A space needs only the background; when the foreground already matches, a
                    // full block needs nothing at all.
                    if cur_fg == Some(t) && cur_bg != Some(t) {
                        out.extend_from_slice("█".as_bytes());
                        continue;
                    }
                    if cur_bg != Some(t) {
                        bg(out, t);
                        cur_bg = Some(t);
                    }
                    out.push(b' ');
                    continue;
                }
                if cur_fg != Some(t) {
                    fg(out, t);
                    cur_fg = Some(t);
                }
                if cur_bg != Some(b) {
                    bg(out, b);
                    cur_bg = Some(b);
                }
                out.extend_from_slice("▀".as_bytes());
            }
        }
        reset(out);
    }
}

impl Default for Pixels {
    fn default() -> Self {
        Self::new()
    }
}

/// Dots drawn as braille: each cell is 2 × 4 dots, colored by the brightest dot that lands in it.
pub struct Braille {
    pub w: usize,
    pub h: usize,
    cols: usize,
    rows: usize,
    bits: Vec<u8>,
    color: Vec<Rgb>,
    weight: Vec<u16>,
}

impl Braille {
    pub fn new() -> Self {
        Self { w: 0, h: 0, cols: 0, rows: 0, bits: Vec::new(), color: Vec::new(), weight: Vec::new() }
    }

    /// Fits to the terminal and clears; returns whether the size changed.
    pub fn fit(&mut self, cols: u16, rows: u16) -> bool {
        let changed = (cols as usize, rows as usize) != (self.cols, self.rows);
        self.cols = cols as usize;
        self.rows = rows as usize;
        self.w = self.cols * 2;
        self.h = self.rows * 4;
        let n = self.cols * self.rows;
        self.bits.clear();
        self.bits.resize(n, 0);
        self.color.clear();
        self.color.resize(n, Rgb::BLACK);
        self.weight.clear();
        self.weight.resize(n, 0);
        changed
    }

    #[inline]
    pub fn dot(&mut self, x: isize, y: isize, c: Rgb) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        const BIT: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
        let i = (y / 4) * self.cols + x / 2;
        self.bits[i] |= BIT[x % 2][y % 4];
        let lum = c.0 as u16 + c.1 as u16 + c.2 as u16;
        if lum >= self.weight[i] {
            self.weight[i] = lum;
            self.color[i] = c;
        }
    }

    /// A line between two dots (Bresenham).
    pub fn line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, c: Rgb) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        for _ in 0..4096 {
            self.dot(x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// The escape sequences for every cell, on a background color.
    pub fn render(&self, out: &mut Vec<u8>, background: Rgb) {
        bg(out, background);
        let mut cur: Option<Rgb> = None;
        for row in 0..self.rows {
            goto(out, 0, row as u16);
            for col in 0..self.cols {
                let i = row * self.cols + col;
                let bits = self.bits[i];
                if bits == 0 {
                    out.push(b' ');
                    continue;
                }
                if cur != Some(self.color[i]) {
                    fg(out, self.color[i]);
                    cur = Some(self.color[i]);
                }
                let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
                let mut b = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
            }
        }
        reset(out);
    }
}

impl Default for Braille {
    fn default() -> Self {
        Self::new()
    }
}

/// A grid of characters with their own colors, for text demos (rain, dashboards, unicode).
#[derive(Clone)]
pub struct Cells {
    pub cols: usize,
    pub rows: usize,
    pub ch: Vec<char>,
    pub fg: Vec<Rgb>,
    pub bg: Vec<Rgb>,
}

impl Cells {
    pub fn new() -> Self {
        Self { cols: 0, rows: 0, ch: Vec::new(), fg: Vec::new(), bg: Vec::new() }
    }

    pub fn fit(&mut self, cols: u16, rows: u16) -> bool {
        let changed = (cols as usize, rows as usize) != (self.cols, self.rows);
        if changed {
            self.cols = cols as usize;
            self.rows = rows as usize;
            let n = self.cols * self.rows;
            self.ch = vec![' '; n];
            self.fg = vec![Rgb::WHITE; n];
            self.bg = vec![Rgb::BLACK; n];
        }
        changed
    }

    pub fn clear(&mut self, background: Rgb) {
        self.ch.fill(' ');
        self.fg.fill(Rgb::WHITE);
        self.bg.fill(background);
    }

    #[inline]
    pub fn put(&mut self, col: usize, row: usize, ch: char, fg: Rgb, bg: Rgb) {
        if col < self.cols && row < self.rows {
            let i = row * self.cols + col;
            self.ch[i] = ch;
            self.fg[i] = fg;
            self.bg[i] = bg;
        }
    }

    pub fn render(&self, out: &mut Vec<u8>) {
        let mut cur_fg: Option<Rgb> = None;
        let mut cur_bg: Option<Rgb> = None;
        let mut b = [0u8; 4];
        for row in 0..self.rows {
            goto(out, 0, row as u16);
            for col in 0..self.cols {
                let i = row * self.cols + col;
                let ch = self.ch[i];
                if ch == '\0' {
                    continue;
                }
                if cur_bg != Some(self.bg[i]) {
                    bg(out, self.bg[i]);
                    cur_bg = Some(self.bg[i]);
                }
                if ch != ' ' && cur_fg != Some(self.fg[i]) {
                    fg(out, self.fg[i]);
                    cur_fg = Some(self.fg[i]);
                }
                out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
            }
        }
        reset(out);
    }
}

impl Default for Cells {
    fn default() -> Self {
        Self::new()
    }
}

/// A small, fast, seeded random number generator (xorshift64*), so runs can be repeated.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// In 0..1.
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// In 0..n (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_in_decimal() {
        let mut out = Vec::new();
        push_num(&mut out, 0);
        out.push(b' ');
        push_num(&mut out, 4096);
        assert_eq!(out, b"0 4096");
    }

    #[test]
    fn half_blocks_only_repeat_colors_when_they_change() {
        let mut p = Pixels::new();
        p.fit(3, 1);
        p.px.fill(Rgb(1, 2, 3));
        let mut out = Vec::new();
        p.render(&mut out);
        assert_eq!(out, b"\x1b[1;1H\x1b[48;2;1;2;3m   \x1b[0m");
    }

    #[test]
    fn braille_sets_the_right_dots() {
        let mut b = Braille::new();
        b.fit(1, 1);
        b.dot(0, 0, Rgb::WHITE);
        b.dot(1, 3, Rgb::WHITE);
        let mut out = Vec::new();
        b.render(&mut out, Rgb::BLACK);
        assert!(String::from_utf8_lossy(&out).contains('\u{2881}'));
    }
}
