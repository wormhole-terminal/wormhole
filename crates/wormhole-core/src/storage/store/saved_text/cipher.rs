//! Sealing a terminal's saved text: ChaCha20-Poly1305 (ring's, the AEAD rustls already brings) with
//! a random nonce per record and the terminal's id as associated data, so a record can't be read
//! without the key, changed without being noticed, or passed off as another terminal's.
//!
//! A record is `[FORMAT][nonce: 12 bytes][ciphertext][tag: 16 bytes]`; the format byte is in the
//! associated data too. Random 96-bit nonces are safe here: a profile writes one record per changed
//! terminal a minute, nowhere near the 2^32 records one key could seal before a repeat became a
//! real risk.
//!
//! The raw key bytes are wiped when dropped; ring's expanded key inside [`Cipher`], and the copy the
//! Keychain call makes (`CFData`), are not: both live only in this process's memory.

use anyhow::{Result, anyhow, bail};
use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

/// The key's length: 256 bits.
pub(crate) const KEY_LEN: usize = 32;
/// The first byte of every record: how the rest is laid out.
const FORMAT: u8 = 1;
/// What the associated data starts with, so a record sealed for something else never opens here.
const CONTEXT: &[u8] = b"wormhole saved terminal text v1\0";

/// The key a profile's text is sealed with.
pub(crate) type Key = Zeroizing<[u8; KEY_LEN]>;

/// A fresh random key.
pub(crate) fn new_key() -> Result<Key> {
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    SystemRandom::new().fill(key.as_mut()).map_err(|_| anyhow!("the system has no random numbers"))?;
    Ok(key)
}

pub(crate) struct Cipher {
    key: LessSafeKey,
    random: SystemRandom,
}

impl Cipher {
    pub(super) fn new(key: &Key) -> Result<Self> {
        let key = UnboundKey::new(&CHACHA20_POLY1305, key.as_ref())
            .map_err(|_| anyhow!("a key of the wrong length"))?;
        Ok(Self { key: LessSafeKey::new(key), random: SystemRandom::new() })
    }

    /// `text` sealed for the terminal `tab_id`.
    pub(super) fn seal(&self, tab_id: i64, text: &str) -> Result<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        self.random.fill(&mut nonce).map_err(|_| anyhow!("the system has no random numbers"))?;
        // One buffer, sized up front: the text is sealed where it lies, never copied again.
        let head = 1 + NONCE_LEN;
        let mut record = Vec::with_capacity(head + text.len() + CHACHA20_POLY1305.tag_len());
        record.push(FORMAT);
        record.extend_from_slice(&nonce);
        record.extend_from_slice(text.as_bytes());
        let tag = self
            .key
            .seal_in_place_separate_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(aad(tab_id)),
                &mut record[head..],
            )
            .map_err(|_| anyhow!("sealing the text failed"))?;
        record.extend_from_slice(tag.as_ref());
        Ok(record)
    }

    /// The text of a record sealed for `tab_id`; an error when the key is another, or the record was
    /// changed, cut, or sealed for another terminal (which can't be told apart, by design).
    pub(super) fn open(&self, tab_id: i64, record: &[u8]) -> Result<String> {
        let Some((&FORMAT, rest)) = record.split_first() else { bail!("not a saved text record") };
        if rest.len() < NONCE_LEN + CHACHA20_POLY1305.tag_len() {
            bail!("a saved text record cut short");
        }
        let (nonce, sealed) = rest.split_at(NONCE_LEN);
        let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| anyhow!("a bad nonce"))?;
        let mut buffer = Zeroizing::new(sealed.to_vec());
        let text = self
            .key
            .open_in_place(nonce, Aad::from(aad(tab_id)), buffer.as_mut_slice())
            .map_err(|_| anyhow!("the saved text doesn't open with this key (another key, or changed)"))?;
        Ok(String::from_utf8(text.to_vec())?)
    }
}

fn aad(tab_id: i64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(CONTEXT.len() + 1 + 8);
    aad.extend_from_slice(CONTEXT);
    aad.push(FORMAT);
    aad.extend_from_slice(&tab_id.to_le_bytes());
    aad
}

#[cfg(test)]
mod tests {
    use super::{Cipher, KEY_LEN, Key, new_key};

    fn cipher(byte: u8) -> Cipher {
        Cipher::new(&Key::new([byte; KEY_LEN])).expect("a cipher")
    }

    #[test]
    fn a_record_opens_with_its_key_and_terminal_only() {
        let (a, b) = (cipher(1), cipher(2));
        let record = a.seal(7, "$ echo secret\nsecret\n").expect("seals");
        assert_eq!(a.open(7, &record).expect("opens"), "$ echo secret\nsecret\n");
        assert!(!record.windows(6).any(|w| w == b"secret"), "the text isn't in the record");
        assert!(b.open(7, &record).is_err(), "another key");
        assert!(a.open(8, &record).is_err(), "another terminal's");
    }

    #[test]
    fn a_changed_or_cut_record_is_refused() {
        let a = cipher(1);
        let record = a.seal(7, "text").expect("seals");
        for ix in 0..record.len() {
            let mut changed = record.clone();
            changed[ix] ^= 0x01;
            assert!(a.open(7, &changed).is_err(), "byte {ix} changed");
        }
        for len in 0..record.len() {
            assert!(a.open(7, &record[..len]).is_err(), "cut to {len}");
        }
        assert!(a.open(7, &[]).is_err());
    }

    #[test]
    fn every_record_has_its_own_nonce_and_keys_are_random() {
        let a = cipher(1);
        let (one, two) = (a.seal(7, "same").expect("seals"), a.seal(7, "same").expect("seals"));
        assert_ne!(one, two);
        assert_eq!(a.open(7, &one).expect("opens"), a.open(7, &two).expect("opens"));
        assert_ne!(*new_key().expect("a key"), *new_key().expect("a key"));
        assert_eq!(a.seal(1, "").map(|r| r.len()).expect("seals"), 1 + 12 + 16);
    }
}
