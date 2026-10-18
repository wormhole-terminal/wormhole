//! Device control strings and application program commands: collecting the string and sending it
//! to what answers it.
//!
//! Spec: xterm's ctlseqs. DECRQSS (`DCS $ q`, `dcs_rqss`) and XTGETTCAP (`DCS + q`, `dcs_tcap`)
//! are requests and are answered at once. `DCS q` is sixel, `DCS = 1 s` and `DCS = 2 s` begin and
//! end a synchronized update in the older iTerm2 form, and APC with a `G` is the kitty graphics
//! protocol. The image protocols are `graphics`' business; this file only collects a sixel's data
//! and passes an APC string on as it streams in.

use crate::parser::Csi;

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
}

/// Bytes as uppercase hex digits.
pub(super) struct Hex<'a>(pub(super) &'a [u8]);

impl std::fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02X}"))
    }
}
