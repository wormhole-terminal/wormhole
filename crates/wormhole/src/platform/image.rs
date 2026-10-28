//! Decoding a picture the person's file holds, for the Files viewer: by the system's own decoders (ImageIO),
//! which read what a camera or a phone makes (HEIC) as well as PNG, JPEG, GIF and WebP.
//!
//! The file is hostile input, so nothing is decoded at its own size: the size it declares is read first and
//! a picture of more than [`MAX_DECLARED_PIXELS`] is refused (a few-KB GIF can declare 65535 x 65535), and what
//! is decoded is a thumbnail of at most [`MAX_SIDE`] pixels on its long side, which the decoder makes while it
//! reads. Only the first frame of an animation. Blocks while it decodes: call it off the main thread.

use super::os::image as os;

/// The most pixels a picture may declare (width x height): 10,000 x 10,000.
pub const MAX_DECLARED_PIXELS: u64 = 100_000_000;
/// The longest side of what is decoded, in pixels. Large enough for a retina display's window; a picture
/// bigger than this is scaled down while it is read.
pub const MAX_SIDE: u32 = 4096;

/// A decoded picture: straight (not premultiplied) RGBA, `width * height * 4` bytes.
#[derive(Debug)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Why a picture was not decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// It declares more pixels than [`MAX_DECLARED_PIXELS`] (or a side that is not a size).
    TooLarge { width: u64, height: u64 },
    /// The system could not read it as a picture.
    Unreadable,
}

/// The first frame of the picture in `bytes`, at most [`MAX_SIDE`] on its long side.
pub fn decode(bytes: &[u8]) -> Result<Decoded, Refused> {
    os::decode(bytes)
}

/// Pictures made to declare a size they do not have, for tests of what a hostile file can ask for.
#[cfg(test)]
pub(crate) mod crafted {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }

    /// A PNG that is a signature, an IHDR declaring `width` x `height` (RGBA, 8 bits) and an IEND: a few dozen
    /// bytes, and no pixels.
    pub(crate) fn png_declaring(width: u32, height: u32) -> Vec<u8> {
        let chunk = |kind: &[u8; 4], data: &[u8]| {
            let mut out = (data.len() as u32).to_be_bytes().to_vec();
            let body: Vec<u8> = kind.iter().chain(data).copied().collect();
            out.extend(&body);
            out.extend(crc32(&body).to_be_bytes());
            out
        };
        let mut header = Vec::new();
        header.extend(width.to_be_bytes());
        header.extend(height.to_be_bytes());
        header.extend([8, 6, 0, 0, 0]);
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(chunk(b"IHDR", &header));
        png.extend(chunk(b"IEND", &[]));
        png
    }

    /// A GIF whose logical screen is `width` x `height` (up to 65535 each: 17 GB a frame as RGBA) with no
    /// frame: a header, the screen descriptor and the trailer.
    pub(crate) fn gif_declaring(width: u16, height: u16) -> Vec<u8> {
        let mut gif = b"GIF89a".to_vec();
        gif.extend(width.to_le_bytes());
        gif.extend(height.to_le_bytes());
        gif.extend([0x00, 0x00, 0x00, 0x3B]);
        gif
    }

    /// A GIF of one 1 x 1 frame on a logical screen of `width` x `height`.
    pub(crate) fn gif_with_a_pixel_on_a_screen(width: u16, height: u16) -> Vec<u8> {
        let mut gif =
            hex("47494638396101000100800000000000ffffff21f90401000000002c00000000010001000002024401003b");
        gif[6..8].copy_from_slice(&width.to_le_bytes());
        gif[8..10].copy_from_slice(&height.to_le_bytes());
        gif
    }

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len()).step_by(2).map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap()).collect()
    }
}
