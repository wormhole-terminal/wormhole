//! Finding where a run of bytes ends, many at a time.
//!
//! Three questions are asked of the input millions of times a second: how far does printable ASCII
//! go (ground state), where is the first C0 control (an OSC payload) and where is the first byte
//! that ends or interrupts a DCS/APC/PM/SOS payload (CAN, SUB, ESC, DEL). Short runs, which are
//! most of them, are answered a `u64` at a time: the test is made on every byte of the word at
//! once, the byte lanes kept apart by clearing bit 7 before adding, so that no carry crosses from
//! one lane into the next and every flagged lane is exact; on a little-endian load the lowest
//! flagged lane is the earliest byte. Past 64 bytes the answer is a reduction over a whole block
//! (the largest byte, say), which the compiler turns into the machine's vector instructions. Safe
//! code only: no `unsafe`, no `std::arch`.

const ONES: u64 = 0x0101_0101_0101_0101;
const HIGH: u64 = 0x8080_8080_8080_8080;
const LOW7: u64 = 0x7F7F_7F7F_7F7F_7F7F;

/// Bit 7 of each lane that is not 0x20..=0x7E. With `y` the low seven bits of a lane, `y + 0x60`
/// reaches 0x80 exactly when `y >= 0x20` and `y + 1` exactly when `y == 0x7F`; a set bit 7 of the
/// byte itself covers 0x80..=0xFF.
#[inline(always)]
fn not_printable(w: u64) -> u64 {
    let y = w & LOW7;
    (w | !(y + 0x60 * ONES) | (y + ONES)) & HIGH
}

/// Bit 7 of each lane below 0x20 (a C0 control); bytes of 0x80 and up are not controls.
#[inline(always)]
fn is_control(w: u64) -> u64 {
    let y = w & LOW7;
    !(w | (y + 0x60 * ONES)) & HIGH
}

/// Bit 7 of each lane that is a C0 control or DEL, the bytes among which CAN, SUB and ESC lie.
#[inline(always)]
fn is_control_or_del(w: u64) -> u64 {
    let y = w & LOW7;
    (!(y + 0x60 * ONES) | (y + ONES)) & !w & HIGH
}

#[inline(always)]
fn word(bytes: &[u8; 8]) -> u64 {
    u64::from_le_bytes(*bytes)
}

/// The index of the first byte of `bytes` that fails a question, or `bytes.len()`. `flag` asks it
/// of eight bytes at once. `ok64` asks it of sixty-four and is written as a plain reduction over
/// the bytes, which the compiler turns into the machine's vector instructions (no `unsafe`, no
/// `std::arch`); that is several times cheaper per byte than the words but pays only for a long
/// run. Lines of text and escape sequences are short, and the words alone find where those end
/// with one mispredicted branch, so they come first.
#[inline(always)]
fn find(bytes: &[u8], ok64: impl Fn(&[u8; 64]) -> bool, flag: impl Fn(u64) -> u64) -> usize {
    let mut at = 0;
    let (words, _) = bytes.as_chunks::<8>();
    for w in words.iter().take(8) {
        let hit = flag(word(w));
        if hit != 0 {
            return at + (hit.trailing_zeros() / 8) as usize;
        }
        at += 8;
    }
    while let Some(block) = bytes[at..].first_chunk::<64>() {
        if !ok64(block) {
            break;
        }
        at += 64;
    }
    // The stop is within the next eight words, or in a tail of fewer than eight bytes.
    let (words, _) = bytes[at..].as_chunks::<8>();
    for w in words {
        let hit = flag(word(w));
        if hit != 0 {
            return at + (hit.trailing_zeros() / 8) as usize;
        }
        at += 8;
    }
    // Pad the tail with a space, which no question flags.
    let rest = &bytes[at..];
    let mut buf = [b' '; 8];
    buf[..rest.len()].copy_from_slice(rest);
    at + ((flag(word(&buf)).trailing_zeros() / 8) as usize).min(rest.len())
}

/// The length of the printable ASCII (0x20..=0x7E) at the start of `bytes`.
#[inline]
pub(super) fn printable_run(bytes: &[u8]) -> usize {
    find(bytes, |b| b.iter().fold(0, |m, &x| m.max(x.wrapping_sub(0x20))) < 0x5F, not_printable)
}

/// The index of the first C0 control (a byte below 0x20) in `bytes`, or its length. DEL and the
/// bytes of UTF-8 text are not controls.
#[inline]
pub(super) fn first_control(bytes: &[u8]) -> usize {
    find(bytes, |b| b.iter().fold(0xFF, |m, &x| m.min(x)) >= 0x20, is_control)
}

/// The index of the first byte in `bytes` that ends or interrupts a DCS, APC, PM or SOS payload
/// (CAN 0x18, SUB 0x1A, ESC 0x1B, or DEL 0x7F, which is dropped), or its length. Other C0
/// controls are part of the payload.
#[inline]
pub(super) fn string_run(bytes: &[u8]) -> usize {
    let mut at = 0;
    loop {
        // Controls and DEL are rare in a payload, so find them in bulk and look at each.
        at += find(
            &bytes[at..],
            |b| b.iter().fold(0, |m, &x| m | u8::from(x < 0x20) | u8::from(x == 0x7F)) == 0,
            is_control_or_del,
        );
        match bytes.get(at) {
            Some(0x18 | 0x1A | 0x1B | 0x7F) | None => return at,
            Some(_) => at += 1,
        }
    }
}
