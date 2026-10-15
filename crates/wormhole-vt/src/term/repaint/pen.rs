//! The pen of the terminal being written: the rendition, link, protection and block mark the next
//! cell printed will carry, kept as the bytes written so far leave them, so that a change costs
//! only what changed.

use crate::grid::{Content, Style};
use crate::types::{Color, Flags, Hyperlink, NamedColor};

use super::{Gaps, Writer};

/// What the receiver's pen holds now.
#[derive(Clone, Debug)]
pub(super) struct Pen<'a> {
    pub(super) style: Style,
    pub(super) link: Option<&'a Hyperlink>,
    pub(super) protected: bool,
    pub(super) guarded: bool,
    pub(super) content: Content,
    /// The block the pen belongs to; 0 before the first `OSC 133;A`.
    pub(super) block: u32,
}

impl Default for Pen<'_> {
    fn default() -> Self {
        Pen {
            style: Style::DEFAULT,
            link: None,
            protected: false,
            guarded: false,
            content: Content::None,
            block: 0,
        }
    }
}

/// The SGR parameters for an attribute flag, in the order they are written.
const ATTRS: [(Flags, &str); 8] = [
    (Flags::BOLD, "1"),
    (Flags::DIM, "2"),
    (Flags::ITALIC, "3"),
    (Flags::BLINK, "5"),
    (Flags::INVERSE, "7"),
    (Flags::HIDDEN, "8"),
    (Flags::STRIKEOUT, "9"),
    (Flags::OVERLINE, "53"),
];

const UNDERLINES: [(Flags, &str); 5] = [
    (Flags::UNDERLINE, "4"),
    (Flags::DOUBLE_UNDERLINE, "21"),
    (Flags::UNDERCURL, "4:3"),
    (Flags::DOTTED_UNDERLINE, "4:4"),
    (Flags::DASHED_UNDERLINE, "4:5"),
];

impl<'a> Writer<'a> {
    /// Bring the receiver's pen to this look.
    pub(super) fn set_style(&mut self, style: &Style) {
        let current = self.pen.style;
        if current == *style {
            return;
        }
        if *style == Style::DEFAULT {
            self.put(b"\x1b[m");
            self.pen.style = *style;
            return;
        }
        // Only the colors differ: say those. Otherwise start over from the default.
        let colors_only = style.attrs == current.attrs && style.underline_color == current.underline_color;
        self.put(b"\x1b[");
        let mut first = true;
        let mut sep = |w: &mut Self| {
            if !std::mem::take(&mut first) {
                w.out.push(b';');
            }
        };
        if !colors_only {
            sep(self);
            self.out.push(b'0');
            for (_, code) in ATTRS.iter().filter(|(flag, _)| style.attrs.contains(*flag)) {
                sep(self);
                self.put(code.as_bytes());
            }
            if let Some((_, code)) = UNDERLINES.iter().find(|(flag, _)| style.attrs.contains(*flag)) {
                sep(self);
                self.put(code.as_bytes());
            }
        }
        let (old_fg, old_bg) =
            if colors_only { (current.fg, current.bg) } else { (Style::DEFAULT.fg, Style::DEFAULT.bg) };
        if style.fg != old_fg {
            sep(self);
            self.color(30, style.fg);
        }
        if style.bg != old_bg {
            sep(self);
            self.color(40, style.bg);
        }
        if !colors_only && let Some(underline) = style.underline_color {
            sep(self);
            self.color(50, underline);
        }
        self.out.push(b'm');
        self.pen.style = *style;
    }

    /// The parameters that select `color` for the foreground (`base` 30), the background (40) or
    /// the underline (50, which only has the extended forms).
    fn color(&mut self, base: u16, color: Color) {
        match color {
            Color::Named(NamedColor::Foreground) if base == 30 => self.put(b"39"),
            Color::Named(NamedColor::Background) if base == 40 => self.put(b"49"),
            Color::Named(named) if (named as u16) < 16 && base != 50 => {
                let n = named as u16;
                let code = if n < 8 { base + n } else { base + 60 + (n - 8) };
                self.num(usize::from(code));
            }
            Color::Named(_) => {
                // A default or the cursor color as the color of a cell: no SGR says that.
                self.gaps |= Gaps::COLOR;
                self.put(match base {
                    40 => b"49",
                    50 => b"59",
                    _ => b"39",
                });
            }
            Color::Indexed(i) => {
                self.num(usize::from(base) + 8);
                self.put(b";5;");
                self.num(usize::from(i));
            }
            Color::Rgb(rgb) => {
                self.num(usize::from(base) + 8);
                self.put(b";2;");
                self.num(usize::from(rgb.r));
                self.out.push(b';');
                self.num(usize::from(rgb.g));
                self.out.push(b';');
                self.num(usize::from(rgb.b));
            }
        }
    }

    /// OSC 8: the link the next cells are under. A link is written with the id it has, so the
    /// receiver's cells carry the same one.
    pub(super) fn set_link(&mut self, link: Option<&'a Hyperlink>) {
        if self.pen.link == link {
            return;
        }
        self.put(b"\x1b]8;");
        if let Some(link) = link {
            self.put(b"id=");
            self.put(link.id.as_bytes());
        }
        self.out.push(b';');
        if let Some(link) = link {
            self.put(link.uri.as_bytes());
        }
        self.put(b"\x1b\\");
        self.pen.link = link;
    }

    /// DECSCA.
    pub(super) fn set_protected(&mut self, on: bool) {
        if self.pen.protected != on {
            self.put(if on { b"\x1b[1\"q" } else { b"\x1b[0\"q" });
            self.pen.protected = on;
        }
    }

    /// SPA and EPA.
    pub(super) fn set_guarded(&mut self, on: bool) {
        if self.pen.guarded != on {
            self.put(if on { b"\x1bV" } else { b"\x1bW" });
            self.pen.guarded = on;
        }
    }

    /// Everything the pen can carry back to the receiver's default (the block it belongs to
    /// stays: nothing but a reset takes it away).
    pub(super) fn reset_pen(&mut self) {
        self.set_style(&Style::DEFAULT);
        self.set_link(None);
        self.set_protected(false);
        self.set_guarded(false);
        self.want_content(0, Content::None);
    }

    /// Bring the pen to the content `content` of block `block` (OSC 133). A block is entered by
    /// counting the receiver's counter up to it with prompt marks, which is why the blocks of
    /// the rows can only come in the order they are numbered.
    pub(super) fn want_content(&mut self, block: u32, content: Content) {
        if content == Content::None {
            if self.pen.content != Content::None {
                self.osc("133;D");
                self.pen.content = Content::None;
            }
            return;
        }
        if block == 0 {
            self.gaps |= Gaps::BLOCKS;
            return self.want_content(0, Content::None);
        }
        if self.pen.block != block {
            let seed = *self.seed_block.get_or_insert_with(|| block - 1);
            if self.counter < seed {
                self.counter = seed;
            }
            if block <= self.counter {
                self.gaps |= Gaps::BLOCKS;
                return self.want_content(0, Content::None);
            }
            while self.counter < block {
                self.osc("133;A");
                self.counter += 1;
            }
            self.pen.block = block;
            self.pen.content = Content::Prompt;
        }
        if self.alt && matches!(content, Content::Command | Content::Output) {
            self.gaps |= Gaps::ALT_CONTENT;
            return;
        }
        if self.pen.content != content {
            self.osc(match content {
                // A prompt that goes on in the same block: a secondary prompt does not count up.
                Content::Prompt => "133;A;k=s",
                Content::Command => "133;B",
                _ => "133;C",
            });
            self.pen.content = content;
        }
    }
}
