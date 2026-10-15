//! A plain parser to check the real one against, written straight from Paul Williams' state tables
//! (vt100.net/emu/dec_ansi_parser): one state, one byte at a time, an entry and an exit action per
//! state, no fast paths and no streaming (a string's payload is collected in a `Vec`). It shares
//! nothing with `Parser` but the deviations from the tables that `parser/mod.rs` lists, so where
//! the two agree on random input, the fast paths do what the tables say.
//!
//! UTF-8 is decoded byte by byte with the ranges of the Unicode Standard's Table 3-7, apart from
//! `utf8.rs`, which decodes whole sequences.

use super::{End, end_name, payload, text};

/// Williams' states. His CSI Entry and DCS Entry are here, so that the marker rule is his
/// (a private marker is valid in Entry only) and not the "nothing read yet" test the parser uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum S {
    Ground,
    Escape,
    EscapeIntermediate,
    CsiEntry,
    CsiParam,
    CsiIntermediate,
    CsiIgnore,
    DcsEntry,
    DcsParam,
    DcsIntermediate,
    DcsPassthrough,
    DcsIgnore,
    OscString,
    SosPmString,
    ApcString,
}

/// What the reference reports. A sequence is already its text, as `Csi`'s `Debug` writes it.
#[derive(Clone, Debug, PartialEq)]
pub enum RefEvent {
    Print(char),
    Execute(u8),
    /// The sequence as typed (with the limits applied) and whether it overflowed.
    Csi(String, bool),
    Esc(Vec<u8>, u8),
    Osc(Vec<u8>, End),
    Dcs(String, bool, Vec<u8>, End),
    Apc(Vec<u8>, End),
}

/// A UTF-8 character in progress: the code point so far, how many continuation bytes are still
/// due, and the range of the next one (Table 3-7: only the second byte's range varies).
#[derive(Clone, Copy)]
struct Utf8 {
    cp: u32,
    due: u8,
    lo: u8,
    hi: u8,
}

pub struct Reference {
    state: S,
    /// An ESC that ended a string, or interrupted a DCS, is followed by `\` as its ST.
    after_string: bool,
    utf8: Option<Utf8>,
    events: Vec<RefEvent>,
    // The collected sequence (Williams: private flag, intermediate characters, parameters).
    marker: Option<u8>,
    intermediates: Vec<u8>,
    /// Closed parameter values: (value, empty, follows a colon).
    values: Vec<(u32, bool, bool)>,
    current: Option<u32>,
    next_is_sub: bool,
    // The string being read.
    data: Vec<u8>,
    hook: Option<(String, bool)>,
}

impl Reference {
    pub fn new() -> Reference {
        Reference {
            state: S::Ground,
            after_string: false,
            utf8: None,
            events: Vec::new(),
            marker: None,
            intermediates: Vec::new(),
            values: Vec::new(),
            current: None,
            next_is_sub: false,
            data: Vec::new(),
            hook: None,
        }
    }

    pub fn feed_all(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.step(b);
        }
    }

    /// The events so far, with a string still open reported as `End::Open`.
    pub fn finish(mut self) -> Vec<RefEvent> {
        match self.state {
            S::OscString => self.events.push(RefEvent::Osc(std::mem::take(&mut self.data), End::Open)),
            S::ApcString => self.events.push(RefEvent::Apc(std::mem::take(&mut self.data), End::Open)),
            S::DcsPassthrough => {
                let (text, overflow) = self.hook.take().expect("hooked");
                self.events.push(RefEvent::Dcs(text, overflow, std::mem::take(&mut self.data), End::Open));
            }
            _ => {}
        }
        self.events
    }

    // ---- actions ----

    fn clear(&mut self) {
        self.marker = None;
        self.intermediates.clear();
        self.values.clear();
        self.current = None;
        self.next_is_sub = false;
    }

    fn collect(&mut self, b: u8) {
        self.intermediates.push(b);
    }

    fn digit(&mut self, b: u8) {
        let d = u32::from(b - b'0');
        self.current = Some((self.current.unwrap_or(0) * 10 + d).min(65535));
    }

    /// Close the parameter being read (`;`, `:`, or the end of the parameters).
    fn close_value(&mut self) {
        let value = self.current.take();
        self.values.push((value.unwrap_or(0), value.is_none(), self.next_is_sub));
        self.next_is_sub = false;
    }

    /// The sequence as `Csi`'s `Debug` writes it, under the limits of 32 values and 4 intermediates.
    fn text(&mut self, final_byte: u8) -> (String, bool) {
        self.close_value();
        let overflow = self.values.len() > 32 || self.intermediates.len() > 4;
        let mut params = String::new();
        for (k, &(value, empty, sub)) in self.values.iter().take(32).enumerate() {
            if k > 0 {
                params.push(if sub { ':' } else { ';' });
            }
            if !empty {
                params += &value.to_string();
            }
        }
        let mut out = String::from("CSI");
        if self.marker.is_some() || !params.is_empty() {
            out.push(' ');
            out.extend(self.marker.map(|m| m as char));
            out += &params;
        }
        for &i in self.intermediates.iter().take(4) {
            out += &if i == b' ' { " SP".to_string() } else { format!(" {}", i as char) };
        }
        out += &format!(" {}", final_byte as char);
        (out, overflow)
    }

    fn csi_dispatch(&mut self, b: u8) {
        let (text, overflow) = self.text(b);
        self.events.push(RefEvent::Csi(text, overflow));
    }

    fn esc_dispatch(&mut self, b: u8) {
        // More than four intermediates: the sequence does nothing.
        if self.intermediates.len() <= 4 {
            self.events.push(RefEvent::Esc(self.intermediates.clone(), b));
        }
    }

    /// DCS Passthrough's entry action.
    fn hook(&mut self, b: u8) {
        self.hook = Some(self.text(b));
        self.data.clear();
    }

    fn end_string(&mut self, end: End) {
        match self.state {
            S::OscString => self.events.push(RefEvent::Osc(std::mem::take(&mut self.data), end)),
            S::ApcString => self.events.push(RefEvent::Apc(std::mem::take(&mut self.data), end)),
            S::DcsPassthrough => {
                let (text, overflow) = self.hook.take().expect("hooked");
                self.events.push(RefEvent::Dcs(text, overflow, std::mem::take(&mut self.data), end));
            }
            _ => {}
        }
    }

    fn goto(&mut self, state: S) {
        // Entry actions.
        match state {
            S::Escape | S::CsiEntry | S::DcsEntry => self.clear(),
            S::OscString | S::ApcString => self.data.clear(),
            _ => {}
        }
        self.state = state;
    }

    fn print(&mut self, c: char) {
        self.events.push(RefEvent::Print(c));
    }

    // ---- the table ----

    fn step(&mut self, b: u8) {
        if self.state == S::Ground && self.utf8.is_some() && self.continue_utf8(b) {
            return;
        }
        // The rows of Williams' "anywhere": CAN and SUB execute and go to ground, ESC starts over.
        // (The 8-bit controls 0x80..=0x9F are not controls here: see `parser/mod.rs`.)
        match b {
            0x18 | 0x1A => {
                // A string that is cancelled is told so; a header or a plain sequence just stops.
                self.end_string(End::Cancel);
                self.events.push(RefEvent::Execute(b));
                self.goto(S::Ground);
                return;
            }
            0x1B => {
                self.end_string(End::St);
                // An ESC that ends a string, or interrupts a DCS, is followed by its ST.
                self.after_string = matches!(
                    self.state,
                    S::OscString
                        | S::ApcString
                        | S::SosPmString
                        | S::DcsPassthrough
                        | S::DcsIgnore
                        | S::DcsEntry
                        | S::DcsParam
                        | S::DcsIntermediate
                );
                self.goto(S::Escape);
                return;
            }
            _ => {}
        }
        match self.state {
            S::Ground => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x7E => self.print(b as char),
                0x7F => {}
                _ => self.start_utf8(b),
            },
            S::Escape => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x2F => {
                    self.after_string = false;
                    self.collect(b);
                    self.goto(S::EscapeIntermediate);
                }
                0x5B => {
                    self.after_string = false;
                    self.goto(S::CsiEntry);
                }
                0x50 => {
                    self.after_string = false;
                    self.goto(S::DcsEntry);
                }
                0x5D => {
                    self.after_string = false;
                    self.goto(S::OscString);
                }
                0x58 | 0x5E => {
                    self.after_string = false;
                    self.goto(S::SosPmString);
                }
                0x5F => {
                    self.after_string = false;
                    self.goto(S::ApcString);
                }
                0x5C if self.after_string => {
                    self.after_string = false;
                    self.goto(S::Ground);
                }
                0x30..=0x4F | 0x51..=0x57 | 0x59 | 0x5A | 0x5C | 0x60..=0x7E => {
                    self.after_string = false;
                    self.esc_dispatch(b);
                    self.goto(S::Ground);
                }
                _ => {} // 0x7F, and bytes of 0x80 and more
            },
            S::EscapeIntermediate => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x2F => self.collect(b),
                0x30..=0x7E => {
                    self.esc_dispatch(b);
                    self.goto(S::Ground);
                }
                _ => {}
            },
            S::CsiEntry => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x2F => {
                    self.collect(b);
                    self.goto(S::CsiIntermediate);
                }
                0x30..=0x39 => {
                    self.digit(b);
                    self.state = S::CsiParam;
                }
                0x3A => {
                    self.close_value();
                    self.next_is_sub = true;
                    self.state = S::CsiParam;
                }
                0x3B => {
                    self.close_value();
                    self.state = S::CsiParam;
                }
                0x3C..=0x3F => {
                    self.marker = Some(b);
                    self.state = S::CsiParam;
                }
                0x40..=0x7E => {
                    self.csi_dispatch(b);
                    self.goto(S::Ground);
                }
                _ => {}
            },
            S::CsiParam => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x2F => {
                    self.collect(b);
                    self.goto(S::CsiIntermediate);
                }
                0x30..=0x39 => self.digit(b),
                0x3A => {
                    self.close_value();
                    self.next_is_sub = true;
                }
                0x3B => self.close_value(),
                0x3C..=0x3F => self.goto(S::CsiIgnore),
                0x40..=0x7E => {
                    self.csi_dispatch(b);
                    self.goto(S::Ground);
                }
                _ => {}
            },
            S::CsiIntermediate => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x20..=0x2F => self.collect(b),
                0x30..=0x3F => self.goto(S::CsiIgnore),
                0x40..=0x7E => {
                    self.csi_dispatch(b);
                    self.goto(S::Ground);
                }
                _ => {}
            },
            S::CsiIgnore => match b {
                0x00..=0x17 | 0x19 | 0x1C..=0x1F => self.events.push(RefEvent::Execute(b)),
                0x40..=0x7E => self.goto(S::Ground),
                _ => {}
            },
            // The DCS header: controls are ignored, not executed.
            S::DcsEntry => match b {
                0x20..=0x2F => {
                    self.collect(b);
                    self.goto(S::DcsIntermediate);
                }
                0x30..=0x39 => {
                    self.digit(b);
                    self.state = S::DcsParam;
                }
                0x3A => {
                    self.close_value();
                    self.next_is_sub = true;
                    self.state = S::DcsParam;
                }
                0x3B => {
                    self.close_value();
                    self.state = S::DcsParam;
                }
                0x3C..=0x3F => {
                    self.marker = Some(b);
                    self.state = S::DcsParam;
                }
                0x40..=0x7E => self.start_data(b),
                _ => {}
            },
            S::DcsParam => match b {
                0x20..=0x2F => {
                    self.collect(b);
                    self.goto(S::DcsIntermediate);
                }
                0x30..=0x39 => self.digit(b),
                0x3A => {
                    self.close_value();
                    self.next_is_sub = true;
                }
                0x3B => self.close_value(),
                0x3C..=0x3F => self.goto(S::DcsIgnore),
                0x40..=0x7E => self.start_data(b),
                _ => {}
            },
            S::DcsIntermediate => match b {
                0x20..=0x2F => self.collect(b),
                0x30..=0x3F => self.goto(S::DcsIgnore),
                0x40..=0x7E => self.start_data(b),
                _ => {}
            },
            // `put` takes everything but DEL (and ESC, CAN and SUB, handled above).
            S::DcsPassthrough | S::ApcString => {
                if b != 0x7F {
                    self.data.push(b);
                }
            }
            // The text of an OSC: no C0 controls but BEL, which ends it. DEL is text.
            S::OscString => match b {
                0x07 => {
                    self.end_string(End::Bel);
                    self.goto(S::Ground);
                }
                0x00..=0x1F => {}
                _ => self.data.push(b),
            },
            S::DcsIgnore | S::SosPmString => {}
        }
    }

    /// DCS Passthrough, entered by the final byte of the header.
    fn start_data(&mut self, b: u8) {
        self.hook(b);
        self.goto(S::DcsPassthrough);
    }

    // ---- UTF-8, Table 3-7, a byte at a time ----

    fn start_utf8(&mut self, b: u8) {
        let (due, lo, hi, cp) = match b {
            0xC2..=0xDF => (1, 0x80, 0xBF, b & 0x1F),
            0xE0 => (2, 0xA0, 0xBF, b & 0x0F),
            0xE1..=0xEC | 0xEE | 0xEF => (2, 0x80, 0xBF, b & 0x0F),
            0xED => (2, 0x80, 0x9F, b & 0x0F),
            0xF0 => (3, 0x90, 0xBF, b & 0x07),
            0xF1..=0xF3 => (3, 0x80, 0xBF, b & 0x07),
            0xF4 => (3, 0x80, 0x8F, b & 0x07),
            // A stray continuation byte, 0xC0, 0xC1 and 0xF5..: never the start of anything.
            _ => {
                self.print('\u{FFFD}');
                return;
            }
        };
        self.utf8 = Some(Utf8 { cp: u32::from(cp), due, lo, hi });
    }

    /// Feed `b` to the character in progress. True when `b` was part of it; false when it broke
    /// the character (a replacement is printed) and is to be read again as the start of something.
    fn continue_utf8(&mut self, b: u8) -> bool {
        let Some(mut u) = self.utf8.take() else { return false };
        if !(u.lo..=u.hi).contains(&b) {
            self.print('\u{FFFD}');
            return false;
        }
        u.cp = u.cp << 6 | u32::from(b & 0x3F);
        u.due -= 1;
        if u.due == 0 {
            self.print(char::from_u32(u.cp).expect("Table 3-7 allows scalar values only"));
        } else {
            u.lo = 0x80;
            u.hi = 0xBF;
            self.utf8 = Some(u);
        }
        true
    }
}

/// The events in the notation of `super::show`.
pub fn show_ref(events: &[RefEvent]) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = String::new();
    for event in events {
        if let RefEvent::Print(c) = event {
            run.push(*c);
            continue;
        }
        if !run.is_empty() {
            out.push(text(&std::mem::take(&mut run)));
        }
        out.push(match event {
            RefEvent::Print(_) => unreachable!(),
            RefEvent::Execute(b) => format!("exec {b:02x}"),
            RefEvent::Csi(csi, _) => csi.clone(),
            RefEvent::Esc(inter, b) => {
                let mut s = String::from("ESC");
                for &i in inter {
                    s += &if i == b' ' { " SP".to_string() } else { format!(" {}", i as char) };
                }
                s += &format!(" {}", *b as char);
                s
            }
            RefEvent::Osc(data, end) => format!("OSC {} {}", payload(data), end_name(*end)),
            RefEvent::Dcs(csi, _, data, end) => {
                format!("{} {} {}", csi.replacen("CSI", "DCS", 1), payload(data), end_name(*end))
            }
            RefEvent::Apc(data, end) => format!("APC {} {}", payload(data), end_name(*end)),
        });
    }
    if !run.is_empty() {
        out.push(text(&run));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::property::{Rng, gen_noise, gen_stream};
    use super::super::{Event, run};
    use super::{RefEvent, Reference};

    #[test]
    fn the_reference_reads_the_tables() {
        let reference = |bytes: &[u8]| {
            let mut r = Reference::new();
            r.feed_all(bytes);
            super::show_ref(&r.finish())
        };
        assert_eq!(reference(b"a\x1b[1;2Hb"), ["\"a\"", "CSI 1;2 H", "\"b\""]);
        assert_eq!(reference(b"\x1b]0;t\x07"), ["OSC \"0;t\" BEL"]);
        assert_eq!(reference(b"\x1bP1q#\x1b\\x"), ["DCS 1 q \"#\" ST", "\"x\""]);
        assert_eq!(reference("é\u{FFFD}".as_bytes()), ["\"é\u{FFFD}\""]);
        assert_eq!(reference(b"\x1b[?1 2h"), Vec::<String>::new());
    }

    /// `show` has the text of a `Csi`; whether it overflowed is checked here, in stream order.
    #[test]
    fn overflow_flags_agree_with_the_reference() {
        let mut rng = Rng::new(0x0F10_0001);
        let mut seen = 0;
        for round in 0..4000 {
            let bytes = if round % 5 == 0 { gen_noise(&mut rng) } else { gen_stream(&mut rng) };
            let mut reference = Reference::new();
            reference.feed_all(&bytes);
            let want: Vec<bool> = reference
                .finish()
                .iter()
                .filter_map(|e| match e {
                    RefEvent::Csi(_, overflow) | RefEvent::Dcs(_, overflow, ..) => Some(*overflow),
                    _ => None,
                })
                .collect();
            let got: Vec<bool> = run(&bytes)
                .iter()
                .filter_map(|e| match e {
                    Event::Csi(csi) | Event::Dcs(csi, ..) => Some(csi.overflow()),
                    _ => None,
                })
                .collect();
            assert_eq!(got, want, "input {bytes:?}");
            seen += want.iter().filter(|&&o| o).count();
        }
        assert!(seen > 20, "the generator should produce overflowing sequences ({seen})");
    }
}
