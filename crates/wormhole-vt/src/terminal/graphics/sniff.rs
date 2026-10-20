//! A picture's size from the first bytes of its file, without decoding it.
//!
//! The engine places a picture the moment its command ends, so it needs the size then, while the
//! host decodes later and elsewhere. The formats are the ones `imgcat` and kitty's `icat` send:
//! PNG (IHDR), GIF (the logical screen), JPEG (the first frame header), WebP (the three chunk
//! kinds) and BMP. The sizes are checked again by the decoder; this only has to be right for
//! pictures that decode.
//!
//! Specs: PNG (RFC 2083, §11.2.2), GIF89a (§18), JPEG (ITU T.81, B.2.2), WebP's container
//! specification (RIFF; VP8, VP8L and VP8X chunks), Windows BMP (BITMAPINFOHEADER).

/// The width and height in pixels, `None` when the bytes are not a picture of a known format or
/// their header is cut short.
pub(super) fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    png(data).or_else(|| gif(data)).or_else(|| jpeg(data)).or_else(|| webp(data)).or_else(|| bmp(data))
}

fn be32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?))
}

fn le16(bytes: &[u8]) -> Option<u32> {
    Some(u32::from(u16::from_le_bytes(bytes.get(..2)?.try_into().ok()?)))
}

fn le24(bytes: &[u8]) -> Option<u32> {
    let b = bytes.get(..3)?;
    Some(u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16)
}

fn le32(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?))
}

/// The signature, then the IHDR chunk whose first eight bytes are the size.
fn png(data: &[u8]) -> Option<(u32, u32)> {
    let rest = data.strip_prefix(b"\x89PNG\r\n\x1a\n")?;
    let rest = rest.get(4..)?.strip_prefix(b"IHDR")?;
    Some((be32(rest)?, be32(rest.get(4..)?)?))
}

fn gif(data: &[u8]) -> Option<(u32, u32)> {
    let rest = data.strip_prefix(b"GIF87a").or_else(|| data.strip_prefix(b"GIF89a"))?;
    Some((le16(rest)?, le16(rest.get(2..)?)?))
}

/// Walk the marker segments to the first start-of-frame marker (SOF0..SOF15 but DHT, JPG and
/// DAC); its payload is precision (1), height (2), width (2).
fn jpeg(data: &[u8]) -> Option<(u32, u32)> {
    let mut at = data.strip_prefix(&[0xFF, 0xD8])?;
    loop {
        // Fill bytes before a marker are allowed.
        let skip = at.iter().position(|&b| b != 0xFF)?;
        if skip == 0 {
            return None;
        }
        let marker = *at.get(skip)?;
        at = at.get(skip + 1..)?;
        match marker {
            // Standalone: no length (RSTn, TEM, SOI).
            0x01 | 0xD0..=0xD8 => continue,
            0xD9 | 0xDA => return None,
            _ => {}
        }
        let len = usize::from(u16::from_be_bytes(at.get(..2)?.try_into().ok()?));
        if len < 2 {
            return None;
        }
        if matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let frame = at.get(2..len)?;
            let height = u32::from(u16::from_be_bytes(frame.get(1..3)?.try_into().ok()?));
            let width = u32::from(u16::from_be_bytes(frame.get(3..5)?.try_into().ok()?));
            return Some((width, height));
        }
        at = at.get(len..)?;
    }
}

/// `RIFF size WEBP` then a chunk: `VP8 ` (a key frame: start code `9D 01 2A`, then two 14-bit
/// sizes), `VP8L` (signature `2F`, then two 14-bit sizes minus one) or `VP8X` (two 24-bit sizes
/// minus one).
fn webp(data: &[u8]) -> Option<(u32, u32)> {
    let rest = data.strip_prefix(b"RIFF")?.get(4..)?.strip_prefix(b"WEBP")?;
    let (kind, rest) = (rest.get(..4)?, rest.get(8..)?);
    match kind {
        b"VP8 " => {
            let frame = rest.get(3..)?.strip_prefix(&[0x9D, 0x01, 0x2A])?;
            Some((le16(frame)? & 0x3FFF, le16(frame.get(2..)?)? & 0x3FFF))
        }
        b"VP8L" => {
            let bits = le32(rest.strip_prefix(&[0x2F])?)?;
            Some(((bits & 0x3FFF) + 1, (bits >> 14 & 0x3FFF) + 1))
        }
        b"VP8X" => {
            let sizes = rest.get(4..)?;
            Some((le24(sizes)? + 1, le24(sizes.get(3..)?)? + 1))
        }
        _ => None,
    }
}

/// `BM`, the file header (14 bytes), then a header that starts with its size: width and height as
/// signed 32-bit numbers (a negative height is a top-down bitmap), or 16-bit ones in the old
/// 12-byte header.
fn bmp(data: &[u8]) -> Option<(u32, u32)> {
    let header = data.strip_prefix(b"BM")?.get(12..)?;
    match le32(header)? {
        12 => Some((le16(header.get(4..)?)?, le16(header.get(6..)?)?)),
        size if size >= 40 => {
            let width = le32(header.get(4..)?)? as i32;
            let height = le32(header.get(8..)?)? as i32;
            Some((width.unsigned_abs(), height.unsigned_abs()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::dimensions;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        out.extend_from_slice(&w.to_be_bytes());
        out.extend_from_slice(&h.to_be_bytes());
        out.extend_from_slice(&[8, 6, 0, 0, 0]);
        out
    }

    #[test]
    fn png_gif_and_bmp_headers() {
        assert_eq!(dimensions(&png(640, 480)), Some((640, 480)));
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[0x40, 0x01, 0xF0, 0x00, 0, 0, 0]);
        assert_eq!(dimensions(&gif), Some((320, 240)));
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&[0; 12]);
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&30i32.to_le_bytes());
        bmp.extend_from_slice(&(-20i32).to_le_bytes());
        assert_eq!(dimensions(&bmp), Some((30, 20)));
    }

    #[test]
    fn jpeg_walks_to_the_frame_header_and_skips_other_segments() {
        let mut jpeg = vec![0xFF, 0xD8];
        // An APP0 segment of 16 bytes, a DHT (0xC4: not a frame), then SOF2.
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
        jpeg.extend_from_slice(&[0; 14]);
        jpeg.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x04, 0, 0]);
        jpeg.extend_from_slice(&[0xFF, 0xC2, 0x00, 0x08, 8, 0x01, 0x00, 0x02, 0x00, 3]);
        assert_eq!(dimensions(&jpeg), Some((512, 256)));
        assert_eq!(dimensions(&jpeg[..20]), None, "cut short");
    }

    #[test]
    fn webp_in_its_three_forms() {
        let riff = |kind: &[u8], body: &[u8]| {
            let mut out = b"RIFF\0\0\0\0WEBP".to_vec();
            out.extend_from_slice(kind);
            out.extend_from_slice(&(body.len() as u32).to_le_bytes());
            out.extend_from_slice(body);
            out
        };
        let lossy = riff(b"VP8 ", &[0, 0, 0, 0x9D, 0x01, 0x2A, 0x64, 0x00, 0x32, 0x00]);
        assert_eq!(dimensions(&lossy), Some((100, 50)));
        let bits: u32 = 99 | 49 << 14;
        let mut body = vec![0x2F];
        body.extend_from_slice(&bits.to_le_bytes());
        assert_eq!(dimensions(&riff(b"VP8L", &body)), Some((100, 50)));
        let mut ext = vec![0; 4];
        ext.extend_from_slice(&[99, 0, 0, 49, 0, 0]);
        assert_eq!(dimensions(&riff(b"VP8X", &ext)), Some((100, 50)));
    }

    #[test]
    fn anything_else_is_nothing_and_nothing_panics() {
        assert_eq!(dimensions(b""), None);
        assert_eq!(dimensions(b"hello, world"), None);
        assert_eq!(dimensions(&[0xFF, 0xD8, 0xFF]), None);
        assert_eq!(dimensions(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0"), None);
        let mut rng = 0x1234_5678_u32;
        for len in 0..200 {
            let bytes: Vec<u8> = (0..len)
                .map(|_| {
                    rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (rng >> 24) as u8
                })
                .collect();
            let _ = dimensions(&bytes);
            let mut jpeg = vec![0xFF, 0xD8];
            jpeg.extend_from_slice(&bytes);
            let _ = dimensions(&jpeg);
        }
    }
}
