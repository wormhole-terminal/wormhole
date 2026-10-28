//! Reading a file to show it in the Files viewer: what kind it is and, for text, how much of it.
//!
//! The file belongs to a repository, so it is hostile input: it is read through [`LinkPolicy::Within`] (a
//! link that leads out of the root is refused), under a cap (a huge log must not fill the memory; its start
//! is shown with a note), and never run or interpreted. What is not text is not decoded as text. Blocks on
//! the disk: call it off the main thread.

use std::io;
use std::path::Path;

use super::{LinkPolicy, Refusal, read_capped, read_prefix_capped, refusal};

/// How much of a text file is read: more is cut, and the viewer says so.
pub const TEXT_CAP: u64 = 2 << 20;
/// The largest image shown.
pub const IMAGE_CAP: u64 = 50 << 20;
/// How much of the start a file is judged binary by: a NUL byte in it.
const SNIFF: usize = 8 << 10;

/// An image format the viewer shows. SVG is deliberately not one: it is text, and shown as text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageKind {
    Png,
    Jpeg,
    Gif,
    Webp,
    /// Apple's HEIF: the system decodes it, the app's image loader does not.
    Heic,
}

impl ImageKind {
    /// The format a file's extension claims (case does not matter).
    #[must_use]
    pub fn of_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match extension.as_str() {
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "heic" | "heif" => Self::Heic,
            _ => return None,
        })
    }

    /// Whether the first bytes are those of this format: a file called `.png` that is something else is
    /// not shown as a picture.
    #[must_use]
    pub fn matches(self, head: &[u8]) -> bool {
        match self {
            Self::Png => head.starts_with(b"\x89PNG\r\n\x1a\n"),
            Self::Jpeg => head.starts_with(&[0xFF, 0xD8, 0xFF]),
            Self::Gif => head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a"),
            Self::Webp => head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP",
            Self::Heic => {
                head.len() >= 12
                    && &head[4..8] == b"ftyp"
                    && matches!(&head[8..12], b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1")
            }
        }
    }
}

/// Why a file can't be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// It is not there (any more).
    Gone,
    /// With links resolved it is outside the folder the tree is of.
    Outside,
    /// A folder, a FIFO, a device.
    NotAFile,
    /// The system does not let this app read it.
    Denied,
    /// Anything else.
    Other,
}

impl Problem {
    fn of(err: &io::Error) -> Self {
        match refusal(err) {
            Some(Refusal::OutsideRoot(_)) => Self::Outside,
            Some(Refusal::NotRegularFile) => Self::NotAFile,
            Some(_) => Self::Other,
            None => match err.kind() {
                io::ErrorKind::NotFound => Self::Gone,
                io::ErrorKind::PermissionDenied => Self::Denied,
                _ => Self::Other,
            },
        }
    }
}

/// What the viewer has to show of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loaded {
    /// Text, whole or its first [`TEXT_CAP`] bytes (`truncated`); `size` is the file's.
    Text { text: String, size: u64, truncated: bool },
    /// Not text (or an image too large to show).
    Binary { size: u64 },
    /// A picture, its bytes as the file has them.
    Image { bytes: Vec<u8>, kind: ImageKind, size: u64 },
    /// It can't be shown.
    Unavailable(Problem),
}

/// Reads the file `relative` below `root` for the viewer.
pub fn load(root: &Path, relative: &Path) -> Loaded {
    let path = root.join(relative);
    let policy = LinkPolicy::Within(root);
    if let Some(kind) = ImageKind::of_extension(&path) {
        match read_capped(&path, IMAGE_CAP, policy) {
            Ok(bytes) if kind.matches(&bytes) => {
                let size = bytes.len() as u64;
                return Loaded::Image { bytes, kind, size };
            }
            // Called a picture and is not one: shown as what it is.
            Ok(_) => {}
            Err(err) if matches!(refusal(&err), Some(Refusal::TooLarge(_))) => {
                return match std::fs::metadata(&path) {
                    Ok(meta) => Loaded::Binary { size: meta.len() },
                    Err(err) => Loaded::Unavailable(Problem::of(&err)),
                };
            }
            Err(err) => return Loaded::Unavailable(Problem::of(&err)),
        }
    }
    match read_prefix_capped(&path, TEXT_CAP, policy) {
        Ok(prefix) => {
            let size = prefix.size;
            let truncated = prefix.truncated();
            match text_of(prefix.bytes, truncated) {
                Some(text) => Loaded::Text { text, size, truncated },
                None => Loaded::Binary { size },
            }
        }
        Err(err) => Loaded::Unavailable(Problem::of(&err)),
    }
}

/// `bytes` as text, or `None` when it is not: a NUL in the start, or not UTF-8. A cut file may end in the
/// middle of a character, which is dropped. A leading byte order mark is dropped.
fn text_of(bytes: Vec<u8>, cut: bool) -> Option<String> {
    if bytes[..bytes.len().min(SNIFF)].contains(&0) {
        return None;
    }
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => {
            let valid = err.utf8_error().valid_up_to();
            // Only the end of a cut file may be unfinished.
            if !cut || err.utf8_error().error_len().is_some() || err.as_bytes().len() - valid > 3 {
                return None;
            }
            let mut bytes = err.into_bytes();
            bytes.truncate(valid);
            String::from_utf8(bytes).ok()?
        }
    };
    Some(match text.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => text,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{ImageKind, Loaded, Problem, TEXT_CAP, load};
    use crate::platform::fs::symlink;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

    fn root_with(name: &str, bytes: &[u8]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(name), bytes).unwrap();
        root
    }

    #[test]
    fn text_is_read_whole() {
        let root = root_with("a.rs", b"fn main() {}\n");
        assert_eq!(
            load(root.path(), Path::new("a.rs")),
            Loaded::Text { text: "fn main() {}\n".into(), size: 13, truncated: false }
        );
    }

    #[test]
    fn a_file_over_the_cap_shows_its_start_and_says_so() {
        let root = tempfile::tempdir().unwrap();
        let line = "0123456789abcdef\n";
        let big = line.repeat(TEXT_CAP as usize / line.len() + 100);
        fs::write(root.path().join("big.log"), &big).unwrap();
        let Loaded::Text { text, size, truncated } = load(root.path(), Path::new("big.log")) else {
            panic!("text")
        };
        assert!(truncated);
        assert_eq!(size, big.len() as u64);
        assert_eq!(text.len() as u64, TEXT_CAP);
        assert!(big.starts_with(&text));
    }

    #[test]
    fn a_cut_in_the_middle_of_a_character_drops_that_character() {
        let root = tempfile::tempdir().unwrap();
        let mut bytes = vec![b'a'; TEXT_CAP as usize - 1];
        bytes.extend("é".as_bytes()); // two bytes: the cap falls between them
        bytes.extend(b"tail");
        fs::write(root.path().join("t.txt"), &bytes).unwrap();
        let Loaded::Text { text, truncated, .. } = load(root.path(), Path::new("t.txt")) else {
            panic!("text")
        };
        assert!(truncated);
        assert_eq!(text.len(), TEXT_CAP as usize - 1);
    }

    #[test]
    fn a_nul_byte_or_bad_utf8_is_binary() {
        let root = root_with("a.bin", b"MZ\0\0data");
        assert_eq!(load(root.path(), Path::new("a.bin")), Loaded::Binary { size: 8 });
        fs::write(root.path().join("latin.txt"), b"caf\xe9\n").unwrap();
        assert_eq!(load(root.path(), Path::new("latin.txt")), Loaded::Binary { size: 5 });
    }

    #[test]
    fn a_byte_order_mark_is_not_part_of_the_text() {
        let root = root_with("a.txt", b"\xef\xbb\xbfhello");
        assert!(
            matches!(load(root.path(), Path::new("a.txt")), Loaded::Text { text, .. } if text == "hello")
        );
    }

    #[test]
    fn a_picture_is_read_as_one_and_an_svg_as_text() {
        let root = root_with("p.PNG", PNG);
        assert!(matches!(
            load(root.path(), Path::new("p.PNG")),
            Loaded::Image { kind: ImageKind::Png, size, .. } if size == PNG.len() as u64
        ));
        fs::write(root.path().join("v.svg"), "<svg/>").unwrap();
        assert!(matches!(load(root.path(), Path::new("v.svg")), Loaded::Text { .. }));
        // Called a picture, not one: what it is.
        fs::write(root.path().join("fake.png"), "just text\n").unwrap();
        assert!(matches!(load(root.path(), Path::new("fake.png")), Loaded::Text { .. }));
    }

    #[test]
    fn every_picture_format_is_known_by_its_first_bytes() {
        let heic = b"\0\0\0\x18ftypheic\0\0\0\0";
        let webp = b"RIFF\x10\0\0\0WEBPVP8 ";
        assert!(ImageKind::Heic.matches(heic) && !ImageKind::Png.matches(heic));
        assert!(ImageKind::Webp.matches(webp));
        assert!(ImageKind::Jpeg.matches(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(ImageKind::Gif.matches(b"GIF89a.."));
        assert_eq!(ImageKind::of_extension(Path::new("a/b.JPEG")), Some(ImageKind::Jpeg));
        assert_eq!(ImageKind::of_extension(Path::new("a.svg")), None);
    }

    #[test]
    fn a_fifo_or_a_device_is_refused_before_it_is_opened() {
        let root = tempfile::tempdir().unwrap();
        let made = std::process::Command::new("mkfifo").arg(root.path().join("pipe")).status();
        if made.is_ok_and(|status| status.success()) {
            // Opening it for reading would wait for a writer (or, a named pipe in a repository, hang the reader).
            assert_eq!(load(root.path(), Path::new("pipe")), Loaded::Unavailable(Problem::NotAFile));
        }
        // A device inside a folder is a link here, for a plain folder cannot hold one; the system's own is
        // refused by type all the same.
        assert_eq!(
            load(Path::new("/dev"), Path::new("null")),
            Loaded::Unavailable(Problem::NotAFile),
            "a character device"
        );
    }

    #[test]
    fn a_link_out_of_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let outside = root_with("secret.txt", b"secret");
        symlink(outside.path().join("secret.txt"), root.path().join("l.txt")).unwrap();
        assert_eq!(load(root.path(), Path::new("l.txt")), Loaded::Unavailable(Problem::Outside));
        // A folder and a missing file are told apart.
        fs::create_dir(root.path().join("d")).unwrap();
        assert_eq!(load(root.path(), Path::new("d")), Loaded::Unavailable(Problem::NotAFile));
        assert_eq!(load(root.path(), Path::new("nope")), Loaded::Unavailable(Problem::Gone));
    }
}
