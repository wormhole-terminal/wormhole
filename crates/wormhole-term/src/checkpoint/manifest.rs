//! A checkpoint's manifest: what a restore needs to know before it opens the sealed screen. UTF-8 text, one
//! `key=value` per line, unknown keys ignored by readers, `0600`; the same conventions as a host's meta file
//! (`hosted::meta`).
//!
//! ```text
//! wormhole-checkpoint=1
//! min_reader=1
//! generation=7
//! time=1760000000
//! key_id=0123456789abcdef
//! screen_len=1234
//! ended=exit:0
//! ```
//!
//! **Nothing here says what the person did.** The manifest is plain text on a disk that backups and other users'
//! tools may read, so it holds no folder, no title (shells put whole command lines there) and no shell: a
//! restore takes none of them from here (the folder and the shell come from the terminal's saved record, through
//! the same path as any new terminal), and anything a later format needs of that kind goes into the sealed
//! screen. Only the fact that a checkpoint exists, when it was made, which key sealed it and how long the sealed
//! file is are readable without the key. `ended` appears only in the last generation of a program that ended.

use std::fmt::Write as _;

use super::seal::KeyId;
use super::{FORMAT, READER};

/// The most a manifest may hold when read.
pub const READ_MAX: u64 = 16 * 1024;
/// The longest `ended` value kept.
const VALUE_MAX: usize = 64;

/// The sealed screen file the manifest vouches for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenFile {
    /// The key that sealed it.
    pub key_id: KeyId,
    /// Its size in bytes, so a cut file is told from a whole one before anything is decrypted.
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// Counts up for each checkpoint of a terminal; the newest readable one is restored.
    pub generation: u64,
    /// Unix seconds.
    pub time: u64,
    /// How the program ended (`exit:<status>`, `signal:<number>`), once it has: the last generation a host
    /// writes. A restore draws the screen with its divider and starts a shell, and does not pretend the
    /// program is still there.
    pub ended: Option<String>,
    pub screen: ScreenFile,
}

/// Why a manifest did not read.
#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    /// A line is not `key=value`, or a value is not what its key needs.
    Malformed(&'static str),
    /// A key every reader needs is missing.
    Missing(&'static str),
    /// Written by a build that asks readers to be at least this level (`min_reader`).
    NewerFormat(u32),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(key) => write!(f, "checkpoint manifest: bad value for {key}"),
            Self::Missing(key) => write!(f, "checkpoint manifest: {key} is missing"),
            Self::NewerFormat(n) => {
                write!(f, "checkpoint manifest: needs a reader of level {n}, this build is {READER}")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

/// `value` on one line, cut to [`VALUE_MAX`] bytes at a character, with control characters as `\xNN`.
fn one_line(value: &str) -> String {
    let mut out = String::with_capacity(value.len().min(VALUE_MAX));
    for c in value.chars() {
        if out.len() + 4 > VALUE_MAX {
            break;
        }
        match c {
            '\\' => out.push_str("\\\\"),
            c if c.is_ascii_control() => {
                let _ = write!(out, "\\x{:02x}", c as u8);
            }
            c => out.push(c),
        }
    }
    out
}

/// What [`one_line`] wrote; `None` for an escape it did not write.
fn unescape(value: &str) -> Option<String> {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '\\' => out.push('\\'),
            'x' => {
                let hex: String = chars.by_ref().take(2).collect();
                out.push(char::from(u8::from_str_radix(&hex, 16).ok().filter(|b| *b < 0x20 || *b == 0x7f)?));
            }
            _ => return None,
        }
    }
    Some(out)
}

impl Manifest {
    /// The file's text.
    pub fn render(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "wormhole-checkpoint={FORMAT}");
        let _ = writeln!(text, "min_reader={}", super::MIN_READER);
        let _ = writeln!(text, "generation={}", self.generation);
        let _ = writeln!(text, "time={}", self.time);
        let _ = writeln!(text, "key_id={}", self.screen.key_id);
        let _ = writeln!(text, "screen_len={}", self.screen.len);
        if let Some(ended) = &self.ended {
            let _ = writeln!(text, "ended={}", one_line(ended));
        }
        text
    }

    /// Read a manifest's text. Unknown keys are skipped; a repeated key takes its last value.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let (mut format, mut min_reader, mut generation, mut time) = (None, None, None, None);
        let (mut key_id, mut screen_len, mut ended) = (None, None, None);
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else { continue };
            match key {
                "wormhole-checkpoint" => {
                    format = Some(value.parse::<u32>().map_err(|_| ManifestError::Malformed("format"))?);
                }
                "min_reader" => {
                    min_reader =
                        Some(value.parse::<u32>().map_err(|_| ManifestError::Malformed("min_reader"))?);
                }
                "generation" => {
                    generation =
                        Some(value.parse::<u64>().map_err(|_| ManifestError::Malformed("generation"))?);
                }
                "time" => time = Some(value.parse::<u64>().map_err(|_| ManifestError::Malformed("time"))?),
                "key_id" => key_id = Some(KeyId::parse(value).ok_or(ManifestError::Malformed("key_id"))?),
                "screen_len" => {
                    screen_len =
                        Some(value.parse::<u64>().map_err(|_| ManifestError::Malformed("screen_len"))?);
                }
                "ended" => ended = Some(unescape(value).ok_or(ManifestError::Malformed("ended"))?),
                _ => {}
            }
        }
        // A newer format is read with the keys this build knows, as long as it says it can be: only
        // `min_reader` above this build's level refuses it.
        format.ok_or(ManifestError::Missing("wormhole-checkpoint"))?;
        if let Some(needed) = min_reader.filter(|&needed| needed > READER) {
            return Err(ManifestError::NewerFormat(needed));
        }
        Ok(Self {
            generation: generation.ok_or(ManifestError::Missing("generation"))?,
            time: time.unwrap_or(0),
            ended,
            screen: ScreenFile {
                key_id: key_id.ok_or(ManifestError::Missing("key_id"))?,
                len: screen_len.ok_or(ManifestError::Missing("screen_len"))?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            generation: 7,
            time: 1_760_000_000,
            ended: None,
            screen: ScreenFile { key_id: KeyId([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]), len: 1234 },
        }
    }

    /// The manifest of format 1 as it is written; a change that alters these bytes is a new format.
    const GOLDEN: &str = include_str!("../../tests/vectors/checkpoint-manifest-1.txt");

    #[test]
    fn the_manifest_of_format_1_is_these_bytes() {
        assert_eq!(sample().render(), GOLDEN);
        assert_eq!(Manifest::parse(GOLDEN).expect("reads"), sample());
    }

    /// The plaintext says nothing of what the person did: no folder, no title, no shell, not even a size.
    #[test]
    fn the_manifest_names_only_the_checkpoint_itself() {
        let text = Manifest { ended: Some("exit:0".into()), ..sample() }.render();
        let keys: Vec<&str> = text.lines().filter_map(|l| l.split_once('=')).map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            ["wormhole-checkpoint", "min_reader", "generation", "time", "key_id", "screen_len", "ended"]
        );
    }

    #[test]
    fn a_terminal_whose_program_ended_says_how() {
        let manifest = Manifest { ended: Some("exit:0".into()), ..sample() };
        let text = manifest.render();
        assert!(text.ends_with("\nended=exit:0\n"), "{text:?}");
        assert_eq!(Manifest::parse(&text).expect("reads").ended.as_deref(), Some("exit:0"));
        assert!(!sample().render().contains("ended"));
    }

    #[test]
    fn a_value_stays_on_its_line_and_short() {
        let manifest =
            Manifest { ended: Some(format!("a\nb\r\\c\u{1b}[31m{}", "x".repeat(500))), ..sample() };
        let text = manifest.render();
        assert_eq!(text.lines().count(), 7, "one line each: {text:?}");
        let ended = Manifest::parse(&text).expect("reads").ended.unwrap();
        assert!(ended.starts_with("a\nb\r\\c\u{1b}[31m") && ended.len() <= VALUE_MAX);
    }

    #[test]
    fn unknown_keys_are_skipped_and_the_last_value_wins() {
        let text = format!("{GOLDEN}future=whatever\ntime=5\n");
        assert_eq!(Manifest::parse(&text).expect("reads").time, 5);
    }

    #[test]
    fn a_manifest_that_needs_a_newer_reader_is_refused_and_one_that_only_adds_keys_is_read() {
        let newer = GOLDEN.replace("min_reader=1", "min_reader=2");
        assert_eq!(Manifest::parse(&newer), Err(ManifestError::NewerFormat(2)));
        let added = GOLDEN.replace("wormhole-checkpoint=1", "wormhole-checkpoint=2");
        assert_eq!(Manifest::parse(&added).expect("a newer format that says it can be read"), sample());
    }

    #[test]
    fn what_a_reader_needs_must_be_there() {
        for gone in ["wormhole-checkpoint", "generation", "key_id", "screen_len"] {
            let text: String =
                GOLDEN.lines().filter(|l| !l.starts_with(gone)).map(|l| format!("{l}\n")).collect();
            assert!(matches!(Manifest::parse(&text), Err(ManifestError::Missing(_))), "{gone}");
        }
        assert!(Manifest::parse("generation=x\n").is_err());
    }
}
