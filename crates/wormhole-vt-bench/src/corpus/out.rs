//! What a generator writes to.

use super::words::WORDS;
use crate::rng::Rng;

/// What a generator writes to: the bytes so far, the random numbers and the screen size.
pub(crate) struct Out {
    pub bytes: Vec<u8>,
    pub rng: Rng,
    pub cols: usize,
    pub rows: usize,
}

impl Out {
    pub fn new(seed: u64, cols: usize, rows: usize) -> Out {
        Out { bytes: Vec::new(), rng: Rng::new(seed), cols, rows }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn text(&mut self, s: &str) {
        self.bytes.extend_from_slice(s.as_bytes());
    }

    /// `ESC [ body`.
    pub fn csi(&mut self, body: &str) {
        self.text("\x1b[");
        self.text(body);
    }

    pub fn sgr(&mut self, params: &str) {
        self.csi(params);
        self.text("m");
    }

    pub fn goto(&mut self, row: usize, col: usize) {
        self.csi(&format!("{row};{col}H"));
    }

    pub fn word(&mut self) -> &'static str {
        self.rng.pick(WORDS)
    }

    /// `n` words joined by blanks.
    pub fn words(&mut self, n: usize) -> String {
        let mut s = String::new();
        for i in 0..n {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(self.word());
        }
        s
    }

    pub fn words_between(&mut self, lo: usize, hi: usize) -> String {
        let n = self.rng.range(lo, hi);
        self.words(n)
    }

    pub fn line_between(&mut self, lo: usize, hi: usize) -> String {
        let n = self.rng.range(lo, hi);
        self.line_of(n)
    }

    pub fn base64_between(&mut self, lo: usize, hi: usize) -> String {
        let n = self.rng.range(lo, hi);
        self.base64(n)
    }

    /// A line of about `len` characters made of words.
    pub fn line_of(&mut self, len: usize) -> String {
        let mut s = String::new();
        while s.len() < len {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(self.word());
        }
        s.truncate(len.max(1));
        s
    }

    /// Standard base64 of `n` random bytes (the payload of an image or a clipboard).
    pub fn base64(&mut self, n: usize) -> String {
        const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut data = Vec::with_capacity(n);
        while data.len() < n {
            data.extend_from_slice(&self.rng.next_u64().to_le_bytes());
        }
        data.truncate(n);
        let mut out = String::with_capacity(n.div_ceil(3) * 4);
        for chunk in data.chunks(3) {
            let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
}
