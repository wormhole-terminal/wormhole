//! XTGETTCAP: terminfo capabilities asked for by name.
//!
//! Spec: xterm's ctlseqs. `DCS + q hex-name ; … ST` gets one `DCS 1 + r name = value ST` (or
//! `DCS 0 + r ST`) for each name, the names and values hex encoded. `dcs` collects the request.

use crate::terminal::Terminal;
use crate::terminal::dispatch::dcs::Hex;

impl Terminal {
    /// One reply per name asked for.
    pub(in crate::terminal) fn answer_xtgettcap(&mut self, request: &[u8]) {
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
