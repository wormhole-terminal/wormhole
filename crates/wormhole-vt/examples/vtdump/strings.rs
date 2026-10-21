//! The names of control strings: OSC, DCS and APC.

use wormhole_vt::parser::Csi;

use crate::items::{KEEP, Open};
use crate::names::params_text;
use crate::text::{escaped, shortened};

pub(crate) fn string_name(open: &Open) -> String {
    match open {
        Open::Osc(data, total) => osc_name(data, *total),
        Open::Dcs(header, data, total) => dcs_name(header, data, *total),
        Open::Apc(data, total) => apc_name(data, *total),
    }
}

pub(crate) fn osc_name(data: &[u8], total: usize) -> String {
    let (number, rest) = match data.iter().position(|&b| b == b';') {
        Some(at) => (&data[..at], &data[at + 1..]),
        None => (data, &data[data.len()..]),
    };
    let number = String::from_utf8_lossy(number).into_owned();
    let text = || shortened(&String::from_utf8_lossy(rest), 48);
    let name = match number.as_str() {
        "0" => format!("title and icon name {:?}", text()),
        "1" => format!("icon name {:?}", text()),
        "2" => format!("title {:?}", text()),
        "4" => "set palette color".into(),
        "7" => format!("working directory {:?}", text()),
        "8" => {
            let uri = rest.iter().position(|&b| b == b';').map_or(&rest[rest.len()..], |at| &rest[at + 1..]);
            if uri.is_empty() {
                "hyperlink end".into()
            } else {
                format!("hyperlink {:?}", shortened(&String::from_utf8_lossy(uri), 48))
            }
        }
        "9" if rest.starts_with(b"4;") => "progress".into(),
        "9" => format!("notification {:?}", text()),
        "10" => "foreground color".into(),
        "11" => "background color".into(),
        "12" => "cursor color".into(),
        "52" => "clipboard".into(),
        "99" => "kitty notification".into(),
        "104" => "reset palette color".into(),
        "110" => "reset foreground".into(),
        "111" => "reset background".into(),
        "112" => "reset cursor color".into(),
        "133" => match rest.first() {
            Some(b'A') => "shell integration: prompt start".into(),
            Some(b'B') => "shell integration: command start".into(),
            Some(b'C') => "shell integration: output start".into(),
            Some(b'D') => format!(
                "shell integration: command finished {}",
                shortened(&String::from_utf8_lossy(&rest[1..]), 16)
            )
            .trim_end()
            .into(),
            _ => "shell integration".into(),
        },
        "777" => "notification (rxvt)".into(),
        "1337" => {
            if rest.starts_with(b"File=") {
                "iTerm2 inline image".into()
            } else {
                format!("iTerm2 {:?}", shortened(&String::from_utf8_lossy(rest), 24))
            }
        }
        _ => "unknown".into(),
    };
    let more = if total > KEEP { format!(", {total} bytes") } else { String::new() };
    format!("OSC {number} {name}{more}")
}

pub(crate) fn dcs_name(header: &Csi, data: &[u8], total: usize) -> String {
    let inter = header.intermediates();
    let kind = match (header.marker(), inter, header.final_byte()) {
        (None, [], b'q') => format!("Sixel, {total} bytes"),
        (None, [b'$'], b'q') => format!("DECRQSS, request {:?}", String::from_utf8_lossy(data)),
        (None, [b'+'], b'q') => format!("XTGETTCAP, {} bytes", total),
        (None, [b'+'], b'p') => format!("XTSETTCAP, {} bytes", total),
        (None, [b'$'], b't') => format!("DECRSPS, {total} bytes"),
        (None, [b'|'], _) => "DECUDK".into(),
        _ => format!("{} bytes", total),
    };
    let params = params_text(header);
    format!("DCS {params}{}{} ({kind})", escaped(inter), header.final_byte() as char)
}

pub(crate) fn apc_name(data: &[u8], total: usize) -> String {
    match data.first() {
        Some(b'G') => {
            let keys = data[1..].iter().position(|&b| b == b';').map_or(&data[1..], |at| &data[1..1 + at]);
            format!("APC G kitty graphics {} ({total} bytes)", shortened(&String::from_utf8_lossy(keys), 60))
        }
        _ => format!("APC ({total} bytes)"),
    }
}
