//! The comparison: what the two engines show after the same stream, with the few differences
//! that are not mistakes left out.

use wormhole_vt_bench::Engine;
use wormhole_vt_bench::diff::Split;
use wormhole_vt_bench::engines::WormholeEngine;
use wormhole_vt_bench::vtt::show_bytes;

use super::fixed::FixedAlacritty;

fn feed<E: Engine>(engine: &mut E, bytes: &[u8], how: Split) {
    match how {
        Split::Whole => engine.feed(bytes),
        Split::Bytes(n) => bytes.chunks(n.max(1)).for_each(|piece| engine.feed(piece)),
        Split::Random { seed, max } => {
            let mut rng = wormhole_vt_bench::rng::Rng::new(seed);
            let mut rest = bytes;
            while !rest.is_empty() {
                let take = rng.range(1, max.max(1)).min(rest.len());
                let (piece, tail) = rest.split_at(take);
                engine.feed(piece);
                rest = tail;
            }
        }
    }
}

/// Whether the stream has a CSI whose final byte is one of `finals`.
fn has_csi(stream: &[u8], finals: &[u8]) -> bool {
    stream.windows(2).enumerate().any(|(i, w)| {
        w == b"\x1b[" && {
            let rest = &stream[i + 2..];
            let end = rest.iter().position(|b| !(0x20..=0x3f).contains(b));
            end.is_some_and(|n| finals.contains(&rest[n]))
        }
    })
}

/// Whether the rows' soft-wrap flags mean the same in both. alacritty keeps the flag in the last
/// cell of a row, so writing there again, an insert, an erase or a scroll that moves or replaces
/// that cell changes it. The new engine keeps it with the row: a row stays a continuation of the
/// one above it when its last cell is overwritten, and the flag moves with the row. So they are
/// compared only where nothing but text, line feeds, tabs and SGR came: the flag is checked by
/// the conformance suite in the other cases.
fn wrap_is_comparable(stream: &[u8]) -> bool {
    let mut i = 0;
    while i < stream.len() {
        match stream[i] {
            0x08 | 0x0d => return false,
            0x1b => {
                // Only SGR moves nothing.
                let rest = &stream[i + 1..];
                let Some(body) = rest.strip_prefix(b"[") else { return false };
                let end = body.iter().position(|b| (0x40..=0x7e).contains(b));
                match end {
                    Some(n) if body[n] == b'm' => i += 2 + n,
                    _ => return false,
                }
            }
            _ => {}
        }
        i += 1;
    }
    true
}

/// Streams in which alacritty's handling of wide characters and marks is what would show. It
/// leaves half of a wide character when an erase or an insert cuts it (`known-fail: alacritty`),
/// loses a character at the edge in insert mode, and puts a mark on another cell than the new
/// engine when autowrap is off. What those do is in the conformance suite; here they only hide
/// the rest.
fn unreliable(stream: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stream);
    let wide = text.chars().any(|c| matches!(c, '日' | '本' | '語' | '한' | '😀' | 'Ａ'));
    let marks = !text.is_ascii();
    let insert = stream.windows(4).any(|w| w == b"\x1b[4h");
    let edits = has_csi(stream, b"JKX@P");
    let no_wrap = stream.windows(5).any(|w| w == b"\x1b[?7l");
    (wide && (insert || edits)) || (no_wrap && marks)
}

/// The first difference between alacritty and the new engine on `stream`; the new engine is fed
/// as `how` says, alacritty whole.
pub fn differ(stream: &[u8], (cols, rows): (usize, usize), how: Split) -> Option<String> {
    if unreliable(stream) {
        return None;
    }
    let mut a = FixedAlacritty::new(cols, rows, 50);
    let mut b = WormholeEngine::new(cols, rows, 50);
    feed(&mut a, stream, Split::Whole);
    feed(&mut b, stream, how);
    let history = !has_csi(stream, b"M");
    let wrapped = wrap_is_comparable(stream);
    let diff = |what: String, x: String, y: String| {
        (x != y).then(|| format!("{what}\n  alacritty: {x}\n  wormhole:  {y}"))
    };
    let (ra, rb) = (a.take_replies(), b.take_replies());
    if let Some(d) = diff("replies".into(), show_bytes(&ra), show_bytes(&rb)) {
        return Some(d);
    }
    if let Some(d) = diff("title".into(), format!("{:?}", a.title()), format!("{:?}", b.title())) {
        return Some(d);
    }
    if let Some(d) = diff("alternate screen".into(), a.alt_screen().to_string(), b.alt_screen().to_string()) {
        return Some(d);
    }
    let (ca, cb) = (a.cursor(), b.cursor());
    if (ca.row, ca.col, ca.visible) != (cb.row, cb.col, cb.visible) {
        return diff("cursor".into(), format!("{ca:?}"), format!("{cb:?}"));
    }
    for row in 0..rows {
        if let Some(d) = diff(
            format!("row {} text", row + 1),
            format!("{:?}", a.row_text(row)),
            format!("{:?}", b.row_text(row)),
        ) {
            return Some(d);
        }
        if wrapped
            && let Some(d) = diff(
                format!("row {} wrapped", row + 1),
                a.row_wrapped(row).to_string(),
                b.row_wrapped(row).to_string(),
            )
        {
            return Some(d);
        }
        for col in 0..cols {
            let (mut x, y) = (a.cell(row, col), b.cell(row, col));
            // The blank a wide character that did not fit leaves at the end of a row: its
            // look is nobody's, the new engine gives it the background only.
            // A blank that was half of a wide character keeps the hyperlink in alacritty.
            if x.text == " " && y.text == " " {
                x.link = y.link.clone();
            }
            if col == cols - 1 && x.text == " " && y.text == " " && a.row_wrapped(row) {
                x.attrs = y.attrs;
                x.fg = y.fg;
                x.underline_color = y.underline_color;
            }
            if x != y {
                return diff(format!("cell {},{}", row + 1, col + 1), format!("{x:?}"), format!("{y:?}"));
            }
        }
    }
    if history {
        if let Some(d) =
            diff("history length".into(), a.scrollback_len().to_string(), b.scrollback_len().to_string())
        {
            return Some(d);
        }
        for i in 0..a.scrollback_len() {
            if let Some(d) = diff(
                format!("history line {}", i + 1),
                format!("{:?}", a.scrollback_text(i)),
                format!("{:?}", b.scrollback_text(i)),
            ) {
                return Some(d);
            }
        }
    }
    None
}
