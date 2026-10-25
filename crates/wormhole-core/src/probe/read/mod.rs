//! Reading a Probe's folder: the manifest and every file beside it, once, under the caps.
//!
//! The folder is hostile input (a prompt-injected agent may have written it): every file goes
//! through `storage::files::read_capped` with `LinkPolicy::Within(folder)`, so a link that leaves
//! the folder, a FIFO or a file that grew is an error, and the folder is bounded in files, bytes
//! and depth before anything is held in memory. The files are read exactly once and kept, so what
//! is fingerprinted, validated and later approved are the same bytes.
//!
//! Must not use anything above `storage`; it knows no settings and starts nothing.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::manifest::{Manifest, ManifestError, ParseWarning};
use crate::platform::fs as platform_fs;
use crate::storage::files::{self, LinkPolicy, Refusal};

/// The manifest's name; a folder without it is not a Probe.
pub const MANIFEST_FILE: &str = "probe.toml";
/// Files in a Probe's folder, directories not counted.
pub const MAX_FILES: usize = 200;
/// Bytes in all of them: what makes hashing the folder before every spawn cheap.
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024;
/// `probe.toml` alone.
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Entries of every kind (folders included) the walk looks at: a folder of thousands of empty folders
/// is cheap to make and costly to list.
pub const MAX_ENTRIES: usize = 1000;
/// Folders below the Probe's own: more is a build tree, not a Probe.
const MAX_DEPTH: usize = 8;
/// Left out of the folder: Finder writes it on opening a folder, nothing reads it, and counting it
/// would turn an approved Probe into a changed one for having been looked at.
const IGNORED: &[&str] = &[".DS_Store"];

/// One file of a Probe's folder, as read.
#[derive(Clone, PartialEq, Eq)]
pub struct ProbeFile {
    /// Relative to the folder, with `/` between parts.
    pub path: String,
    bytes: Vec<u8>,
    /// Whether the file had an execute bit. Not part of the fingerprint (a mode is not content);
    /// the approved copy keeps it, so a script that ran before still runs from the copy.
    executable: bool,
}

impl ProbeFile {
    pub(crate) fn new(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self { path: path.into(), bytes: bytes.into(), executable: false }
    }

    /// The same file with its execute bit set to `executable`.
    pub(crate) fn with_executable(mut self, executable: bool) -> Self {
        self.executable = executable;
        self
    }

    pub fn is_executable(&self) -> bool {
        self.executable
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn size(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// The content as text, when it is UTF-8.
    pub fn text(&self) -> Option<&str> {
        std::str::from_utf8(&self.bytes).ok()
    }
}

impl fmt::Debug for ProbeFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProbeFile({}, {} bytes)", self.path, self.bytes.len())
    }
}

/// A Probe's folder: its manifest, what parsing it warned about, and every file in it.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeFolder {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub warnings: Vec<ParseWarning>,
    /// Every file, `probe.toml` included, sorted by path.
    pub files: Vec<ProbeFile>,
}

impl ProbeFolder {
    /// Read the Probe in `root`.
    pub fn read(root: &Path) -> Result<Self, ReadError> {
        match fs::metadata(root) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => return Err(ReadError::NotAFolder(root.to_path_buf())),
            Err(source) => return Err(ReadError::Io { path: root.to_path_buf(), source }),
        }
        let mut found = Vec::new();
        list(root, "", 0, &mut found, &mut 0)?;
        found.sort_by(|a, b| a.0.cmp(&b.0));

        let mut files = Vec::with_capacity(found.len());
        let mut total = 0u64;
        for (path, abs) in found {
            // The manifest has a cap of its own, and the folder's total caps it as well.
            let remaining = MAX_TOTAL_BYTES - total;
            let (cap, total_is_the_limit) = if path == MANIFEST_FILE {
                (MAX_MANIFEST_BYTES.min(remaining), remaining < MAX_MANIFEST_BYTES)
            } else {
                (remaining, true)
            };
            let bytes = files::read_capped(&abs, cap, LinkPolicy::Within(root))
                .map_err(|err| read_error(&path, &abs, err, total_is_the_limit))?;
            total += bytes.len() as u64;
            let mut file = ProbeFile::new(path, bytes);
            // Through a link to the file it names, as the read did.
            file.executable = fs::metadata(&abs).is_ok_and(|meta| platform_fs::is_executable(&meta));
            files.push(file);
        }

        let manifest_file =
            files.iter().find(|file| file.path == MANIFEST_FILE).ok_or(ReadError::NoManifest)?;
        let text = std::str::from_utf8(manifest_file.bytes()).map_err(|_| {
            ReadError::Manifest(ManifestError {
                message: "probe.toml is not valid UTF-8".into(),
                line: None,
                column: None,
            })
        })?;
        let parsed = Manifest::parse(text).map_err(ReadError::Manifest)?;
        Ok(Self { root: root.to_path_buf(), manifest: parsed.manifest, warnings: parsed.warnings, files })
    }

    /// The file at `path` (relative, `/`-separated), when the folder has it.
    pub fn file(&self, path: &str) -> Option<&ProbeFile> {
        self.files.iter().find(|file| file.path == path)
    }
}

/// The files below `dir` as (relative path, real path), at most [`MAX_FILES`].
fn list(
    dir: &Path,
    prefix: &str,
    depth: usize,
    out: &mut Vec<(String, PathBuf)>,
    entries: &mut usize,
) -> Result<(), ReadError> {
    let io_err = |source| ReadError::Io { path: dir.to_path_buf(), source };
    for entry in fs::read_dir(dir).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        *entries += 1;
        if *entries > MAX_ENTRIES {
            return Err(ReadError::TooManyEntries);
        }
        let name = entry.file_name();
        let relative =
            |name: &str| if prefix.is_empty() { name.to_string() } else { format!("{prefix}/{name}") };
        let Some(name) = name.to_str().filter(|name| !super::hidden::has_hidden(name)) else {
            return Err(ReadError::BadName(relative(&name.to_string_lossy())));
        };
        if IGNORED.contains(&name) {
            continue;
        }
        let path = relative(name);
        let abs = entry.path();
        let meta =
            fs::symlink_metadata(&abs).map_err(|source| ReadError::Io { path: abs.clone(), source })?;
        let is_dir = if meta.file_type().is_symlink() {
            // A link to a file is read (and kept inside the folder) by `read_capped`; a link to a
            // folder would need walking, and a walk is exactly what a link could steer.
            let target = fs::metadata(&abs).map_err(|source| ReadError::Io { path: abs.clone(), source })?;
            if target.is_dir() {
                return Err(ReadError::FolderLink(path));
            }
            false
        } else {
            meta.is_dir()
        };
        if is_dir {
            if depth >= MAX_DEPTH {
                return Err(ReadError::TooDeep(path));
            }
            list(&abs, &path, depth + 1, out, entries)?;
        } else {
            out.push((path, abs));
            if out.len() > MAX_FILES {
                return Err(ReadError::TooManyFiles);
            }
        }
    }
    Ok(())
}

fn read_error(path: &str, abs: &Path, err: io::Error, total_is_the_limit: bool) -> ReadError {
    match files::refusal(&err) {
        Some(Refusal::OutsideRoot(_)) => ReadError::LeavesFolder(path.to_string()),
        Some(Refusal::TooLarge(_)) if total_is_the_limit => ReadError::TooMuch,
        Some(refusal) => ReadError::Refused { path: path.to_string(), reason: refusal.to_string() },
        None => ReadError::Io { path: abs.to_path_buf(), source: err },
    }
}

/// Why a folder is not a Probe that can be read.
#[derive(Debug)]
pub enum ReadError {
    NotAFolder(PathBuf),
    Io {
        path: PathBuf,
        source: io::Error,
    },
    /// A file that resolves, through a link, outside the folder.
    LeavesFolder(String),
    /// A link to a folder: not followed.
    FolderLink(String),
    /// A file the read refused (a FIFO, a device, `probe.toml` over its cap).
    Refused {
        path: String,
        reason: String,
    },
    TooManyFiles,
    /// More than [`MAX_ENTRIES`] files and folders together.
    TooManyEntries,
    TooMuch,
    TooDeep(String),
    /// A file name that is not UTF-8 or holds a control, bidi or invisible character.
    BadName(String),
    NoManifest,
    Manifest(ManifestError),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFolder(path) => write!(f, "{} is not a folder", path.display()),
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::LeavesFolder(path) => write!(f, "{path} is a link that leads out of the Probe's folder"),
            Self::FolderLink(path) => write!(f, "{path} is a link to a folder, which is not followed"),
            Self::Refused { path, reason } => write!(f, "{path} {reason}"),
            Self::TooManyFiles => write!(f, "the folder has more than {MAX_FILES} files"),
            Self::TooManyEntries => write!(f, "the folder has more than {MAX_ENTRIES} files and folders"),
            Self::TooMuch => write!(f, "the folder holds more than {} MiB", MAX_TOTAL_BYTES >> 20),
            Self::TooDeep(path) => write!(f, "{path} is more than {MAX_DEPTH} folders deep"),
            Self::BadName(name) => write!(f, "{name:?} is not a usable file name"),
            Self::NoManifest => write!(f, "the folder has no {MANIFEST_FILE}"),
            Self::Manifest(err) => write!(f, "{MANIFEST_FILE}:{err}"),
        }
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Manifest(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
