//! The types the host sees: pictures to decode, and virtual placements.

use crate::types::ImageHandle;

/// How the bytes of an [`ImageUpload`] are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    /// `width * height` pixels of red, green, blue.
    Rgb,
    /// `width * height` pixels of red, green, blue, alpha (straight, not premultiplied).
    Rgba,
    /// A PNG file.
    Png,
    /// A picture file in some format the host's decoder reads (iTerm2 sends whatever `imgcat`
    /// was given: PNG, JPEG, GIF, …). The size in the upload was read from the file's header.
    File,
}

/// A transform applied to the data on top of its format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// As it is.
    None,
    /// zlib (RFC 1950) deflate; kitty's `o=z`. Raw pixels only.
    Zlib,
}

/// Why the host could not make pixels of an upload; the engine answers the program with the
/// matching code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageError {
    /// The data is not the picture it says it is (`ENODATA`).
    BadData,
    /// It decodes to more than the host keeps (`EFBIG`).
    TooBig,
    /// A format or feature the host does not read (`EBADF`).
    Unsupported,
}

/// A picture the host is to decode and keep under [`ImageUpload::image`] until it is told to let
/// go ([`Event::ImageFree`](crate::terminal::Event::ImageFree)).
///
/// The host must call [`Terminal::image_decoded`](crate::terminal::Terminal::image_decoded) once for
/// every upload when it has the pixels or has given up; the answer to a program that asked for
/// one (kitty's `OK` after a PNG, say) waits for it, and so do the replies after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageUpload {
    /// The key to store the pixels under.
    pub image: ImageHandle,
    /// How `data` is laid out.
    pub format: ImageFormat,
    /// What was done to `data` on top.
    pub compression: Compression,
    /// Pixels across: given for raw data, read from the header for the file formats. The decoded
    /// picture must have exactly this size, or the upload failed.
    pub width: u32,
    /// Pixels down; see `width`.
    pub height: u32,
    /// The bytes (base64 already undone).
    pub data: Vec<u8>,
}

impl ImageUpload {
    /// Bytes the decoded picture takes as RGBA: what a host budgets for.
    pub fn decoded_bytes(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4
    }
}

/// A virtual placement (kitty's `U=1`): the program prints the picture itself as placeholder
/// characters, and this says how many cells the picture spans. The placeholder cells are read with
/// [`placeholder_at`](crate::terminal::placeholder_at).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualPlacement {
    /// The picture.
    pub image: ImageHandle,
    /// The id the program gave the picture.
    pub image_id: u32,
    /// The id the program gave the placement, 0 for none.
    pub placement_id: u32,
    /// Columns of placeholder cells the picture spans.
    pub cols: u32,
    /// Rows of placeholder cells the picture spans.
    pub rows: u32,
}
