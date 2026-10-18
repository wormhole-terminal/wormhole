//! DECRQSS: a setting asked for, answered as the sequence that would set it.
//!
//! Spec: xterm's ctlseqs. `DCS $ q Pt ST` asks and is answered `DCS 1 $ r Pt ST`, or
//! `DCS 0 $ r ST` for a setting it does not know. `dcs` collects the request.

use crate::grid::Style;
use crate::types::{Color, Flags, NamedColor};

use super::Terminal;

impl Terminal {
    pub(super) fn answer_decrqss(&mut self, request: &[u8]) {
        let Some(setting) = Setting::parse(request) else {
            self.reply(format_args!("\x1bP0$r\x1b\\"));
            return;
        };
        let (top, bottom, left, right) =
            (self.margins.top + 1, self.margins.bottom + 1, self.margins.left + 1, self.margins.right + 1);
        let style = self.pen.style;
        let code = self.cursor_style_code();
        let protected = u8::from(self.pen.protected);
        let (rows, cols) = (self.rows, self.cols);
        let (level, c1) = (self.level.param(), u8::from(!self.eight_bit));
        self.reply(format_args!("\x1bP1$r"));
        match setting {
            Setting::Sgr => self.reply(format_args!("{}", SgrParams(&style))),
            Setting::Margins => self.reply(format_args!("{top};{bottom}r")),
            Setting::ColumnMargins => self.reply(format_args!("{left};{right}s")),
            Setting::CursorStyle => self.reply(format_args!("{code} q")),
            Setting::Protection => self.reply(format_args!("{protected}\"q")),
            Setting::Level => self.reply(format_args!("{level};{c1}\"p")),
            Setting::Rows => self.reply(format_args!("{rows}t")),
            Setting::Columns => self.reply(format_args!("{cols}$|")),
            Setting::Lines => self.reply(format_args!("{rows}*|")),
            Setting::Kept(setting) => {
                let kept = self.kept;
                match setting {
                    Kept::LocalFunctions => self.reply(format_args!("{}+q", kept.local_functions)),
                    Kept::FunctionKeyControl => self.reply(format_args!("{}*}}", kept.function_key_control)),
                    Kept::StatusDisplay => self.reply(format_args!("{}$}}", kept.status_display)),
                    Kept::ModifierKeys => self.reply(format_args!("{}+r", kept.modifier_keys)),
                    Kept::StatusLine => self.reply(format_args!("{}$~", kept.status_line)),
                    Kept::AttributeExtent => self.reply(format_args!("{}*x", kept.attribute_extent)),
                }
            }
        }
        self.reply(format_args!("\x1b\\"));
    }

    /// The DECSCUSR parameter that gives the cursor the style it has.
    fn cursor_style_code(&self) -> u8 {
        use crate::types::CursorShape::{Beam, Block, Underline};
        let steady = self.cursor_blink == Some(false);
        match self.cursor_shape {
            Some(Block) => 2 - u8::from(!steady),
            Some(Underline) => 4 - u8::from(!steady),
            Some(Beam) => 6 - u8::from(!steady),
            _ => 0,
        }
    }
}

/// A setting DECRQSS can be asked for.
#[derive(Clone, Copy)]
enum Setting {
    Sgr,
    Margins,
    ColumnMargins,
    CursorStyle,
    Protection,
    Level,
    Rows,
    Columns,
    Lines,
    /// A setting that is only kept, reported as the number and the sequence's tail.
    Kept(Kept),
}

#[derive(Clone, Copy)]
enum Kept {
    LocalFunctions,
    FunctionKeyControl,
    StatusDisplay,
    ModifierKeys,
    StatusLine,
    AttributeExtent,
}

impl Setting {
    fn parse(request: &[u8]) -> Option<Setting> {
        Some(match request {
            b"m" => Setting::Sgr,
            b"r" => Setting::Margins,
            b"s" => Setting::ColumnMargins,
            b" q" => Setting::CursorStyle,
            b"\"q" => Setting::Protection,
            b"\"p" => Setting::Level,
            b"t" => Setting::Rows,
            b"$|" => Setting::Columns,
            b"*|" => Setting::Lines,
            b"+q" => Setting::Kept(Kept::LocalFunctions),
            b"*}" => Setting::Kept(Kept::FunctionKeyControl),
            b"$}" => Setting::Kept(Kept::StatusDisplay),
            b"+r" => Setting::Kept(Kept::ModifierKeys),
            b"$~" => Setting::Kept(Kept::StatusLine),
            b"*x" => Setting::Kept(Kept::AttributeExtent),
            _ => return None,
        })
    }
}

/// The SGR sequence's parameters that would set a style: `0` then what differs from the
/// default, and the final `m`.
struct SgrParams<'a>(&'a Style);

impl std::fmt::Display for SgrParams<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let style = self.0;
        f.write_str("0")?;
        for (flag, code) in [
            (Flags::BOLD, "1"),
            (Flags::DIM, "2"),
            (Flags::ITALIC, "3"),
            (Flags::UNDERLINE, "4"),
            (Flags::DOUBLE_UNDERLINE, "21"),
            (Flags::UNDERCURL, "4:3"),
            (Flags::DOTTED_UNDERLINE, "4:4"),
            (Flags::DASHED_UNDERLINE, "4:5"),
            (Flags::BLINK, "5"),
            (Flags::INVERSE, "7"),
            (Flags::HIDDEN, "8"),
            (Flags::STRIKEOUT, "9"),
            (Flags::OVERLINE, "53"),
        ] {
            if style.attrs.contains(flag) {
                write!(f, ";{code}")?;
            }
        }
        write_color(f, style.fg, 30, 90, 38)?;
        write_color(f, style.bg, 40, 100, 48)?;
        if let Some(color) = style.underline_color {
            write_color(f, color, 0, 0, 58)?;
        }
        f.write_str("m")
    }
}

/// One color of an SGR string: the short form for the 16 named colors, the long one otherwise.
fn write_color(
    f: &mut std::fmt::Formatter<'_>,
    color: Color,
    base: u16,
    bright: u16,
    extended: u16,
) -> std::fmt::Result {
    match color {
        Color::Named(NamedColor::Foreground | NamedColor::Background | NamedColor::Cursor) => Ok(()),
        Color::Named(named) if base != 0 && (named as u16) < 8 => write!(f, ";{}", base + named as u16),
        Color::Named(named) if base != 0 && (named as u16) < 16 => {
            write!(f, ";{}", bright + named as u16 - 8)
        }
        Color::Named(named) => write!(f, ";{extended};5;{}", named as u16),
        Color::Indexed(n) => write!(f, ";{extended};5;{n}"),
        Color::Rgb(rgb) => write!(f, ";{extended};2;{};{};{}", rgb.r, rgb.g, rgb.b),
    }
}
