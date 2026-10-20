//! Where a sequence the parser found goes: the private adapter between the parser's `Handler` and
//! the terminal, then the controls, ESC and CSI finals, and the collecting of DCS and APC strings.

pub(super) mod control;
pub(super) mod csi;
pub(super) mod dcs;
pub(super) mod esc;
pub(super) mod handler;
pub(super) mod vt52;
