//! The pieces the result screens are drawn with: a dark panel (so a screenshot looks the same in
//! any theme), lines that know their width, and flat bars in one accent with eighth-cell precision.

use crate::canvas::{self, Rgb};

pub const BG: Rgb = Rgb(14, 16, 26);
pub const TRACK: Rgb = Rgb(33, 37, 54);
pub const TEXT: Rgb = Rgb(228, 232, 245);
pub const DIM: Rgb = Rgb(124, 134, 164);
pub const FAINT: Rgb = Rgb(58, 66, 92);
pub const GOOD: Rgb = Rgb(110, 231, 160);
pub const BAD: Rgb = Rgb(251, 113, 133);
/// The one color the bars and the title are drawn in: amber, so it is neither GOOD nor BAD.
pub const ACCENT: Rgb = Rgb(229, 170, 72);

/// Left eighth blocks: index n fills n eighths of a cell.
const PARTIAL: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// How many eighths of each cell a bar of `frac` (0..=1) of `width` cells fills. Anything above
/// zero shows at least one eighth, so a tiny result is still visible.
pub fn eighths(frac: f64, width: usize) -> Vec<u8> {
    let total = (frac.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let total = if frac > 0.0 { total.max(1) } else { total };
    (0..width).map(|i| total.saturating_sub(i * 8).min(8) as u8).collect()
}

/// The first `max` characters of `s`, with an ellipsis when it was cut.
pub fn fit(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
    if max > 0 {
        t.push('…');
    }
    t
}

/// One row of a panel: escape sequences and the number of cells they take.
#[derive(Default)]
pub struct Line {
    buf: Vec<u8>,
    w: usize,
}

impl Line {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width(&self) -> usize {
        self.w
    }

    pub fn text(&mut self, s: &str, c: Rgb) -> &mut Self {
        canvas::fg(&mut self.buf, c);
        self.buf.extend_from_slice(s.as_bytes());
        self.w += s.chars().count();
        self
    }

    pub fn bold(&mut self, s: &str, c: Rgb) -> &mut Self {
        self.buf.extend_from_slice(b"\x1b[1m");
        self.text(s, c);
        self.buf.extend_from_slice(b"\x1b[22m");
        self
    }

    /// The screen's title: bold, in the accent.
    pub fn title(&mut self, s: &str) -> &mut Self {
        self.bold(s, ACCENT)
    }

    pub fn pad(&mut self, n: usize) -> &mut Self {
        self.buf.extend(std::iter::repeat_n(b' ', n));
        self.w += n;
        self
    }

    /// Spaces up to column `col` (nothing if already past it).
    pub fn pad_to(&mut self, col: usize) -> &mut Self {
        self.pad(col.saturating_sub(self.w))
    }

    /// `s` ending at column `end`.
    pub fn right(&mut self, s: &str, c: Rgb, end: usize) -> &mut Self {
        self.pad_to(end.saturating_sub(s.chars().count()));
        self.text(s, c)
    }

    /// A flat bar of `frac` of `width` cells on a dim track; `dim` (0..=1) darkens it for the losers.
    pub fn bar(&mut self, frac: f64, width: usize, dim: f64) -> &mut Self {
        canvas::bg(&mut self.buf, TRACK);
        canvas::fg(&mut self.buf, ACCENT.scale(dim));
        for e in eighths(frac, width) {
            let mut b = [0u8; 4];
            self.buf.extend_from_slice(PARTIAL[e as usize].encode_utf8(&mut b).as_bytes());
        }
        canvas::bg(&mut self.buf, BG);
        self.w += width;
        self
    }
}

pub struct Panel {
    pub width: usize,
    lines: Vec<Line>,
}

impl Panel {
    pub fn new(width: usize) -> Self {
        Self { width, lines: Vec::new() }
    }

    pub fn push(&mut self, line: Line) {
        self.lines.push(line);
    }

    pub fn blank(&mut self) {
        self.lines.push(Line::new());
    }

    /// A horizontal rule with a margin of `margin` cells on each side.
    pub fn rule(&mut self, margin: usize) {
        let mut l = Line::new();
        l.pad(margin).text(&"─".repeat(self.width.saturating_sub(margin * 2)), FAINT);
        self.lines.push(l);
    }

    #[cfg(test)]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Every row filled to the panel's width, ending in a reset and a newline.
    pub fn write(&self, out: &mut Vec<u8>) {
        for line in &self.lines {
            canvas::bg(out, BG);
            out.extend_from_slice(&line.buf);
            out.extend(std::iter::repeat_n(b' ', self.width.saturating_sub(line.w)));
            canvas::reset(out);
            out.push(b'\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eighths_fill_whole_cells_then_a_part() {
        assert_eq!(eighths(0.5, 4), vec![8, 8, 0, 0]);
        assert_eq!(eighths(0.4375, 4), vec![8, 6, 0, 0]);
        assert_eq!(eighths(1.0, 3), vec![8, 8, 8]);
        assert_eq!(eighths(2.0, 2), vec![8, 8]);
        assert_eq!(eighths(0.0, 3), vec![0, 0, 0]);
        assert_eq!(eighths(0.0001, 10)[0], 1);
        assert!(eighths(0.5, 0).is_empty());
    }

    #[test]
    fn bars_are_one_flat_color() {
        let mut l = Line::new();
        l.bar(1.0, 8, 1.0);
        let text = String::from_utf8_lossy(&l.buf);
        assert_eq!(text.matches("\x1b[38;2;").count(), 1, "{text:?}");
        assert!(text.contains("229;170;72"), "{text:?}");
    }

    #[test]
    fn lines_count_their_cells() {
        let mut l = Line::new();
        l.pad(2).text("héllo", TEXT).bold("ab", DIM).bar(0.5, 10, 1.0).title("xyz");
        assert_eq!(l.width(), 2 + 5 + 2 + 10 + 3);
        let mut r = Line::new();
        r.text("ab", TEXT).right("xy", DIM, 10);
        assert_eq!(r.width(), 10);
    }

    #[test]
    fn panels_fill_each_row() {
        let mut p = Panel::new(12);
        let mut l = Line::new();
        l.text("abc", TEXT);
        p.push(l);
        p.rule(2);
        let mut out = Vec::new();
        p.write(&mut out);
        let text = String::from_utf8_lossy(&out);
        assert_eq!(text.matches('\n').count(), 2);
        assert!(text.contains("──────── ") || text.contains("────────"));
    }

    #[test]
    fn long_text_is_cut_with_an_ellipsis() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abc", 4), "abc");
        assert_eq!(fit("abc", 0), "");
    }
}
