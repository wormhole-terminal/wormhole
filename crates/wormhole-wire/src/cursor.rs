//! Bounds-checked reading and appending of the primitive fields, and the tag-length-value tail
//! every non-bulk message ends with.
//!
//! Layout rules, all little-endian: integers fixed width; a string is `u16` length and UTF-8; a
//! blob that may be large is `u32` length and bytes; an optional or future field is a TLV
//! (`tag: u8, len: u16, value`) in the tail. A reader skips tags it does not know, which is how a
//! newer minor adds fields without breaking an older reader.

use crate::error::{EncodeError, WireError};

/// A reader over one frame's payload. Every method checks its bounds first and names the message
/// in the error; none can panic.
pub(crate) struct Reader<'a> {
    b: &'a [u8],
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8], what: &'static str) -> Self {
        Self { b, what }
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if self.b.len() < n {
            return Err(WireError::Truncated(self.what));
        }
        let (head, rest) = self.b.split_at(n);
        self.b = rest;
        Ok(head)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub fn u8(&mut self) -> Result<u8, WireError> {
        Ok(self.array::<1>()?[0])
    }
    pub fn u16(&mut self) -> Result<u16, WireError> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32, WireError> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64, WireError> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    pub fn i32(&mut self) -> Result<i32, WireError> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    pub fn bool(&mut self) -> Result<bool, WireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WireError::BadValue(self.what)),
        }
    }
    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N], WireError> {
        self.array()
    }
    pub fn skip(&mut self, n: usize) -> Result<(), WireError> {
        self.take(n).map(|_| ())
    }

    pub fn str16(&mut self) -> Result<&'a str, WireError> {
        let n = usize::from(self.u16()?);
        let bytes = self.take(n)?;
        std::str::from_utf8(bytes).map_err(|_| WireError::BadUtf8(self.what))
    }

    pub fn bytes32(&mut self) -> Result<&'a [u8], WireError> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    /// Everything not read yet.
    pub fn rest(&mut self) -> &'a [u8] {
        std::mem::take(&mut self.b)
    }

    pub fn remaining(&self) -> usize {
        self.b.len()
    }

    /// The tail, as TLVs.
    pub fn tlvs(&mut self) -> Tlvs<'a> {
        Tlvs { r: Reader { b: self.rest(), what: self.what } }
    }
}

/// The TLVs of a tail, in order. A truncated one ends the iteration with an error.
pub(crate) struct Tlvs<'a> {
    r: Reader<'a>,
}

impl<'a> Iterator for Tlvs<'a> {
    type Item = Result<(u8, &'a [u8]), WireError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.r.remaining() == 0 {
            return None;
        }
        let item = (|| {
            let tag = self.r.u8()?;
            let len = usize::from(self.r.u16()?);
            Ok((tag, self.r.take(len)?))
        })();
        if item.is_err() {
            self.r.rest();
        }
        Some(item)
    }
}

/// Which known tags a tail has shown, so a repeated one is an error and not "last wins".
#[derive(Default)]
pub(crate) struct Seen(u64);

impl Seen {
    pub fn once(&mut self, tag: u8, what: &'static str) -> Result<(), WireError> {
        let bit = 1u64 << (tag & 63);
        if self.0 & bit != 0 {
            return Err(WireError::DuplicateField(what));
        }
        self.0 |= bit;
        Ok(())
    }
}

/// Appends to the caller's buffer.
pub(crate) struct Writer<'v> {
    v: &'v mut Vec<u8>,
}

impl<'v> Writer<'v> {
    pub fn new(v: &'v mut Vec<u8>) -> Self {
        Self { v }
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }

    pub fn u8(&mut self, x: u8) {
        self.v.push(x);
    }
    pub fn u16(&mut self, x: u16) {
        self.v.extend_from_slice(&x.to_le_bytes());
    }
    pub fn u32(&mut self, x: u32) {
        self.v.extend_from_slice(&x.to_le_bytes());
    }
    pub fn u64(&mut self, x: u64) {
        self.v.extend_from_slice(&x.to_le_bytes());
    }
    pub fn i32(&mut self, x: i32) {
        self.v.extend_from_slice(&x.to_le_bytes());
    }
    pub fn bool(&mut self, x: bool) {
        self.v.push(u8::from(x));
    }
    pub fn raw(&mut self, b: &[u8]) {
        self.v.extend_from_slice(b);
    }
    /// Overwrite four bytes written earlier at `at` (a length that was not known yet).
    pub fn patch_u32(&mut self, at: usize, x: u32) {
        self.v[at..at + 4].copy_from_slice(&x.to_le_bytes());
    }
    pub fn zeros(&mut self, n: usize) {
        self.v.resize(self.v.len() + n, 0);
    }

    pub fn str16(&mut self, field: &'static str, s: &str) -> Result<(), EncodeError> {
        let n = u16::try_from(s.len()).map_err(|_| EncodeError::StringTooLong(field))?;
        self.u16(n);
        self.raw(s.as_bytes());
        Ok(())
    }

    pub fn bytes32(&mut self, field: &'static str, b: &[u8]) -> Result<(), EncodeError> {
        let n = u32::try_from(b.len()).map_err(|_| EncodeError::DataTooLarge(field))?;
        self.u32(n);
        self.raw(b);
        Ok(())
    }

    /// Start a TLV; returns what [`Writer::tlv_end`] needs to fill in the length.
    pub fn tlv_begin(&mut self, tag: u8) -> usize {
        self.u8(tag);
        self.u16(0);
        self.v.len()
    }

    pub fn tlv_end(&mut self, start: usize, field: &'static str) -> Result<(), EncodeError> {
        let n = u16::try_from(self.v.len() - start).map_err(|_| EncodeError::StringTooLong(field))?;
        self.v[start - 2..start].copy_from_slice(&n.to_le_bytes());
        Ok(())
    }

    /// A TLV whose value is the bytes as they are.
    pub fn tlv_raw(&mut self, tag: u8, field: &'static str, value: &[u8]) -> Result<(), EncodeError> {
        let start = self.tlv_begin(tag);
        self.raw(value);
        self.tlv_end(start, field)
    }
}
