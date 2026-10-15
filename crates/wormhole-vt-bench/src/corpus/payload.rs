//! Long strings: graphics protocols and clipboard transfers between lines of text.

use super::Out;

/// Lines of text between strings that run for kilobytes: kitty graphics in 4 KB chunks, sixel,
/// OSC 52, an inline file (OSC 1337) and a very long title. The parser moves between its ground
/// state and its string states and must not buffer or leak any payload.
pub(super) fn payloads(out: &mut Out, target: usize) {
    let mut id = 0u32;
    let mut item = 0usize;
    while out.len() < target {
        let len = out.rng.range(20, 100);
        let line = out.line_of(len);
        out.text(&line);
        out.text("\r\n");
        // In turn, so that a short stream has every kind.
        item += 1;
        match item % 5 {
            0 => {
                // A kitty image sent in chunks of 4096 base64 bytes: every chunk but the last has m=1.
                id += 1;
                let chunks = out.rng.range(2, 6);
                for i in 0..chunks {
                    let more = u8::from(i + 1 < chunks);
                    let data = out.base64(3072);
                    if i == 0 {
                        out.text(&format!("\x1b_Ga=T,f=100,t=d,i={id},m={more};{data}\x1b\\"));
                    } else {
                        out.text(&format!("\x1b_Gm={more};{data}\x1b\\"));
                    }
                }
            }
            1 => {
                let (w, h) = (out.rng.range(8, 64), out.rng.range(6, 48));
                out.text(&format!("\x1bP0;0;0q\"1;1;{w};{h}#0;2;0;0;0#1;2;100;0;0"));
                for _ in 0..out.rng.range(40, 200) {
                    out.text("#1");
                    for _ in 0..w {
                        let sixel = (63 + out.rng.below(64)) as u8;
                        out.raw(&[sixel]);
                    }
                    out.text("$");
                    out.text("-");
                }
                out.text("\x1b\\");
            }
            2 => {
                let data = out.base64_between(1000, 6000);
                out.text(&format!("\x1b]52;c;{data}\x07"));
            }
            3 => {
                let data = out.base64_between(2000, 8000);
                out.text(&format!("\x1b]1337;File=inline=1;size={}:{data}\x07", data.len()));
            }
            _ => {
                let title = out.line_of(4096);
                out.text(&format!("\x1b]0;{title}\x1b\\"));
                out.text("\x1bP+q544e\x1b\\");
            }
        }
    }
}
