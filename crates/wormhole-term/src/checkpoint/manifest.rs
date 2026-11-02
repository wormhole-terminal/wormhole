//! A checkpoint's manifest: what a restore needs to know before it opens the sealed screen, and what is
//! useful without it (the folder and the shell). UTF-8 text, one `key=value` per line, unknown keys
//! ignored by readers, `0600`; the same conventions as a host's meta file (`hosted::meta`).
//!
//! ```text
//! wormhole-checkpoint=1
//! min_reader=1
//! generation=7
//! time=1760000000
//! cols=80
//! rows=24
//! cwd=/Users/me/src/project
//! shell=login
//! title=cargo test
//! key_id=0123456789abcdef
//! screen_len=1234
//! ```
//!
//! `cwd`, `shell` and `title` are escaped so a value is always one line (`\\`, `\n`, `\r` and other control
//! characters as `\xNN`). `key_id` and `screen_len` are there only when a sealed screen file goes with the
//! manifest. Nothing of the program's environment is stored, ever: a restore derives its own.

use std::fmt::Write as _;

use super::seal::KeyId;
use super::{FORMAT, READER};

/// The most a manifest may hold when read.
pub const READ_MAX: u64 = 16 * 1024;
/// The longest a value (folder, shell, title) is kept.
pub const VALUE_MAX: usize = 4096;

/// A sealed screen file the manifest vouches for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenFile {
    /// The key that sealed it.
    pub key_id: KeyId,
    /// Its size in bytes, so a cut file is told from a whole one before anything is decrypted.
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Manifest {
    /// Counts up for each checkpoint of a terminal; the newest readable one is restored.
    pub generation: u64,
    /// Unix seconds.
    pub time: u64,
    pub cols: u16,
    pub rows: u16,
    /// The folder the shell was in (not trusted at restore: see `restore`).
    pub cwd: Option<String>,
    /// The key of the shell the terminal ran (`ShellChoice::key`, or `login`).
    pub shell: Option<String>,
    pub title: Option<String>,
    pub screen: Option<ScreenFile>,
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

/// `value` on one line.
fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
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

/// What [`escape`] wrote; `None` for an escape it did not write.
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

/// A value cut to [`VALUE_MAX`] bytes at a character boundary.
fn capped(value: &str) -> &str {
    if value.len() <= VALUE_MAX {
        return value;
    }
    let mut end = VALUE_MAX;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

impl Manifest {
    /// The file's text.
    pub fn render(&self) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "wormhole-checkpoint={FORMAT}");
        let _ = writeln!(text, "min_reader={}", super::MIN_READER);
        let _ = writeln!(text, "generation={}", self.generation);
        let _ = writeln!(text, "time={}", self.time);
        let _ = writeln!(text, "cols={}", self.cols);
        let _ = writeln!(text, "rows={}", self.rows);
        for (key, value) in [("cwd", &self.cwd), ("shell", &self.shell), ("title", &self.title)] {
            if let Some(value) = value {
                let _ = writeln!(text, "{key}={}", escape(capped(value)));
            }
        }
        if let Some(screen) = &self.screen {
            let _ = writeln!(text, "key_id={}", screen.key_id);
            let _ = writeln!(text, "screen_len={}", screen.len);
        }
        text
    }

    /// Read a manifest's text. Unknown keys are skipped; a repeated key takes its last value.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let mut manifest = Self::default();
        let (mut format, mut min_reader) = (None, None);
        let (mut generation, mut time, mut cols, mut rows) = (None, None, None, None);
        let (mut key_id, mut screen_len) = (None, None);
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
                "cols" => cols = Some(value.parse::<u16>().map_err(|_| ManifestError::Malformed("cols"))?),
                "rows" => rows = Some(value.parse::<u16>().map_err(|_| ManifestError::Malformed("rows"))?),
                "cwd" => manifest.cwd = Some(unescape(value).ok_or(ManifestError::Malformed("cwd"))?),
                "shell" => manifest.shell = Some(unescape(value).ok_or(ManifestError::Malformed("shell"))?),
                "title" => manifest.title = Some(unescape(value).ok_or(ManifestError::Malformed("title"))?),
                "key_id" => key_id = Some(KeyId::parse(value).ok_or(ManifestError::Malformed("key_id"))?),
                "screen_len" => {
                    screen_len =
                        Some(value.parse::<u64>().map_err(|_| ManifestError::Malformed("screen_len"))?);
                }
                _ => {}
            }
        }
        // A newer format is read with the keys this build knows, as long as it says it can be: only
        // `min_reader` above this build's level refuses it.
        format.ok_or(ManifestError::Missing("wormhole-checkpoint"))?;
        if let Some(needed) = min_reader.filter(|&needed| needed > READER) {
            return Err(ManifestError::NewerFormat(needed));
        }
        manifest.generation = generation.ok_or(ManifestError::Missing("generation"))?;
        manifest.time = time.unwrap_or(0);
        manifest.cols = cols.ok_or(ManifestError::Missing("cols"))?;
        manifest.rows = rows.ok_or(ManifestError::Missing("rows"))?;
        manifest.screen = match (key_id, screen_len) {
            (Some(key_id), Some(len)) => Some(ScreenFile { key_id, len }),
            (None, None) => None,
            _ => return Err(ManifestError::Malformed("screen_len")),
        };
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Manifest {
        Manifest {
            generation: 7,
            time: 1_760_000_000,
            cols: 80,
            rows: 24,
            cwd: Some("/Users/me/src/project".into()),
            shell: Some("login".into()),
            title: Some("cargo test".into()),
            screen: Some(ScreenFile {
                key_id: KeyId([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
                len: 1234,
            }),
        }
    }

    /// The manifest of format 1 as it is written; a change that alters these bytes is a new format.
    const GOLDEN: &str = include_str!("../../tests/vectors/checkpoint-manifest-1.txt");

    #[test]
    fn the_manifest_of_format_1_is_these_bytes() {
        assert_eq!(sample().render(), GOLDEN);
        assert_eq!(Manifest::parse(GOLDEN).expect("reads"), sample());
    }

    #[test]
    fn values_with_control_characters_stay_on_their_line() {
        let manifest = Manifest { title: Some("a\nb\r\\c\u{1b}[31m".into()), ..sample() };
        let text = manifest.render();
        assert_eq!(text.lines().count(), 11, "one line each: {text:?}");
        assert_eq!(Manifest::parse(&text).expect("reads").title, manifest.title);
    }

    #[test]
    fn unknown_keys_are_skipped_and_the_last_value_wins() {
        let text = format!("{GOLDEN}future=whatever\ncols=100\n");
        let manifest = Manifest::parse(&text).expect("reads");
        assert_eq!((manifest.cols, manifest.rows), (100, 24));
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
        for gone in ["wormhole-checkpoint", "generation", "cols", "rows"] {
            let text: String =
                GOLDEN.lines().filter(|l| !l.starts_with(gone)).map(|l| format!("{l}\n")).collect();
            assert!(matches!(Manifest::parse(&text), Err(ManifestError::Missing(_))), "{gone}");
        }
        // A key id without a length (or the reverse) is a damaged manifest.
        let text: String =
            GOLDEN.lines().filter(|l| !l.starts_with("screen_len")).map(|l| format!("{l}\n")).collect();
        assert!(matches!(Manifest::parse(&text), Err(ManifestError::Malformed(_))));
        assert!(Manifest::parse("generation=x\n").is_err());
    }

    #[test]
    fn a_value_is_capped_at_a_character_boundary() {
        let manifest = Manifest { title: Some("é".repeat(VALUE_MAX)), ..sample() };
        let read = Manifest::parse(&manifest.render()).expect("reads");
        let title = read.title.expect("a title");
        assert!(title.len() <= VALUE_MAX && title.chars().all(|c| c == 'é'));
    }

    #[test]
    fn a_manifest_without_a_screen_has_neither_key_nor_length() {
        let text = Manifest { screen: None, ..sample() }.render();
        assert!(!text.contains("key_id") && !text.contains("screen_len"));
        assert_eq!(Manifest::parse(&text).expect("reads").screen, None);
    }
}
