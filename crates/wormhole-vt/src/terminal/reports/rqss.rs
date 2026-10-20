//! DECRQSS: a setting asked for, answered as the sequence that would set it.
//!
//! Spec: xterm's ctlseqs. `DCS $ q Pt ST` asks and is answered `DCS 1 $ r Pt ST`, with the sequence
//! that sets the setting in place of `Pt` (without its `CSI`), or `DCS 0 $ r ST` for a setting it
//! does not have. `dcs` collects the request.
//!
//! What the engine answers, besides the VT420's settings it keeps: xterm's `> 4 m` (modifyOtherKeys, as
//! XTMODKEYS would set it: `> 4 ; Pv m`) and `> Pm t` (the title modes that are on, as XTSMTITLE
//! would set them: `> Pm ; Pm t`; invalid while none is, since there is nothing to restore).
//! Not answered, because the engine has no such setting and a terminal that does not have one answers
//! invalid: DECPRO (`}`), DECTTC (`|`), DECTLTC (`'s`), the VT520's (DECATC `, }`, DECAC `, |`, DECCRTST
//! `- q`, DECDLDA `, z`, DECSEST `- r`, DECARR `- p`, DECSTGLT `) {`, DECSZS `, {`, DECSPMA `, x`, DECTME
//! `SP ~`, DECUS `, y`, and the rest of that menu), and xterm's other key modifier options and key
//! formats (`> Pm m` with a `Pm` other than 4, `> Pm f`).

use crate::grid::Style;
use crate::types::{Color, Flags, NamedColor};

use crate::terminal::Terminal;

impl Terminal {
    pub(in crate::terminal) fn answer_decrqss(&mut self, request: &[u8]) {
        let Some(setting) = Setting::parse(request) else {
            self.reply(format_args!("\x1bP0$r\x1b\\"));
            return;
        };
        let title_modes = self.titles.modes.parts();
        if matches!(setting, Setting::TitleModes) && title_modes.iter().all(|(on, _)| !on) {
            self.reply(format_args!("\x1bP0$r\x1b\\"));
            return;
        }
        let (top, bottom, left, right) =
            (self.margins.top + 1, self.margins.bottom + 1, self.margins.left + 1, self.margins.right + 1);
        let style = self.pen.style;
        let code = self.cursor_style_code();
        let protected = u8::from(self.pen.protected);
        let (rows, cols) = (self.rows, self.cols);
        let (level, c1) = (self.level.param(), u8::from(!self.eight_bit));
        self.reply_with(|t, reply| {
            reply.put(format_args!("\x1bP1$r"));
            match setting {
                Setting::Sgr => reply.put(format_args!("{}", SgrParams(&style))),
                Setting::Margins => reply.put(format_args!("{top};{bottom}r")),
                Setting::ColumnMargins => reply.put(format_args!("{left};{right}s")),
                Setting::CursorStyle => reply.put(format_args!("{code} q")),
                Setting::Protection => reply.put(format_args!("{protected}\"q")),
                Setting::Level => reply.put(format_args!("{level};{c1}\"p")),
                Setting::Rows => reply.put(format_args!("{rows}t")),
                Setting::Columns => reply.put(format_args!("{cols}$|")),
                Setting::Lines => reply.put(format_args!("{rows}*|")),
                Setting::ModifyOtherKeys => {
                    reply.put(format_args!(">4;{}m", t.modify_other_keys_level()));
                }
                Setting::TitleModes => {
                    reply.put(format_args!(">"));
                    let mut first = true;
                    for (_, param) in title_modes.iter().filter(|(on, _)| *on) {
                        reply.put(format_args!("{}{param}", if first { "" } else { ";" }));
                        first = false;
                    }
                    reply.put(format_args!("t"));
                }
                Setting::Kept(setting) => {
                    let kept = t.kept;
                    match setting {
                        Kept::LocalFunctions => reply.put(format_args!("{}+q", kept.local_functions)),
                        Kept::FunctionKeyControl => {
                            reply.put(format_args!("{}*}}", kept.function_key_control));
                        }
                        Kept::StatusDisplay => {
                            reply.put(format_args!("{}$}}", u8::from(t.status.active())));
                        }
                        Kept::ModifierKeys => reply.put(format_args!("{}+r", kept.modifier_keys)),
                        Kept::StatusLine => reply.put(format_args!("{}$~", t.status.kind())),
                        Kept::AttributeExtent => reply.put(format_args!("{}*x", kept.attribute_extent)),
                    }
                }
            }
            reply.put(format_args!("\x1b\\"));
        });
    }

    /// The DECSCUSR parameter that gives the cursor the style it has.
    fn cursor_style_code(&self) -> u8 {
        use crate::types::CursorShape::{Beam, Block, Underline};
        let steady = self.cursor_style.blink == Some(false);
        match self.cursor_style.shape {
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
    /// xterm's XTQMODKEYS for modifyOtherKeys, `> 4 m`.
    ModifyOtherKeys,
    /// xterm's XTSMTITLE, `> Pm t`.
    TitleModes,
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
            b">4m" => Setting::ModifyOtherKeys,
            // The parameters name the modes to ask about; the answer lists those that are on.
            [b'>', params @ .., b't'] if params.iter().all(|b| b.is_ascii_digit() || *b == b';') => {
                Setting::TitleModes
            }
            _ => return None,
        })
    }
}

/// The SGR sequence's parameters that would set a style: `0` then what differs from the
/// default, and the final `m`.
pub(in crate::terminal) struct SgrParams<'a>(pub(in crate::terminal) &'a Style);

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
