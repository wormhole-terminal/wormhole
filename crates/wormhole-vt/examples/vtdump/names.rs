//! The names of controls, escape sequences and CSI sequences (xterm's ctlseqs and the DEC manuals).

use std::fmt::Write as _;

use wormhole_vt::parser::Csi;

use crate::sgr::sgr_text;
use crate::text::escaped;

pub(crate) fn control_name(b: u8) -> String {
    let name = match b {
        0x00 => "NUL",
        0x05 => "ENQ",
        0x07 => "BEL",
        0x08 => "BS",
        0x09 => "HT",
        0x0a => "LF",
        0x0b => "VT",
        0x0c => "FF",
        0x0d => "CR",
        0x0e => "SO (shift out)",
        0x0f => "SI (shift in)",
        0x18 => "CAN",
        0x1a => "SUB",
        _ => return format!("C0 control {b:#04x}"),
    };
    name.to_string()
}

/// Final bytes of ESC sequences, with their intermediates (`(0` designates DEC special graphics).
pub(crate) fn esc_name(inter: &[u8], b: u8) -> String {
    let known = match (inter, b) {
        ([], b'7') => "DECSC (save cursor)",
        ([], b'8') => "DECRC (restore cursor)",
        ([], b'=') => "DECKPAM (application keypad)",
        ([], b'>') => "DECKPNM (numeric keypad)",
        ([], b'c') => "RIS (full reset)",
        ([], b'D') => "IND (index)",
        ([], b'E') => "NEL (next line)",
        ([], b'H') => "HTS (set tab stop)",
        ([], b'M') => "RI (reverse index)",
        ([], b'N') => "SS2",
        ([], b'O') => "SS3",
        ([], b'Z') => "DECID",
        ([], b'n') => "LS2",
        ([], b'o') => "LS3",
        ([], b'\\') => "ST",
        ([b'#'], b'3') => "DECDHL (double-height line, top half)",
        ([b'#'], b'4') => "DECDHL (double-height line, bottom half)",
        ([b'#'], b'5') => "DECSWL (single-width line)",
        ([b'#'], b'6') => "DECDWL (double-width line)",
        ([b'#'], b'8') => "DECALN (screen alignment test)",
        ([b' '], b'F') => "S7C1T (7-bit controls)",
        ([b' '], b'G') => "S8C1T (8-bit controls)",
        ([b'('], _) | ([b')'], _) | ([b'*'], _) | ([b'+'], _) => {
            let slot = inter.first().map_or('?', |&c| match c {
                b'(' => '0',
                b')' => '1',
                b'*' => '2',
                _ => '3',
            });
            return format!("designate G{slot} = {}", charset_name(b));
        }
        _ => return format!("ESC {}{}", escaped(inter), b as char),
    };
    known.to_string()
}

pub(crate) fn charset_name(b: u8) -> &'static str {
    match b {
        b'B' => "ASCII",
        b'0' => "DEC special graphics",
        b'A' => "UK",
        b'<' => "DEC supplemental",
        _ => "other",
    }
}

pub(crate) fn csi_name(csi: &Csi) -> String {
    let params = params_text(csi);
    let inter = csi.intermediates();
    let marker = csi.marker();
    let first = csi.get(0);
    let one = |what: &str| format!("{what} {params}").trim_end().to_string();
    let name = match (marker, inter, csi.final_byte()) {
        (None, [], b'@') => one("ICH"),
        (None, [], b'A') => one("CUU"),
        (None, [], b'B') => one("CUD"),
        (None, [], b'C') => one("CUF"),
        (None, [], b'D') => one("CUB"),
        (None, [], b'E') => one("CNL"),
        (None, [], b'F') => one("CPL"),
        (None, [], b'G') => one("CHA"),
        (None, [], b'H') => format!("CUP {}", if params.is_empty() { "1;1" } else { &params }),
        (None, [], b'I') => one("CHT"),
        (None, [], b'J') => format!("ED {} ({})", first.unwrap_or(0), ed_name(first)),
        (None, [], b'K') => format!("EL {} ({})", first.unwrap_or(0), el_name(first)),
        (None, [], b'L') => one("IL"),
        (None, [], b'M') => one("DL"),
        (None, [], b'P') => one("DCH"),
        (None, [], b'S') => one("SU"),
        (None, [], b'T') => one("SD"),
        (None, [], b'X') => one("ECH"),
        (None, [], b'Z') => one("CBT"),
        (None, [], b'`') => one("HPA"),
        (None, [], b'a') => one("HPR"),
        (None, [], b'b') => one("REP"),
        (None, [], b'c') => "DA1 (primary device attributes)".into(),
        (Some(b'>'), [], b'c') => "DA2 (secondary device attributes)".into(),
        (Some(b'='), [], b'c') => "DA3 (tertiary device attributes)".into(),
        (None, [], b'd') => one("VPA"),
        (None, [], b'e') => one("VPR"),
        (None, [], b'f') => one("HVP"),
        (None, [], b'g') => one("TBC"),
        (None, [], b'h') => format!("SM {params} (ANSI mode)"),
        (None, [], b'l') => format!("RM {params} (ANSI mode)"),
        (Some(b'?'), [], b'h') => mode_names("DECSET", csi),
        (Some(b'?'), [], b'l') => mode_names("DECRST", csi),
        (Some(b'?'), [], b's') => mode_names("XTSAVE", csi),
        (Some(b'?'), [], b'r') => mode_names("XTRESTORE", csi),
        (None, [], b'm') => {
            format!("SGR {} ({})", if params.is_empty() { "0" } else { &params }, sgr_text(csi))
        }
        (Some(b'>'), [], b'm') => one("XTMODKEYS"),
        (None, [], b'n') => format!("DSR {params} ({})", dsr_name(first, false)),
        (Some(b'?'), [], b'n') => format!("DECDSR {params} ({})", dsr_name(first, true)),
        (None, [], b'r') => format!("DECSTBM {}", if params.is_empty() { "(full screen)" } else { &params }),
        (None, [], b's') => {
            if csi.len() > 1 {
                one("DECSLRM")
            } else {
                "SCOSC / DECSLRM".into()
            }
        }
        (None, [], b't') => format!("XTWINOPS {params} ({})", winop_name(first)),
        (None, [], b'u') => "SCORC (restore cursor)".into(),
        (Some(b'>'), [], b'u') => one("kitty keyboard: push flags"),
        (Some(b'<'), [], b'u') => one("kitty keyboard: pop"),
        (Some(b'='), [], b'u') => one("kitty keyboard: set flags"),
        (Some(b'?'), [], b'u') => "kitty keyboard: query flags".into(),
        (None, [], b'x') => one("DECREQTPARM"),
        (None, [], b'i') => one("MC (media copy)"),
        (Some(b'>'), [], b'q') => "XTVERSION".into(),
        (None, [b' '], b'q') => format!("DECSCUSR {params} ({})", cursor_style_name(first)),
        (None, [b'!'], b'p') => "DECSTR (soft reset)".into(),
        (None, [b'"'], b'p') => one("DECSCL"),
        (None, [b'"'], b'q') => one("DECSCA"),
        (None, [b'$'], b'p') => format!("DECRQM {params} (ANSI mode)"),
        (Some(b'?'), [b'$'], b'p') => format!("DECRQM ?{params}"),
        (None, [b'$'], b'r') => one("DECCARA"),
        (None, [b'$'], b't') => one("DECRARA"),
        (None, [b'$'], b'v') => one("DECCRA"),
        (None, [b'$'], b'x') => one("DECFRA"),
        (None, [b'$'], b'z') => one("DECERA"),
        (None, [b'$'], b'{') => one("DECSERA"),
        (None, [b'*'], b'y') => one("DECRQCRA"),
        (None, [b'#'], b'{') | (None, [], b'{') => one("XTPUSHSGR"),
        (None, [b'#'], b'}') => one("XTPOPSGR"),
        (None, [b'\''], b'z') => one("DECELR"),
        (None, [b'\''], b'{') => one("DECSLE"),
        (None, [b'\''], b'|') => one("DECRQLP"),
        (None, [b'\''], b'w') => one("DECEFR"),
        (None, [b'\''], b'}') => one("DECIC"),
        (None, [b'\''], b'~') => one("DECDC"),
        _ => format!(
            "CSI {}{}{}{}",
            marker.map(|m| m as char).map(String::from).unwrap_or_default(),
            params,
            escaped(inter),
            csi.final_byte() as char
        ),
    };
    if csi.overflow() { format!("{name} (too many parameters, rest dropped)") } else { name }
}

/// The parameters as they were written: `1;31`, `38:2::10:20:30`.
pub(crate) fn params_text(csi: &Csi) -> String {
    let mut out = String::new();
    for (k, group) in csi.groups().enumerate() {
        if k > 0 {
            out.push(';');
        }
        for (j, value) in group.iter().enumerate() {
            if j > 0 {
                out.push(':');
            }
            let _ = write!(out, "{value}");
        }
    }
    // A lone empty parameter reads as none.
    if csi.len() == 1 && csi.get(0).is_none() { String::new() } else { out }
}

pub(crate) fn ed_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 => "cursor to end of screen",
        1 => "start of screen to cursor",
        2 => "whole screen",
        3 => "history",
        _ => "?",
    }
}

pub(crate) fn el_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 => "cursor to end of line",
        1 => "start of line to cursor",
        2 => "whole line",
        _ => "?",
    }
}

pub(crate) fn dsr_name(p: Option<u16>, private: bool) -> &'static str {
    match (private, p) {
        (false, Some(5)) => "status report",
        (false, Some(6)) => "cursor position report request",
        (true, Some(6)) => "DECXCPR",
        (true, Some(15)) => "printer status",
        (true, Some(25)) => "UDK status",
        (true, Some(26)) => "keyboard status",
        (true, Some(996)) => "color scheme query",
        _ => "?",
    }
}

pub(crate) fn winop_name(p: Option<u16>) -> &'static str {
    match p {
        Some(1) => "de-iconify",
        Some(2) => "iconify",
        Some(3) => "move window",
        Some(4) => "resize window in pixels",
        Some(5) => "raise",
        Some(6) => "lower",
        Some(7) => "refresh",
        Some(8) => "resize in cells",
        Some(9) => "maximize",
        Some(10) => "full screen",
        Some(11) => "report window state",
        Some(13) => "report window position",
        Some(14) => "report window size in pixels",
        Some(15) => "report screen size in pixels",
        Some(16) => "report cell size in pixels",
        Some(18) => "report size in cells",
        Some(19) => "report screen size in cells",
        Some(20) => "report icon label",
        Some(21) => "report title",
        Some(22) => "push title",
        Some(23) => "pop title",
        _ => "?",
    }
}

pub(crate) fn cursor_style_name(p: Option<u16>) -> &'static str {
    match p.unwrap_or(0) {
        0 | 1 => "blinking block",
        2 => "steady block",
        3 => "blinking underline",
        4 => "steady underline",
        5 => "blinking bar",
        6 => "steady bar",
        _ => "?",
    }
}

pub(crate) fn mode_names(name: &str, csi: &Csi) -> String {
    let mut out = format!("{name} {}", params_text(csi));
    let described: Vec<String> = csi
        .groups()
        .map(|g| g[0])
        .map(|m| mode_name(m).map_or_else(|| "unknown mode".into(), String::from))
        .collect();
    let _ = write!(out, " ({})", described.join(", "));
    out
}

pub(crate) fn mode_name(mode: u16) -> Option<&'static str> {
    Some(match mode {
        1 => "application cursor keys",
        3 => "132 columns",
        4 => "smooth scroll",
        5 => "reverse video",
        6 => "origin mode",
        7 => "autowrap",
        8 => "autorepeat",
        9 => "X10 mouse",
        12 => "blinking cursor",
        25 => "show cursor",
        40 => "allow 132 columns",
        45 => "reverse wrap",
        47 => "alternate screen",
        66 => "application keypad",
        69 => "left and right margins",
        80 => "sixel scrolling",
        1000 => "mouse press and release",
        1001 => "mouse highlight",
        1002 => "mouse drag",
        1003 => "mouse any motion",
        1004 => "focus events",
        1005 => "UTF-8 mouse",
        1006 => "SGR mouse",
        1007 => "alternate scroll",
        1015 => "urxvt mouse",
        1016 => "pixel mouse",
        1047 => "alternate screen",
        1048 => "save cursor",
        1049 => "alternate screen and save cursor",
        2004 => "bracketed paste",
        2026 => "synchronized output",
        2027 => "grapheme clusters",
        2031 => "color scheme updates",
        _ => return None,
    })
}
