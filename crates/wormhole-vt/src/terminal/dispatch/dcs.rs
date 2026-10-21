//! Device control strings and application program commands: collecting the string and sending it
//! to what answers it.
//!
//! Spec: xterm's ctlseqs. DECRQSS (`DCS $ q`, `dcs_rqss`) and XTGETTCAP (`DCS + q`, `dcs_tcap`)
//! are requests and are answered at once. `DCS q` is sixel, `DCS = 1 s` and `DCS = 2 s` begin and
//! end a synchronized update in the older iTerm2 form, and APC with a `G` is the kitty graphics
//! protocol. The image protocols are `graphics`' business; this file only collects a sixel's data
//! and passes an APC string on as it streams in. A DCS or APC string the host asked for with a
//! `Passthrough` is collected here whole, up to its limit, and queued as an event when it ends.

use crate::parser::Csi;

use crate::terminal::modes::Modes;
use crate::terminal::modes::conformance::Level;
use crate::terminal::{Event, Terminal};

/// Most a DECRQSS request is (the longest is two bytes).
const RQSS_MAX: usize = 32;
/// Most an XTGETTCAP request is.
const TCAP_MAX: usize = 4096;
/// Most a Sixel's data is; it is decoded when the string ends, on the parser's thread.
pub(in crate::terminal) const SIXEL_MAX: usize = 4 * 1024 * 1024;

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
    /// A string the host asked for, collected up to its limit: a DCS (the header is in
    /// `DcsString::header`) or an APC (its first byte is part of the data).
    PassDcs,
    PassApc,
    /// An APC string whose first byte has not come.
    ApcFirst,
    /// A kitty graphics command, streamed to `graphics`.
    Apc,
    /// Read and thrown away.
    Ignore,
}

/// The string being read, whichever sort it is.
pub(in crate::terminal) struct DcsString {
    kind: Kind,
    data: Vec<u8>,
    /// The limit on a Sixel's data: `SIXEL_MAX`, or what the host lowered it to.
    sixel_max: usize,
    /// The numbers before a sixel's `q`.
    params: Vec<u16>,
    /// The limit on `data` for a string the host asked for.
    pass_limit: usize,
    /// The header of a DCS the host asked for, kept for the event at its end.
    header: Option<Csi>,
}

impl Default for DcsString {
    fn default() -> DcsString {
        DcsString {
            kind: Kind::None,
            data: Vec::new(),
            sixel_max: SIXEL_MAX,
            params: Vec::new(),
            pass_limit: 0,
            header: None,
        }
    }
}

impl DcsString {
    pub(in crate::terminal) fn set_sixel_max(&mut self, max: usize) {
        self.sixel_max = max;
    }

    pub(in crate::terminal) fn cancel(&mut self) {
        self.kind = Kind::None;
        self.data = Vec::new();
    }

    pub(in crate::terminal) fn put(&mut self, data: &[u8]) {
        let limit = match self.kind {
            Kind::Decrqss => RQSS_MAX,
            Kind::Xtgettcap => TCAP_MAX,
            Kind::Sixel => self.sixel_max,
            Kind::PassDcs => self.pass_limit,
            Kind::Preferred(_) => RQSS_MAX,
            Kind::RestorePresentation(_) | Kind::RestoreState(_) => {
                crate::terminal::reports::presentation::RESTORE_MAX
            }
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

    pub(in crate::terminal) fn start_apc(&mut self) {
        self.kind = Kind::ApcFirst;
        self.data = Vec::new();
    }
}

impl Terminal {
    pub(in crate::terminal) fn hook_dcs(&mut self, csi: &Csi) {
        self.dcs.data.clear();
        if let Some(limit) = self.passthrough.dcs_cap(csi) {
            self.dcs.pass_limit = limit;
            self.dcs.header = Some(csi.clone());
            self.dcs.kind = Kind::PassDcs;
            return;
        }
        self.dcs.kind = match (csi.marker(), csi.intermediates(), csi.final_byte()) {
            // DECRQSS is a VT320 feature (VT510 manual; vttest: "VT340 implements DECRQSS"): a terminal
            // at the VT100 or VT200 level, as ESC < leaves VT52 mode, does not recognise it.
            (None, [b'$'], b'q') if self.level >= Level::Vt300 => Kind::Decrqss,
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

    pub(in crate::terminal) fn unhook_dcs(&mut self) {
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
            Kind::PassDcs => {
                // The event owns the data, so the buffer goes with it.
                let payload = std::mem::take(&mut self.dcs.data);
                if let Some(header) = self.dcs.header.take() {
                    self.push_event(Event::Dcs {
                        params: header.values().to_vec(),
                        intermediates: header.intermediates().to_vec(),
                        final_byte: header.final_byte(),
                        payload,
                        replies_before: self.replies.len(),
                    });
                }
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
    pub(in crate::terminal) fn put_apc(&mut self, data: &[u8]) {
        if self.dcs.kind == Kind::ApcFirst {
            match data.first() {
                None => return,
                Some(b'G') => {
                    self.dcs.kind = Kind::Apc;
                    self.begin_kitty();
                    self.put_kitty(&data[1..]);
                    return;
                }
                Some(&first) => {
                    self.dcs.kind = match self.passthrough.apc_cap(first) {
                        Some(limit) => {
                            self.dcs.pass_limit = limit;
                            Kind::PassApc
                        }
                        None => Kind::Ignore,
                    };
                }
            }
        }
        match self.dcs.kind {
            Kind::Apc => self.put_kitty(data),
            Kind::PassApc => self.dcs.append(data, self.dcs.pass_limit),
            _ => {}
        }
    }

    pub(in crate::terminal) fn finish_apc(&mut self) {
        match std::mem::take(&mut self.dcs.kind) {
            Kind::Apc => self.finish_kitty(),
            Kind::PassApc => {
                let payload = std::mem::take(&mut self.dcs.data);
                self.push_event(Event::Apc { payload, replies_before: self.replies.len() });
            }
            _ => {}
        }
    }

    pub(in crate::terminal) fn cancel_apc(&mut self) {
        if self.dcs.kind == Kind::Apc {
            self.cancel_kitty();
        }
        self.dcs.cancel();
    }
}

#[cfg(test)]
mod tests {
    use crate::terminal::Terminal;

    /// A Sixel of nothing but carriage returns: `n` bytes of data that paint no pixel.
    fn blank_sixel(n: usize) -> Vec<u8> {
        let mut bytes = b"\x1bPq".to_vec();
        bytes.extend(std::iter::repeat_n(b'$', n));
        bytes.extend_from_slice(b"\x1b\\");
        bytes
    }

    #[test]
    fn the_buffer_of_a_picture_is_kept_for_the_next_unless_it_grew_big() {
        let mut term = Terminal::new(40, 10);
        term.feed(&blank_sixel(20_000));
        assert!(term.dcs.data.capacity() >= 20_000, "kept: the next picture needs no new buffer");
        term.feed(&blank_sixel(100_000));
        assert_eq!(term.dcs.data.capacity(), 0, "past 64 KiB it is given back");
    }

    #[test]
    fn a_cancelled_dcs_leaves_nothing_open_and_gives_its_buffer_back() {
        let mut term = Terminal::new(40, 10);
        let mut sixel = b"\x1bPq".to_vec();
        sixel.extend(std::iter::repeat_n(b'$', 5000));
        term.feed(&sixel);
        assert_eq!(term.dcs.kind, super::Kind::Sixel);
        assert!(term.dcs.data.capacity() >= 5000);
        // CAN ends the string without acting on it.
        term.feed(b"\x18");
        assert_eq!(term.dcs.kind, super::Kind::None);
        assert_eq!(term.dcs.data.capacity(), 0);
    }

    #[test]
    fn a_cancelled_apc_leaves_nothing_open() {
        let mut term = Terminal::new(40, 10);
        for (start, open) in [(&b"\x1b_"[..], super::Kind::ApcFirst), (b"\x1b_Ga=q,i=1", super::Kind::Apc)] {
            term.feed(start);
            assert_eq!(term.dcs.kind, open);
            term.feed(b"\x1a");
            assert_eq!(term.dcs.kind, super::Kind::None, "{open:?} cancelled");
        }
    }
}
