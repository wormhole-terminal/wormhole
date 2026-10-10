//! Image data to RGBA under limits: kitty's raw RGB / RGBA (zlib or not) and PNG, iTerm2's
//! files in whatever format the `image` crate reads (PNG, JPEG, GIF: the first frame).

use std::io::{Cursor, Read};

use super::{Image, MAX_DIMENSION, MAX_PIXELS};

/// What the bytes are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// kitty `f=24`: packed RGB, `width` × `height`.
    Rgb { width: u32, height: u32 },
    /// kitty `f=32`: packed RGBA.
    Rgba { width: u32, height: u32 },
    /// kitty `f=100`.
    Png,
    /// iTerm2: whatever the bytes say they are.
    Any,
}

/// What a decode may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Pixels on either side.
    pub max_dimension: u32,
    pub max_pixels: u64,
    /// Decompressed input bytes (zlib), and what the decoders may allocate.
    pub max_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_dimension: MAX_DIMENSION, max_pixels: MAX_PIXELS, max_bytes: (MAX_PIXELS * 4) as usize }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooBig,
    /// What the decoder said, for the log.
    Invalid(String),
}

/// A decoded image.
pub type Decoded = Image;

fn check_size(width: u32, height: u32, limits: &Limits) -> Result<(), DecodeError> {
    if width == 0 || height == 0 {
        return Err(DecodeError::Invalid("an empty image".into()));
    }
    if width > limits.max_dimension
        || height > limits.max_dimension
        || u64::from(width) * u64::from(height) > limits.max_pixels
    {
        return Err(DecodeError::TooBig);
    }
    Ok(())
}

/// zlib-compressed `data` inflated, up to `limits.max_bytes` (anything more is a bomb).
fn inflate(data: &[u8], limits: &Limits) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let cap = limits.max_bytes as u64;
    flate2::read::ZlibDecoder::new(data)
        .take(cap + 1)
        .read_to_end(&mut out)
        .map_err(|err| DecodeError::Invalid(format!("zlib: {err}")))?;
    if out.len() as u64 > cap {
        return Err(DecodeError::TooBig);
    }
    Ok(out)
}

/// What the `image` crate refused, as this module says it: its limits are the size caps.
fn image_error(err: image::ImageError) -> DecodeError {
    match err {
        image::ImageError::Limits(_) => DecodeError::TooBig,
        other => DecodeError::Invalid(other.to_string()),
    }
}

fn image_limits(limits: &Limits) -> image::Limits {
    let mut out = image::Limits::default();
    out.max_image_width = Some(limits.max_dimension);
    out.max_image_height = Some(limits.max_dimension);
    // Decoders keep the pixels and some scratch; twice the pixels covers both.
    out.max_alloc = Some((limits.max_bytes as u64).saturating_mul(2));
    out
}

/// `data` as `format` says, within `limits`.
pub fn decode(format: Format, data: &[u8], compressed: bool, limits: &Limits) -> Result<Image, DecodeError> {
    let inflated;
    let data = if compressed {
        inflated = inflate(data, limits)?;
        &inflated[..]
    } else {
        data
    };
    match format {
        Format::Rgb { width, height } | Format::Rgba { width, height } => {
            check_size(width, height, limits)?;
            let channels = if matches!(format, Format::Rgb { .. }) { 3 } else { 4 };
            let pixels = (width * height) as usize;
            if data.len() < pixels * channels {
                return Err(DecodeError::Invalid(format!(
                    "{} bytes for {width}×{height} pixels with {channels} channels",
                    data.len()
                )));
            }
            let rgba = if channels == 4 {
                data[..pixels * 4].to_vec()
            } else {
                let mut rgba = Vec::with_capacity(pixels * 4);
                for px in data[..pixels * 3].as_chunks::<3>().0 {
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
                rgba
            };
            Ok(Image { width, height, rgba })
        }
        Format::Png | Format::Any => {
            let mut reader = image::ImageReader::new(Cursor::new(data))
                .with_guessed_format()
                .map_err(|err| DecodeError::Invalid(err.to_string()))?;
            if format == Format::Png && reader.format() != Some(image::ImageFormat::Png) {
                return Err(DecodeError::Invalid("not a PNG".into()));
            }
            reader.limits(image_limits(limits));
            let (width, height) = reader.into_dimensions().map_err(image_error)?;
            check_size(width, height, limits)?;
            let mut reader = image::ImageReader::new(Cursor::new(data))
                .with_guessed_format()
                .map_err(|err| DecodeError::Invalid(err.to_string()))?;
            reader.limits(image_limits(limits));
            let decoded = reader.decode().map_err(image_error)?;
            let rgba = decoded.into_rgba8();
            let (width, height) = rgba.dimensions();
            Ok(Image { width, height, rgba: rgba.into_raw() })
        }
    }
}

/// The image's size in pixels, from its header alone (no decode), or `None` when the bytes
/// don't say.
pub fn dimensions(format: Format, data: &[u8], compressed: bool, limits: &Limits) -> Option<(u32, u32)> {
    match format {
        Format::Rgb { width, height } | Format::Rgba { width, height } => Some((width, height)),
        Format::Png | Format::Any => {
            let inflated;
            let data = if compressed {
                inflated = inflate(data, limits).ok()?;
                &inflated[..]
            } else {
                data
            };
            image::ImageReader::new(Cursor::new(data)).with_guessed_format().ok()?.into_dimensions().ok()
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::io::Write;

    use super::{DecodeError, Format, Limits, decode, dimensions};

    /// A `width` × `height` PNG of one color.
    pub(crate) fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba(rgba));
        let mut out = std::io::Cursor::new(Vec::new());
        image.write_to(&mut out, image::ImageFormat::Png).expect("encodes");
        out.into_inner()
    }

    fn zlib(data: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(data).expect("compresses");
        encoder.finish().expect("finishes")
    }

    #[test]
    fn raw_pixels_png_and_zlib_decode_to_rgba() {
        let limits = Limits::default();
        let rgb =
            decode(Format::Rgb { width: 2, height: 1 }, &[1, 2, 3, 4, 5, 6], false, &limits).expect("rgb");
        assert_eq!(rgb.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
        let rgba =
            decode(Format::Rgba { width: 1, height: 1 }, &[9, 9, 9, 128], false, &limits).expect("rgba");
        assert_eq!((rgba.width, rgba.height, rgba.rgba), (1, 1, vec![9, 9, 9, 128]));
        let packed = zlib(&[7, 8, 9]);
        assert_eq!(
            decode(Format::Rgb { width: 1, height: 1 }, &packed, true, &limits).expect("zlib").rgba,
            [7, 8, 9, 255]
        );
        let png = png(3, 2, [10, 20, 30, 255]);
        let decoded = decode(Format::Png, &png, false, &limits).expect("png");
        assert_eq!((decoded.width, decoded.height), (3, 2));
        assert_eq!(&decoded.rgba[..4], &[10, 20, 30, 255]);
        assert_eq!(dimensions(Format::Png, &png, false, &limits), Some((3, 2)));
        assert_eq!(dimensions(Format::Any, &zlib(&png), true, &limits), Some((3, 2)));
        assert!(decode(Format::Any, &png, false, &limits).is_ok());
    }

    #[test]
    fn bad_and_oversized_data_is_refused_not_decoded() {
        let limits = Limits { max_dimension: 4, max_pixels: 8, max_bytes: 64 };
        assert_eq!(
            decode(Format::Rgb { width: 5, height: 1 }, &[0; 15], false, &limits),
            Err(DecodeError::TooBig)
        );
        assert_eq!(
            decode(Format::Rgba { width: 3, height: 3 }, &[0; 36], false, &limits),
            Err(DecodeError::TooBig)
        );
        assert!(matches!(
            decode(Format::Rgb { width: 2, height: 1 }, &[0; 3], false, &limits),
            Err(DecodeError::Invalid(_))
        ));
        assert!(matches!(decode(Format::Png, b"not a png", false, &limits), Err(DecodeError::Invalid(_))));
        assert!(matches!(decode(Format::Png, &[0; 0], false, &limits), Err(DecodeError::Invalid(_))));
        let jpeg_as_png = {
            let image = image::RgbImage::from_pixel(1, 1, image::Rgb([0, 0, 0]));
            let mut out = std::io::Cursor::new(Vec::new());
            image.write_to(&mut out, image::ImageFormat::Jpeg).expect("encodes");
            out.into_inner()
        };
        assert!(matches!(
            decode(Format::Png, &jpeg_as_png, false, &Limits::default()),
            Err(DecodeError::Invalid(_))
        ));
        assert!(
            decode(Format::Any, &jpeg_as_png, false, &Limits::default()).is_ok(),
            "iTerm2 takes any format"
        );
        // A PNG bigger than the limits is refused by its header, before any pixels exist.
        let big = png(5, 5, [0; 4]);
        assert_eq!(decode(Format::Png, &big, false, &limits), Err(DecodeError::TooBig));
        assert_eq!(dimensions(Format::Png, &big, false, &limits), Some((5, 5)));
        // A zlib bomb stops at the byte cap.
        let bomb = zlib(&vec![0u8; 10_000]);
        assert_eq!(
            decode(Format::Rgb { width: 1, height: 1 }, &bomb, true, &limits),
            Err(DecodeError::TooBig)
        );
        assert_eq!(dimensions(Format::Png, &bomb, true, &limits), None);
    }
}
