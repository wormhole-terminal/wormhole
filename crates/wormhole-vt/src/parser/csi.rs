//! The parameters of a control sequence (and of a DCS header, which is written the same way).
//!
//! Nothing here allocates: the values live in an array in the parser, and the handler is given a
//! reference to it. Which values are sub-parameters and which were left empty is kept in two bit
//! masks next to the array, so a group is found with a shift and a count of trailing ones.

use std::fmt;

/// Parameter values kept, sub-parameters included. ECMA-48 5.4.2 sets no limit; Williams' parser
/// notes that a terminal stores a fixed number and ignores the rest, and xterm's SGR 38:2 with a
/// colour space and tolerances needs 8, so 32 is far past anything a program sends.
pub const MAX_PARAMS: usize = 32;

/// Intermediate bytes kept. Real sequences have at most two (`CSI $ ' z`); Williams has `collect`
/// flag a sequence with more so that its dispatch does nothing.
pub const MAX_INTERMEDIATES: usize = 4;

/// A control sequence, or a DCS header: an optional private marker, parameters, intermediates and
/// a final byte.
///
/// Parameters are read as ECMA-48 5.4.2 writes them: `;` separates parameters, an empty one is a
/// default, and ITU-T T.416 §13.1.8 (the SGR colour forms `38:2::r:g:b`) uses `:` inside a
/// parameter. [`values`](Csi::values) is flat; [`groups`](Csi::groups) and the accessors that take
/// an index count parameters, each with its colon-separated sub-parameters.
#[derive(Clone)]
pub struct Csi {
    values: [u16; MAX_PARAMS],
    /// Bit k: value k follows a `:`, so it belongs to the group before it. Bit 0 is never set.
    sub: u32,
    /// Bit k: value k had no digits (it reads 0, `get` says `None`).
    empty: u32,
    count: u8,
    inter: [u8; MAX_INTERMEDIATES],
    inter_len: u8,
    /// 0 when absent, else one of `<=>?`.
    marker: u8,
    final_byte: u8,
    overflow: bool,
}

impl Csi {
    pub(super) const fn new() -> Csi {
        Csi {
            values: [0; MAX_PARAMS],
            sub: 0,
            empty: 0,
            count: 0,
            inter: [0; MAX_INTERMEDIATES],
            inter_len: 0,
            marker: 0,
            final_byte: 0,
            overflow: false,
        }
    }

    // ---- building, by the parser ----

    /// Start a new sequence. Stale values past `count` are never read, so they stay.
    #[inline(always)]
    pub(super) fn clear(&mut self) {
        self.sub = 0;
        self.empty = 0;
        self.count = 0;
        self.inter_len = 0;
        self.marker = 0;
        self.overflow = false;
    }

    /// Close the parameter being read: `value` is saturated to 65535 by the caller, `digits` says
    /// whether it had any. A value past the limit sets `overflow` and is dropped.
    #[inline(always)]
    pub(super) fn push(&mut self, value: u32, digits: bool) {
        let k = self.count as usize;
        if k < MAX_PARAMS {
            self.values[k] = value as u16;
            self.empty |= u32::from(!digits) << k;
            self.count += 1;
        } else {
            self.overflow = true;
        }
    }

    /// A `:` after `push`: the next value is a sub-parameter of the one just closed.
    #[inline(always)]
    pub(super) fn start_sub(&mut self) {
        let k = self.count as usize;
        if k < MAX_PARAMS {
            self.sub |= 1 << k;
        }
    }

    /// An intermediate byte; the fifth and later only set `overflow`.
    #[inline(always)]
    pub(super) fn collect(&mut self, byte: u8) {
        let k = self.inter_len as usize;
        if k < MAX_INTERMEDIATES {
            self.inter[k] = byte;
            self.inter_len += 1;
        } else {
            self.overflow = true;
        }
    }

    #[inline(always)]
    pub(super) fn set_marker(&mut self, byte: u8) {
        self.marker = byte;
    }

    /// True while nothing has been read: a private marker is only one at the very start of the
    /// parameter string (ECMA-48 5.4.1: its first bit combination decides the format).
    #[inline(always)]
    pub(super) fn at_start(&self, digits: bool) -> bool {
        self.count == 0 && self.marker == 0 && !digits
    }

    #[inline(always)]
    pub(super) fn set_final(&mut self, byte: u8) {
        self.final_byte = byte;
    }

    // ---- reading, by the handler ----

    /// The private marker, one of `<=>?`, when the parameter string starts with one.
    pub fn marker(&self) -> Option<u8> {
        (self.marker != 0).then_some(self.marker)
    }

    /// The intermediate bytes (0x20..=0x2F), at most [`MAX_INTERMEDIATES`].
    pub fn intermediates(&self) -> &[u8] {
        &self.inter[..self.inter_len as usize]
    }

    /// The final byte, 0x40..=0x7E.
    pub fn final_byte(&self) -> u8 {
        self.final_byte
    }

    /// True when the sequence had more than [`MAX_PARAMS`] values or [`MAX_INTERMEDIATES`]
    /// intermediates: the first of each are kept, the rest dropped.
    pub fn overflow(&self) -> bool {
        self.overflow
    }

    /// The number of parameters (groups), at least 1 in every sequence the parser reports: one
    /// without any has a single empty one (ECMA-48 5.4.2: a missing parameter string is one
    /// default). [`is_empty`](Csi::is_empty) is then always false; it is here for the pair's sake.
    pub fn len(&self) -> usize {
        self.count as usize - self.sub.count_ones() as usize
    }

    /// True when there are no parameters at all; see [`len`](Csi::len).
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Every value, sub-parameters included, in order. An empty value reads 0.
    pub fn values(&self) -> &[u16] {
        &self.values[..self.count as usize]
    }

    /// True when any parameter has colon-separated sub-parameters.
    pub fn has_subparams(&self) -> bool {
        self.sub != 0
    }

    /// Bit k set for the values that start a parameter.
    #[inline(always)]
    fn starts(&self) -> u32 {
        let live = ((1u64 << self.count) - 1) as u32;
        !self.sub & live
    }

    /// The index in `values` of the first value of parameter `i`.
    #[inline(always)]
    fn start_of(&self, i: usize) -> Option<usize> {
        if self.sub == 0 {
            return (i < self.count as usize).then_some(i);
        }
        let mut starts = self.starts();
        if i >= starts.count_ones() as usize {
            return None;
        }
        for _ in 0..i {
            starts &= starts - 1; // drop the lowest start
        }
        Some(starts.trailing_zeros() as usize)
    }

    /// Parameter `i`; `None` when it is empty or absent. For a parameter with sub-parameters, the
    /// first value.
    #[inline]
    pub fn get(&self, i: usize) -> Option<u16> {
        let k = self.start_of(i)?;
        (self.empty >> k & 1 == 0).then(|| self.values[k])
    }

    /// Parameter `i`, or `default` when it is empty or absent. An explicit 0 stays 0.
    #[inline]
    pub fn param(&self, i: usize, default: u16) -> u16 {
        self.get(i).unwrap_or(default)
    }

    /// Parameter `i`, or `default` when it is empty, absent or 0: ECMA-48 8.3 gives most movement
    /// and editing functions a default that 0 selects as well.
    #[inline]
    pub fn param_nonzero(&self, i: usize, default: u16) -> u16 {
        match self.get(i) {
            Some(v) if v != 0 => v,
            _ => default,
        }
    }

    /// The parameters, each as its value followed by its sub-parameters.
    pub fn groups(&self) -> Groups<'_> {
        Groups { csi: self, at: 0 }
    }
}

impl PartialEq for Csi {
    /// By what the sequence says; the stale array beyond `count` is not part of it.
    fn eq(&self, other: &Csi) -> bool {
        self.values() == other.values()
            && self.sub == other.sub
            && self.empty == other.empty
            && self.marker == other.marker
            && self.intermediates() == other.intermediates()
            && self.final_byte == other.final_byte
            && self.overflow == other.overflow
    }
}

impl Eq for Csi {}

/// The sequence as typed, for test failures and logs: `CSI ?1;38:2::1 SP q`. A lone empty
/// parameter is left out (`CSI m`), a space intermediate is written `SP`.
impl fmt::Debug for Csi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CSI")?;
        let lone_empty = self.count == 1 && self.empty & 1 == 1;
        if self.marker != 0 || !lone_empty {
            f.write_str(" ")?;
            if self.marker != 0 {
                write!(f, "{}", self.marker as char)?;
            }
            for k in 0..self.count as usize {
                if k > 0 {
                    f.write_str(if self.sub >> k & 1 == 1 { ":" } else { ";" })?;
                }
                if self.empty >> k & 1 == 0 {
                    write!(f, "{}", self.values[k])?;
                }
            }
        }
        for &byte in self.intermediates() {
            if byte == b' ' {
                f.write_str(" SP")?;
            } else {
                write!(f, " {}", byte as char)?;
            }
        }
        write!(f, " {}", self.final_byte as char)
    }
}

/// The parameters of a [`Csi`], one slice each: the value and its sub-parameters.
#[derive(Clone)]
pub struct Groups<'a> {
    csi: &'a Csi,
    at: usize,
}

impl<'a> Iterator for Groups<'a> {
    type Item = &'a [u16];

    #[inline]
    fn next(&mut self) -> Option<&'a [u16]> {
        let csi = self.csi;
        if self.at >= csi.count as usize {
            return None;
        }
        // The group runs over the following values whose bit is set in `sub`.
        let ones = (!(u64::from(csi.sub) >> (self.at + 1))).trailing_zeros() as usize;
        let end = self.at + 1 + ones;
        let group = &csi.values[self.at..end];
        self.at = end;
        Some(group)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rest = self.csi.starts() >> self.at.min(31);
        let n = if self.at >= self.csi.count as usize { 0 } else { rest.count_ones() as usize };
        (n, Some(n))
    }
}

impl ExactSizeIterator for Groups<'_> {}
