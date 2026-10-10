//! Hard text at full speed: tinted bands of CJK, emoji (ZWJ families, skin tones, flags), combining
//! marks and Zalgo, box drawing and blocks, Arabic, Hebrew and Thai, math and Powerline glyphs, each
//! scrolling sideways at its own speed, with emoji raining over them. It stresses the terminal's width
//! logic, grapheme clustering, font fallback, and how a wide glyph is cut or overwritten at a cell
//! boundary.
//!
//! Terminals disagree about the width of ZWJ sequences and flags, so the grid keeps one glyph (a whole
//! cluster) per cell and every wide or multi-codepoint cluster is followed by an explicit cursor move:
//! a terminal that counts differently misplaces that one glyph, never the rest of the row.

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::canvas::{self, Rgb, Rng, bg, fg, goto};
use crate::demo::{Demo, Frame};

const ZWJ: char = '\u{200D}';
const BLANK: u16 = 0;
/// The cell after a wide glyph: part of the glyph before it, never written out.
const CONT: u16 = 1;

fn is_ri(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}

fn is_tone(c: char) -> bool {
    ('\u{1F3FB}'..='\u{1F3FF}').contains(&c)
}

/// Columns a character takes, for the scripts this demo uses: 0 for marks and joiners, 2 for East
/// Asian wide and emoji, 1 for everything else.
fn char_width(c: char) -> u8 {
    let u = c as u32;
    match u {
        0x0300..=0x036F | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x0E31 | 0x0E34..=0x0E3A => 0,
        0x0E47..=0x0E4E | 0x200B..=0x200F | 0x20D0..=0x20FF | 0xFE00..=0xFE0F | 0x1F3FB..=0x1F3FF => 0,
        0x1100..=0x115F | 0x2E80..=0x303E | 0x3041..=0x33FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF => 2,
        0xA000..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE6F | 0xFF01..=0xFF60 => 2,
        0xFFE0..=0xFFE6 | 0x1F300..=0x1F64F | 0x1F680..=0x1F6FF | 0x1F900..=0x1F9FF => 2,
        0x1FA70..=0x1FAFF | 0x2728 | 0x26A1 | 0x2B50 => 2,
        _ => 1,
    }
}

/// Splits text into what a terminal that clusters graphemes shows in one place: a base, its marks, and
/// anything joined to it with a ZWJ, plus flags (two regional indicators).
fn clusters(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut prev, mut ri_single) = (0, None, false);
    for (i, c) in s.char_indices() {
        let cont = match prev {
            None => false,
            Some(p) => char_width(c) == 0 || p == ZWJ || (is_ri(c) && ri_single),
        };
        ri_single = is_ri(c) && !cont;
        if !cont && i > start {
            out.push(&s[start..i]);
            start = i;
        }
        prev = Some(c);
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn cluster_width(s: &str) -> u8 {
    let Some(first) = s.chars().next() else { return 1 };
    if is_ri(first) {
        return 2;
    }
    let joined = s.chars().any(|c| c == ZWJ || c == '\u{FE0F}' || is_tone(c));
    if joined { 2 } else { char_width(first).max(1) }
}

struct Glyph {
    text: String,
    width: u8,
    /// Wide or several codepoints: the terminal may disagree on its width, so the cursor is placed after.
    risky: bool,
}

struct Atlas {
    glyphs: Vec<Glyph>,
    ids: HashMap<String, u16>,
}

impl Atlas {
    fn new() -> Self {
        let g = |t: &str, width| Glyph { text: t.to_string(), width, risky: false };
        Self { glyphs: vec![g(" ", 1), g("", 0)], ids: HashMap::new() }
    }

    fn id(&mut self, s: &str) -> (u16, u8) {
        if let Some(&id) = self.ids.get(s) {
            return (id, self.glyphs[id as usize].width);
        }
        let width = cluster_width(s);
        let id = self.glyphs.len() as u16;
        let risky = width == 2 || s.chars().count() > 1;
        self.glyphs.push(Glyph { text: s.to_string(), width, risky });
        self.ids.insert(s.to_string(), id);
        (id, width)
    }
}

struct Grid {
    cols: usize,
    rows: usize,
    id: Vec<u16>,
    fg: Vec<Rgb>,
    bg: Vec<Rgb>,
}

impl Grid {
    fn fit(&mut self, cols: usize, rows: usize) -> bool {
        if (cols, rows) == (self.cols, self.rows) {
            return false;
        }
        (self.cols, self.rows) = (cols, rows);
        self.id = vec![BLANK; cols * rows];
        self.fg = vec![Rgb::WHITE; cols * rows];
        self.bg = vec![Rgb::BLACK; cols * rows];
        true
    }

    /// Frees a cell, and the other half of a wide glyph it belongs to, so a glyph is never left cut.
    fn clear_cell(&mut self, i: usize, col: usize) {
        if self.id[i] == CONT {
            if col > 0 {
                self.id[i - 1] = BLANK;
            }
        } else if col + 1 < self.cols && self.id[i + 1] == CONT {
            self.id[i + 1] = BLANK;
        }
    }

    /// Puts a glyph of width 1 or 2; one that doesn't fit becomes a blank. `back` None keeps the
    /// background that is there.
    fn put(&mut self, col: usize, row: usize, id: u16, width: u8, color: Rgb, back: Option<Rgb>) {
        if col >= self.cols || row >= self.rows {
            return;
        }
        let i = row * self.cols + col;
        let width = if width == 2 && col + 1 >= self.cols { 0 } else { width };
        self.clear_cell(i, col);
        self.id[i] = if width == 0 { BLANK } else { id };
        self.fg[i] = color;
        if let Some(b) = back {
            self.bg[i] = b;
        }
        if width == 2 {
            self.clear_cell(i + 1, col + 1);
            self.id[i + 1] = CONT;
            self.fg[i + 1] = color;
            if let Some(b) = back {
                self.bg[i + 1] = b;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Cjk,
    Emoji,
    Latin,
    Box,
    Rtl,
    Math,
    Rule,
}

const PATTERN: [Kind; 8] =
    [Kind::Cjk, Kind::Emoji, Kind::Latin, Kind::Box, Kind::Rtl, Kind::Math, Kind::Emoji, Kind::Rule];

const CJK: [&str; 3] = [
    "漢字仮名交じり文。吾輩は猫である。名前はまだ無い。どこで生れたかとんと見当がつかぬ。",
    "한국어 텍스트와 日本語 そして 中文字符 混合显示测试：你好，世界！",
    "ｶﾀｶﾅ ﾊﾝｶｸ halfwidth ｱｲｳｴｵ mixed ｶｷｸｹｺ with FULLWIDTH ＡＢＣＤＥ１２３",
];
const EMOJI: [&str; 3] = [
    "👩‍💻 🧑🏽‍🚀 🇯🇵 🇩🇪 🏳️‍🌈 👨‍👩‍👧‍👦 👍🏽 🚀 🔥 ✨ 🐙 🦀 🎉 🇺🇸 🇧🇷 🇰🇷",
    "🧑‍🔬 👩🏿‍🎨 🤖 🐧 🍣 🌈 🪐 ⚡ 🇫🇷 🇮🇳 👨🏻‍💻 🧙 🧑🏾‍🍳 🇨🇦 🇪🇸",
    "👨‍👩‍👧 🧑🏼‍🤝‍🧑🏿 🏴 🇬🇧 💎 🌸 🍜 🎮 👋🏽 🙌🏻 🫶 🥳 🇮🇹 🇦🇺",
];
const BOXES: [&str; 3] = [
    "╔═══╦═══╗║ ║ ║╠═══╬═══╣╚═══╩═══╝ ▀▄█▌▐ ░▒▓█▓▒░ ",
    "╭──╮│  │╰──╯ ┌┬┐├┼┤└┴┘ ▖▗▘▙▚▛▜▝▞▟ ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁ ",
    "⣿⣷⣯⣟⡿⢿⣻⣽⣾ ━┃┏┳┓┣╋┫┗┻┛ ╱╲╳ ◢◣◤◥ ▰▱▰▱ ◐◑◒◓ ",
];
const RTL: [&str; 3] = [
    "مَرْحَبًا بِالْعَالَم · اختبار النص العربي · السلام عليكم",
    "שלום עולם · בדיקת טקסט עברי · ברוכים הבאים",
    "สวัสดีชาวโลก · ภาษาไทยมีวรรณยุกต์ · ทดสอบข้อความ",
];
const MATH: [&str; 2] = [
    "∑ᵢ₌₁ⁿ i = n(n+1)/2 · ∫₀^∞ e^{-x²}dx = √π/2 · ∇×E = −∂B/∂t · ∀ε>0 ∃δ>0 · ℝ ⊂ ℂ · ⟨ψ|Ĥ|ψ⟩ ≥ E₀ · ∮ B·dl = μ₀I ",
    "α β γ δ ε ζ η θ λ μ π σ φ ψ ω Γ Δ Θ Λ Ξ Π Σ Φ Ψ Ω ⊕ ⊗ ∈ ∉ ⊂ ∪ ∩ ∧ ∨ ¬ ⇒ ⇔ ∴ ∵ ℵ ℏ ",
];
const LATIN: &str =
    "naïve café ñandú Ångström œuvre · e\u{301} a\u{300} n\u{303} o\u{308} u\u{30a} c\u{327} · ";
const PHRASE: &str =
    "the quick brown fox jumps over the lazy dog · pack my box with five dozen liquor jugs · ";
const MARKS: [char; 18] = [
    '\u{300}', '\u{301}', '\u{302}', '\u{303}', '\u{304}', '\u{306}', '\u{307}', '\u{308}', '\u{30a}',
    '\u{30b}', '\u{30c}', '\u{323}', '\u{324}', '\u{325}', '\u{327}', '\u{328}', '\u{330}', '\u{33d}',
];
const RAIN: [&str; 24] = [
    "🔥",
    "🚀",
    "✨",
    "🐙",
    "🦀",
    "🎉",
    "🌈",
    "🪐",
    "⚡",
    "🤖",
    "🐧",
    "🍣",
    "👩‍💻",
    "🧑🏽‍🚀",
    "👍🏽",
    "🇯🇵",
    "🇩🇪",
    "🇧🇷",
    "🏳️‍🌈",
    "👨‍👩‍👧‍👦",
    "💎",
    "🌸",
    "🍜",
    "🎮",
];

fn zalgo(rng: &mut Rng, s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        out.push(c);
        if c.is_ascii_alphabetic() && rng.f() < 0.45 {
            for _ in 0..1 + rng.below(3) {
                out.push(MARKS[rng.below(MARKS.len())]);
            }
        }
    }
    out
}

struct Item {
    id: u16,
    width: u8,
    color: Rgb,
}

/// A looping line of glyphs, longer than the screen, with where each one starts.
struct Strip {
    items: Vec<Item>,
    starts: Vec<usize>,
    total: usize,
}

struct Band {
    strip: Strip,
    pos: f64,
    /// Columns per second, negative to the left.
    speed: f64,
    tint: Rgb,
}

struct Rain {
    col: usize,
    head: f64,
    speed: f64,
    trail: Vec<(u16, u8)>,
}

pub struct Unicode {
    atlas: Atlas,
    grid: Grid,
    texts: Vec<(Kind, String)>,
    bands: Vec<Band>,
    drops: Vec<Rain>,
    rain: Vec<(u16, u8)>,
    vignette: Vec<f64>,
    rng: Rng,
}

impl Default for Unicode {
    fn default() -> Self {
        let mut rng = Rng::new(0x756e_6963_6f64);
        let mut texts: Vec<(Kind, String)> = Vec::new();
        texts.extend(CJK.iter().map(|s| (Kind::Cjk, format!("{s}   "))));
        texts.extend(EMOJI.iter().map(|s| (Kind::Emoji, format!("{s}   "))));
        texts.extend(BOXES.iter().map(|s| (Kind::Box, s.to_string())));
        texts.extend(RTL.iter().map(|s| (Kind::Rtl, format!("{s}   "))));
        texts.extend(MATH.iter().map(|s| (Kind::Math, s.to_string())));
        texts.push((Kind::Latin, LATIN.to_string()));
        texts.push((Kind::Latin, zalgo(&mut rng, PHRASE)));
        texts.push((Kind::Latin, zalgo(&mut rng, "ĦËĽĿǪ · ZALGO · W̷O̷R̷M̷H̷O̷L̷E̷ · ")));
        texts.push((Kind::Rule, "░▒▓█▓▒░ ".to_string()));
        let mut atlas = Atlas::new();
        let rain = RAIN.iter().map(|s| atlas.id(s)).collect();
        Self {
            atlas,
            grid: Grid { cols: 0, rows: 0, id: Vec::new(), fg: Vec::new(), bg: Vec::new() },
            texts,
            bands: Vec::new(),
            drops: Vec::new(),
            rain,
            vignette: Vec::new(),
            rng,
        }
    }
}

fn tint(kind: Kind) -> Rgb {
    match kind {
        Kind::Cjk => Rgb(44, 12, 26),
        Kind::Emoji => Rgb(14, 18, 40),
        Kind::Latin => Rgb(10, 34, 34),
        Kind::Box => Rgb(12, 12, 20),
        Kind::Rtl => Rgb(38, 26, 8),
        Kind::Math => Rgb(26, 14, 44),
        Kind::Rule => Rgb(5, 5, 9),
    }
}

/// A glyph's color from its place in one tile of the text, in whole cycles so the loop has no seam.
fn color(kind: Kind, i: usize, n: usize, row: usize) -> Rgb {
    let p = i as f64 / n.max(1) as f64;
    let wave = (p * 2.0 * PI * 2.0 + row as f64).sin();
    match kind {
        Kind::Cjk => Rgb::hsv(0.95 + 0.07 * wave, 0.38, 1.0),
        Kind::Latin => Rgb::hsv(0.5 + 0.08 * wave, 0.5, 1.0),
        Kind::Box => Rgb::hsv(p + row as f64 * 0.13, 0.7, 0.95),
        Kind::Rtl => Rgb::hsv(0.1 + 0.06 * wave, 0.55, 1.0),
        Kind::Math => Rgb::hsv(0.75 + 0.1 * wave, 0.4, 1.0),
        Kind::Rule => Rgb::hsv(0.62 + 0.1 * wave, 0.5, 0.35),
        Kind::Emoji => Rgb::WHITE,
    }
}

fn build_strip(atlas: &mut Atlas, text: &str, kind: Kind, row: usize, min_total: usize) -> Strip {
    let tile: Vec<(u16, u8)> = clusters(text).into_iter().map(|c| atlas.id(c)).collect();
    let n = tile.len();
    let tile_width: usize = tile.iter().map(|t| t.1 as usize).sum::<usize>().max(1);
    let (mut items, mut starts, mut total) = (Vec::new(), Vec::new(), 0);
    for _ in 0..min_total.div_ceil(tile_width).max(1) {
        for (i, &(id, width)) in tile.iter().enumerate() {
            starts.push(total);
            items.push(Item { id, width, color: color(kind, i, n, row) });
            total += width as usize;
        }
    }
    Strip { items, starts, total }
}

impl Unicode {
    fn rebuild(&mut self) {
        let (cols, rows) = (self.grid.cols, self.grid.rows);
        let (first, last) = if rows >= 4 { (1, rows - 1) } else { (0, rows) };
        self.vignette = (0..cols)
            .map(|x| ((0.55 + 0.45 * (PI * (x as f64 + 0.5) / cols as f64).sin()) * 6.0).round() / 6.0)
            .collect();
        self.bands.clear();
        for row in first..last {
            let i = row - first;
            let kind = PATTERN[i % PATTERN.len()];
            let pool: Vec<&String> = self.texts.iter().filter(|t| t.0 == kind).map(|t| &t.1).collect();
            let text = pool[(i / PATTERN.len() + i % 3) % pool.len()].clone();
            let strip = build_strip(&mut self.atlas, &text, kind, row, cols + 8);
            let speed = if kind == Kind::Rule { 30.0 } else { 3.0 + self.rng.f() * 17.0 };
            let dir = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
            let pos = self.rng.f() * strip.total as f64;
            self.bands.push(Band { strip, pos, speed: speed * dir, tint: tint(kind) });
        }
        let n = (cols / 7).clamp(2, 70);
        self.drops = (0..n).map(|_| self.drop(true)).collect();
    }

    fn drop(&mut self, fresh: bool) -> Rain {
        let rows = self.grid.rows as f64;
        let len = 1 + self.rng.below(3);
        let trail = (0..len).map(|_| self.rain[self.rng.below(self.rain.len())]).collect();
        let head = if fresh { self.rng.range(-3.0, rows) } else { -self.rng.range(0.0, rows * 0.6) };
        Rain {
            col: self.rng.below(self.grid.cols.saturating_sub(1).max(1)),
            head,
            speed: self.rng.range(3.0, 11.0),
            trail,
        }
    }

    fn draw_band(&mut self, row: usize, b: usize) {
        let band = &self.bands[b];
        let strip = &band.strip;
        let cols = self.grid.cols;
        for x in 0..cols {
            self.grid.bg[row * cols + x] = band.tint.scale(self.vignette[x]);
        }
        let offset = (band.pos as i64).rem_euclid(strip.total.max(1) as i64) as usize;
        let mut k = strip.starts.partition_point(|&s| s <= offset).saturating_sub(1);
        let mut x = strip.starts[k] as isize - offset as isize;
        while x < cols as isize && !strip.items.is_empty() {
            let item = &strip.items[k % strip.items.len()];
            let w = item.width as isize;
            if x >= 0 {
                self.grid.put(x as usize, row, item.id, item.width, item.color, None);
            } else if x + w > 0 {
                // A wide glyph cut by the left edge: its second half has nothing to show.
                self.grid.put(0, row, BLANK, 1, item.color, None);
            }
            x += w;
            k += 1;
        }
    }

    fn put_str(&mut self, mut col: usize, row: usize, s: &str, color: Rgb, back: Rgb) -> usize {
        for c in clusters(s) {
            let (id, w) = self.atlas.id(c);
            self.grid.put(col, row, id, w, color, Some(back));
            col += w as usize;
        }
        col
    }

    /// A Powerline bar: segments with arrows between them, from the left and from the right.
    fn powerline(
        &mut self,
        row: usize,
        left: &[(String, Rgb, Rgb)],
        right: &[(String, Rgb, Rgb)],
        fill: Rgb,
    ) {
        let cols = self.grid.cols;
        for x in 0..cols {
            self.grid.put(x, row, BLANK, 1, Rgb::WHITE, Some(fill));
        }
        let width = |s: &str| clusters(s).iter().map(|c| cluster_width(c) as usize).sum::<usize>() + 1;
        let (lw, rw): (usize, usize) =
            (left.iter().map(|s| width(&s.0)).sum(), right.iter().map(|s| width(&s.0)).sum());
        let mut x = 0;
        for (k, (text, f, b)) in left.iter().enumerate() {
            x = self.put_str(x, row, text, *f, *b);
            let next = left.get(k + 1).map_or(fill, |s| s.2);
            x = self.put_str(x, row, "\u{e0b0}", *b, next);
        }
        if lw + rw <= cols {
            let mut x = cols - rw;
            for (k, (text, f, b)) in right.iter().enumerate() {
                let prev = if k == 0 { fill } else { right[k - 1].2 };
                x = self.put_str(x, row, "\u{e0b2}", *b, prev);
                x = self.put_str(x, row, text, *f, *b);
            }
        }
    }

    fn render(&self, out: &mut Vec<u8>) {
        let g = &self.grid;
        let (mut cur_fg, mut cur_bg): (Option<Rgb>, Option<Rgb>) = (None, None);
        for row in 0..g.rows {
            goto(out, 0, row as u16);
            for col in 0..g.cols {
                let i = row * g.cols + col;
                if g.id[i] == CONT {
                    continue;
                }
                let glyph = &self.atlas.glyphs[g.id[i] as usize];
                if cur_bg != Some(g.bg[i]) {
                    bg(out, g.bg[i]);
                    cur_bg = Some(g.bg[i]);
                }
                if g.id[i] != BLANK && cur_fg != Some(g.fg[i]) {
                    fg(out, g.fg[i]);
                    cur_fg = Some(g.fg[i]);
                }
                out.extend_from_slice(glyph.text.as_bytes());
                if glyph.risky && col + (glyph.width as usize) < g.cols {
                    goto(out, (col + glyph.width as usize) as u16, row as u16);
                }
            }
        }
        canvas::reset(out);
    }
}

impl Demo for Unicode {
    fn frame(&mut self, f: &mut Frame) {
        if self.grid.fit(f.size.cols as usize, f.size.rows as usize) {
            self.rebuild();
        }
        let (cols, rows) = (self.grid.cols, self.grid.rows);
        if cols == 0 || rows == 0 {
            return;
        }
        let first = if rows >= 4 { 1 } else { 0 };
        for b in 0..self.bands.len() {
            let band = &mut self.bands[b];
            band.pos += band.speed * f.dt;
            self.draw_band(first + b, b);
        }

        if rows >= 4 {
            let (dark, hi, mid, low) = (Rgb(18, 22, 34), Rgb(97, 175, 239), Rgb(62, 68, 92), Rgb(44, 48, 66));
            let (ink, soft) = (Rgb(18, 22, 34), Rgb(215, 220, 235));
            let tabs = [
                (" 1  unicode.rs ".to_string(), ink, hi),
                (" 2  life.rs ".to_string(), soft, mid),
                (" 3  dashboard.rs ".to_string(), soft, low),
            ];
            self.powerline(0, &tabs, &[], dark);

            let s = f.t as u64;
            let left = [
                (" NORMAL ".to_string(), ink, Rgb(130, 200, 120)),
                (" \u{e0a0} main ".to_string(), soft, mid),
                (" ✓ 12 · +340 -21 ".to_string(), soft, low),
            ];
            let right = [
                (" UTF-8 ".to_string(), soft, low),
                (" 日本語 ".to_string(), soft, mid),
                (format!(" {:>2}% ", 40 + (f.t * 1.3).sin().mul_add(30.0, 30.0) as u32), soft, low),
                (format!(" \u{e0a1} {:>3}:{:<2} ", 1 + s * 7 % 999, 1 + s * 3 % 80), ink, Rgb(130, 200, 120)),
            ];
            self.powerline(rows - 1, &left, &right, dark);
        }

        for d in 0..self.drops.len() {
            let drop = &mut self.drops[d];
            drop.head += drop.speed * f.dt;
            if drop.head - drop.trail.len() as f64 > rows as f64 {
                self.drops[d] = self.drop(false);
                continue;
            }
            let (col, head) = (drop.col, drop.head.floor() as isize);
            for (k, &(id, w)) in drop.trail.iter().enumerate() {
                let r = head - k as isize;
                if r >= 0 && (r as usize) < rows {
                    self.grid.put(col, r as usize, id, w, Rgb::WHITE, None);
                }
            }
        }
        self.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    fn widths(s: &str) -> Vec<u8> {
        clusters(s).iter().map(|c| cluster_width(c)).collect()
    }

    #[test]
    fn clusters_keep_sequences_whole() {
        assert_eq!(clusters("👩‍💻a"), ["👩‍💻", "a"]);
        assert_eq!(clusters("🇯🇵🇩🇪"), ["🇯🇵", "🇩🇪"]);
        assert_eq!(clusters("e\u{301}x"), ["e\u{301}", "x"]);
        assert_eq!(clusters("👍🏽!"), ["👍🏽", "!"]);
        assert_eq!(widths("👩‍💻 🇯🇵 e\u{301} 日"), [2, 1, 2, 1, 1, 1, 2]);
    }

    #[test]
    fn a_wide_glyph_overwritten_in_half_leaves_no_orphan() {
        let mut g =
            Grid { cols: 6, rows: 1, id: vec![BLANK; 6], fg: vec![Rgb::WHITE; 6], bg: vec![Rgb::BLACK; 6] };
        g.put(1, 0, 5, 2, Rgb::WHITE, None);
        g.put(2, 0, 7, 1, Rgb::WHITE, None);
        assert_eq!(g.id[..4], [BLANK, BLANK, 7, BLANK]);
        g.put(0, 0, 5, 2, Rgb::WHITE, None);
        g.put(1, 0, 5, 2, Rgb::WHITE, None);
        assert_eq!(g.id[..3], [BLANK, 5, CONT]);
        g.put(5, 0, 5, 2, Rgb::WHITE, None);
        assert_eq!(g.id[5], BLANK);
    }

    #[test]
    fn draws_at_any_size() {
        for (c, r) in [(0, 0), (1, 1), (2, 3), (20, 5), (80, 24), (300, 80)] {
            let mut d = Unicode::default();
            let mut out = Vec::new();
            for n in 0..120u64 {
                out.clear();
                let size = Size { cols: c, rows: r, px_w: 0, px_h: 0 };
                d.frame(&mut Frame { size, t: n as f64 / 30.0, dt: 1.0 / 30.0, out: &mut out });
            }
            assert!(c == 0 || r == 0 || !out.is_empty());
        }
    }
}
