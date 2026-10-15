//! Device control strings and application program commands.
//!
//! Spec: xterm's ctlseqs. DECRQSS (`DCS $ q Pt ST`) asks for a setting as the sequence that would
//! set it and is answered `DCS 1 $ r Pt ST`, or `DCS 0 $ r ST` for a setting it does not know.
//! XTGETTCAP (`DCS + q hex-name ; … ST`) asks for terminfo capabilities by name, hex encoded, and
//! gets one `DCS 1 + r name = value ST` (or `DCS 0 + r ST`) for each. `DCS q` is sixel, `DCS = 1 s`
//! and `DCS = 2 s` begin and end a synchronized update in the older iTerm2 form, and APC with a
//! `G` is the kitty graphics protocol. The image protocols are `graphics`' business; this file
//! only collects a sixel's data and passes an APC string on as it streams in.

use crate::grid::Style;
use crate::parser::Csi;
use crate::types::{Color, Flags, NamedColor};

use super::Terminal;
use super::conformance::Level;
use super::modes::Modes;

/// Most a DECRQSS request is (the longest is two bytes).
const RQSS_MAX: usize = 32;
/// Most an XTGETTCAP request is.
const TCAP_MAX: usize = 4096;
/// Most a Sixel's data is; it is decoded when the string ends, on the parser's thread.
const SIXEL_MAX: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    None,
    Decrqss,
    Xtgettcap,
    Sixel,
    /// DECAUPSS, with the number before the `!`.
    Preferred(u16),
    /// DECRSPS and DECRSTS: a report to read back (`presentation`, `state`).
    RestorePresentation(u16),
    RestoreState(u16),
    /// An APC string whose first byte has not come.
    ApcFirst,
    /// A kitty graphics command, streamed to `graphics`.
    Apc,
    /// Read and thrown away.
    Ignore,
}

/// The string being read, whichever sort it is.
#[derive(Default)]
pub(super) struct DcsString {
    kind: Kind,
    data: Vec<u8>,
    /// The numbers before a sixel's `q`.
    params: Vec<u16>,
}

impl DcsString {
    pub(super) fn cancel(&mut self) {
        self.kind = Kind::None;
        self.data = Vec::new();
    }

    pub(super) fn put(&mut self, data: &[u8]) {
        let limit = match self.kind {
            Kind::Decrqss => RQSS_MAX,
            Kind::Xtgettcap => TCAP_MAX,
            Kind::Sixel => SIXEL_MAX,
            Kind::Preferred(_) => RQSS_MAX,
            Kind::RestorePresentation(_) | Kind::RestoreState(_) => super::presentation::RESTORE_MAX,
            _ => return,
        };
        self.append(data, limit);
    }

    fn append(&mut self, data: &[u8], limit: usize) {
        if self.data.len() + data.len() > limit {
            self.kind = Kind::Ignore;
            self.data = Vec::new();
        } else {
            self.data.extend_from_slice(data);
        }
    }

    pub(super) fn start_apc(&mut self) {
        self.kind = Kind::ApcFirst;
        self.data = Vec::new();
    }
}

impl Terminal {
    pub(super) fn hook_dcs(&mut self, csi: &Csi) {
        self.dcs.data.clear();
        self.dcs.kind = match (csi.marker(), csi.intermediates(), csi.final_byte()) {
            (None, [b'$'], b'q') => Kind::Decrqss,
            (None, [b'+'], b'q') => Kind::Xtgettcap,
            (None, [], b'q') => {
                self.dcs.params.clear();
                self.dcs.params.extend_from_slice(csi.values());
                Kind::Sixel
            }
            (None, [b'!'], b'u') => Kind::Preferred(csi.param(0, 0)),
            (None, [b'$'], b't') if self.level >= Level::Vt300 => Kind::RestorePresentation(csi.param(0, 0)),
            (None, [b'$'], b'p') if self.level >= Level::Vt400 => Kind::RestoreState(csi.param(0, 0)),
            (Some(b'='), [], b's') => {
                match csi.param(0, 0) {
                    1 => self.modes.insert(Modes::SYNC_OUTPUT),
                    2 => self.modes.remove(Modes::SYNC_OUTPUT),
                    _ => {}
                }
                Kind::Ignore
            }
            _ => Kind::Ignore,
        };
    }

    pub(super) fn unhook_dcs(&mut self) {
        let kind = std::mem::take(&mut self.dcs.kind);
        match kind {
            // The requests are short and answered at once: the buffer stays for the next.
            Kind::Decrqss | Kind::Xtgettcap => {
                let data = std::mem::take(&mut self.dcs.data);
                if kind == Kind::Decrqss {
                    self.answer_decrqss(&data);
                } else {
                    self.answer_xtgettcap(&data);
                }
                self.dcs.data = data;
                self.dcs.data.clear();
            }
            Kind::Preferred(size) => {
                let data = std::mem::take(&mut self.dcs.data);
                self.assign_preferred_set(size, &data);
                self.dcs.data = data;
                self.dcs.data.clear();
            }
            Kind::RestorePresentation(ps) | Kind::RestoreState(ps) => {
                let data = std::mem::take(&mut self.dcs.data);
                if matches!(kind, Kind::RestoreState(_)) {
                    self.restore_state(ps, &data);
                } else {
                    self.restore_presentation(ps, &data);
                }
                // A report can be long (tab stops of a wide screen): the buffer is not kept.
            }
            Kind::Sixel => {
                let data = std::mem::take(&mut self.dcs.data);
                self.run_sixel(&self.dcs.params.clone(), &data);
                // The buffer is kept for the next picture unless a big one grew it.
                if data.capacity() <= 64 * 1024 {
                    self.dcs.data = data;
                    self.dcs.data.clear();
                }
            }
            _ => {}
        }
    }

    /// The next slice of an APC string. Only kitty's graphics protocol (`G`) is read; the other
    /// APC strings are dropped.
    pub(super) fn put_apc(&mut self, data: &[u8]) {
        if self.dcs.kind == Kind::ApcFirst {
            match data.first() {
                None => return,
                Some(b'G') => {
                    self.dcs.kind = Kind::Apc;
                    self.begin_kitty();
                    self.put_kitty(&data[1..]);
                    return;
                }
                Some(_) => self.dcs.kind = Kind::Ignore,
            }
        }
        if self.dcs.kind == Kind::Apc {
            self.put_kitty(data);
        }
    }

    pub(super) fn finish_apc(&mut self) {
        if std::mem::take(&mut self.dcs.kind) == Kind::Apc {
            self.finish_kitty();
        }
    }

    pub(super) fn cancel_apc(&mut self) {
        if self.dcs.kind == Kind::Apc {
            self.cancel_kitty();
        }
        self.dcs.cancel();
    }

    // ---- DECRQSS ----

    fn answer_decrqss(&mut self, request: &[u8]) {
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

    // ---- XTGETTCAP ----

    fn answer_xtgettcap(&mut self, request: &[u8]) {
        for hex_name in request.split(|&b| b == b';') {
            let mut name = [0u8; NAME_MAX];
            let value = decode_hex(hex_name, &mut name)
                .and_then(|len| std::str::from_utf8(&name[..len]).ok())
                .and_then(capability);
            // `decode_hex` accepted it, so the name is hex digits: ASCII.
            let echoed = std::str::from_utf8(hex_name).unwrap_or("");
            match value {
                Some("") => self.reply(format_args!("\x1bP1+r{echoed}\x1b\\")),
                Some(value) => self.reply(format_args!("\x1bP1+r{echoed}={}\x1b\\", Hex(value.as_bytes()))),
                None => self.reply(format_args!("\x1bP0+r\x1b\\")),
            }
        }
    }
}

/// The longest capability name asked for (terminfo's are two to five characters).
const NAME_MAX: usize = 32;

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

/// The terminfo capabilities Wormhole answers (the values are terminfo's strings, with `\E`
/// as ESC; an empty value is a boolean). The terminal is an xterm-256color with true color, styled
/// and colored underlines, cursor styles, synchronized output, focus and paste reporting.
fn capability(name: &str) -> Option<&'static str> {
    Some(match name {
        "TN" | "name" => "xterm-256color",
        "Co" | "colors" => "256",
        "RGB" => "8/8/8",
        "Tc" | "bce" | "XT" => "",
        "setrgbf" => "\x1b[38;2;%p1%d;%p2%d;%p3%dm",
        "setrgbb" => "\x1b[48;2;%p1%d;%p2%d;%p3%dm",
        "Smulx" => "\x1b[4:%p1%dm",
        "Setulc" => "\x1b[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m",
        "Ss" => "\x1b[%p1%d q",
        "Se" => "\x1b[2 q",
        "Ms" => "\x1b]52;%p1%s;%p2%s\x07",
        "BE" => "\x1b[?2004h",
        "BD" => "\x1b[?2004l",
        "PS" => "\x1b[200~",
        "PE" => "\x1b[201~",
        "fe" => "\x1b[?1004h",
        "fd" => "\x1b[?1004l",
        "Sync" => "\x1bP=%p1%ds\x1b\\",
        _ => return None,
    })
}

/// Decode the hex digits of `text` into `out`; the number of bytes, or `None` when it is not
/// hex or does not fit.
fn decode_hex(text: &[u8], out: &mut [u8]) -> Option<usize> {
    if !text.len().is_multiple_of(2) || text.len() / 2 > out.len() {
        return None;
    }
    for (slot, pair) in out.iter_mut().zip(text.chunks(2)) {
        let hi = char::from(pair[0]).to_digit(16)?;
        let lo = char::from(pair[1]).to_digit(16)?;
        *slot = (hi * 16 + lo) as u8;
    }
    Some(text.len() / 2)
}

/// Bytes as uppercase hex digits.
pub(super) struct Hex<'a>(pub(super) &'a [u8]);

impl std::fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02X}"))
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
