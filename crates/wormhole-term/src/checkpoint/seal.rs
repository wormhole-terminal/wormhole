//! Sealing a checkpoint's screen: ChaCha20-Poly1305 (ring's) with a random nonce and associated data that
//! names the terminal, the generation and the key, so a file cannot be read without the key, changed without
//! being noticed, or passed off as another terminal's or another generation's.
//!
//! A sealed file is `"WHCK"`, the format byte, the nonce (12 bytes), the ciphertext and the tag (16 bytes).
//! There is no unsealed form of a screen: without a key nothing of the screen is written at all.

use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, NONCE_LEN, Nonce, UnboundKey};
use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

use super::FORMAT;

/// The key's length: 256 bits.
pub const KEY_LEN: usize = 32;

/// The first bytes of every sealed screen.
const MAGIC: &[u8; 4] = b"WHCK";
/// What the associated data starts with, so a file sealed for something else never opens here.
const CONTEXT: &[u8] = b"wormhole checkpoint screen v1\0";

/// A terminal's checkpoint key. The app derives one for each terminal from the profile's saved-text key and
/// the terminal's host id, and sends it to the host, which keeps it in memory only. Wiped when dropped.
#[derive(Clone)]
pub struct Key(Zeroizing<[u8; KEY_LEN]>);

impl Key {
    pub fn new(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// The key from the bytes the wire carries; `None` unless there are exactly [`KEY_LEN`].
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        Some(Self::new(bytes.try_into().ok()?))
    }

    /// The key's bytes, for the wire.
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// A short name for the key that reveals nothing of it (the first bytes of a digest of it): the
    /// manifest says which key sealed the screen, so a restore with another key says so instead of failing
    /// to open.
    pub fn id(&self) -> KeyId {
        let mut input = Zeroizing::new(Vec::with_capacity(KEY_LEN + 16));
        input.extend_from_slice(b"wormhole checkpoint key id\0");
        input.extend_from_slice(&*self.0);
        let sum = digest::digest(&digest::SHA256, &input);
        let mut id = [0u8; 8];
        id.copy_from_slice(&sum.as_ref()[..8]);
        KeyId(id)
    }
}

/// Equal keys are the same bytes, compared without stopping at the first difference.
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().zip(other.0.iter()).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
    }
}

impl Eq for Key {}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Key({})", self.id())
    }
}

/// What a key is called in the manifest.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct KeyId(pub [u8; 8]);

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl KeyId {
    /// From sixteen hex digits.
    pub fn parse(text: &str) -> Option<Self> {
        if text.len() != 16 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let mut id = [0u8; 8];
        for (i, byte) in id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).ok()?;
        }
        Some(Self(id))
    }
}

/// Why a sealed screen did not open.
#[derive(Debug, PartialEq, Eq)]
pub enum OpenError {
    /// Not a sealed screen, or cut short.
    NotSealed,
    /// Sealed in a layout this build does not know.
    Format(u8),
    /// Another key, another terminal or generation, or changed: which of them cannot be told, by design.
    Refused,
}

/// Where a sealed screen belongs: the terminal and the generation, and the key that sealed it.
#[derive(Clone, Copy)]
pub struct Binding<'a> {
    pub id: &'a str,
    pub generation: u64,
    pub key_id: KeyId,
}

fn aad(binding: &Binding<'_>) -> Vec<u8> {
    let mut aad = Vec::with_capacity(CONTEXT.len() + 1 + binding.id.len() + 8 + 8);
    aad.extend_from_slice(CONTEXT);
    aad.push(FORMAT as u8);
    aad.extend_from_slice(binding.id.as_bytes());
    aad.extend_from_slice(&binding.generation.to_le_bytes());
    aad.extend_from_slice(&binding.key_id.0);
    aad
}

fn cipher(key: &Key) -> LessSafeKey {
    // The key length is fixed by the type.
    LessSafeKey::new(UnboundKey::new(&CHACHA20_POLY1305, key.as_bytes()).expect("a 32-byte key"))
}

/// `screen` sealed for `binding` with a fresh random nonce.
pub fn seal(key: &Key, binding: &Binding<'_>, screen: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut nonce = [0u8; NONCE_LEN];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| std::io::Error::other("the system has no random numbers"))?;
    Ok(seal_with(key, binding, screen, nonce))
}

/// [`seal`] with the nonce given: for the golden vectors, whose bytes must not change from run to run.
pub(crate) fn seal_with(key: &Key, binding: &Binding<'_>, screen: &[u8], nonce: [u8; NONCE_LEN]) -> Vec<u8> {
    let head = MAGIC.len() + 1 + NONCE_LEN;
    let mut file = Vec::with_capacity(head + screen.len() + CHACHA20_POLY1305.tag_len());
    file.extend_from_slice(MAGIC);
    file.push(FORMAT as u8);
    file.extend_from_slice(&nonce);
    file.extend_from_slice(screen);
    let tag = cipher(key)
        .seal_in_place_separate_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(aad(binding)),
            &mut file[head..],
        )
        .expect("sealing a buffer in place cannot fail");
    file.extend_from_slice(tag.as_ref());
    file
}

/// The screen a sealed file holds, if `key` opens it for `binding`.
pub fn open(key: &Key, binding: &Binding<'_>, file: &[u8]) -> Result<Zeroizing<Vec<u8>>, OpenError> {
    let head = MAGIC.len() + 1 + NONCE_LEN;
    if file.len() < head + CHACHA20_POLY1305.tag_len() || &file[..MAGIC.len()] != MAGIC {
        return Err(OpenError::NotSealed);
    }
    let format = file[MAGIC.len()];
    if u32::from(format) != FORMAT {
        return Err(OpenError::Format(format));
    }
    let nonce =
        Nonce::try_assume_unique_for_key(&file[MAGIC.len() + 1..head]).map_err(|_| OpenError::NotSealed)?;
    let mut buffer = Zeroizing::new(file[head..].to_vec());
    let len = cipher(key)
        .open_in_place(nonce, Aad::from(aad(binding)), buffer.as_mut_slice())
        .map_err(|_| OpenError::Refused)?
        .len();
    buffer.truncate(len);
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "abcdefghij12";

    fn key(byte: u8) -> Key {
        Key::new([byte; KEY_LEN])
    }

    fn binding(key: &Key, generation: u64) -> Binding<'static> {
        Binding { id: ID, generation, key_id: key.id() }
    }

    #[test]
    fn a_screen_opens_with_its_key_terminal_and_generation_only() {
        let (a, b) = (key(1), key(2));
        let file = seal(&a, &binding(&a, 3), b"\x1b[1mhello\x1b[0m\r\n").expect("seals");
        assert_eq!(&*open(&a, &binding(&a, 3), &file).expect("opens"), b"\x1b[1mhello\x1b[0m\r\n");
        assert!(!file.windows(5).any(|w| w == b"hello"), "the text is not in the file");
        assert_eq!(open(&b, &binding(&b, 3), &file), Err(OpenError::Refused), "another key");
        assert_eq!(open(&a, &binding(&a, 4), &file), Err(OpenError::Refused), "another generation");
        let other = Binding { id: "zzzzzzzzzzzz", ..binding(&a, 3) };
        assert_eq!(open(&a, &other, &file), Err(OpenError::Refused), "another terminal");
    }

    #[test]
    fn a_changed_or_cut_file_is_refused() {
        let a = key(1);
        let file = seal(&a, &binding(&a, 1), b"text").expect("seals");
        for ix in 0..file.len() {
            let mut changed = file.clone();
            changed[ix] ^= 0x01;
            assert!(open(&a, &binding(&a, 1), &changed).is_err(), "byte {ix} changed");
        }
        for len in 0..file.len() {
            assert!(open(&a, &binding(&a, 1), &file[..len]).is_err(), "cut to {len}");
        }
    }

    #[test]
    fn every_file_has_its_own_nonce() {
        let a = key(1);
        let (x, y) =
            (seal(&a, &binding(&a, 1), b"same").unwrap(), seal(&a, &binding(&a, 1), b"same").unwrap());
        assert_ne!(x, y);
    }

    #[test]
    fn a_key_id_reveals_nothing_of_the_key_and_round_trips_as_text() {
        let a = key(7);
        let id = a.id();
        assert_eq!(KeyId::parse(&id.to_string()), Some(id));
        assert_ne!(id, key(8).id());
        assert!(!id.to_string().contains("0707"), "not the key's bytes");
        assert_eq!(KeyId::parse("xyz"), None);
        assert_eq!(KeyId::parse("0123456789abcdeg"), None);
        assert!(!format!("{a:?}").contains("0707070707"), "Debug does not print the key");
    }
}
