//! A checkpoint's directory: generations of a manifest and a sealed screen, written so that a crash at any
//! point leaves the last generation whole, and read newest first.
//!
//! `<dir>/g<generation>.screen` is written first (a temporary file in the directory, then renamed), the
//! manifest `<dir>/g<generation>.manifest` last: a manifest is the commit of its generation, and a screen
//! without one is a write that never finished and is cleaned up. The newest [`KEEP`] generations are kept.
//! The directory is `0700` and the files `0600`.
//!
//! The reader takes the newest generation whose manifest reads and whose screen opens. A generation that
//! needs a newer reader stops it (`Newer`): the app falls back to its text and the directory stays for the
//! newer build. A generation that is damaged or sealed with another key is skipped for an older one; when
//! none has a screen that opens, the newest manifest is still returned with the reason, because the folder
//! and the shell are worth restoring without it.

use std::fs::{self, DirBuilder, OpenOptions};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use super::manifest::{Manifest, ManifestError, READ_MAX, ScreenFile};
use super::seal::{self, Binding, Key, OpenError};
use crate::host_id::valid as valid_id;
use crate::sys::fs::{private_dir, private_file};

/// How many generations are kept: the newest, and the one before it for when the newest is damaged.
pub const KEEP: usize = 2;
/// The most a sealed screen file may hold when read.
pub const SCREEN_MAX: u64 = 96 * 1024 * 1024;

/// What a checkpoint writes about a terminal at one moment.
pub struct Snapshot<'a> {
    pub time: u64,
    pub cols: u16,
    pub rows: u16,
    pub cwd: Option<&'a str>,
    pub shell: Option<&'a str>,
    pub title: Option<&'a str>,
    /// How the program ended, once it has.
    pub ended: Option<&'a str>,
    /// The primary screen as VT bytes (`vt::primary_screen`); sealed when written, and not written at all
    /// when there is no key.
    pub screen: Option<&'a [u8]>,
}

/// What one write put on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    pub generation: u64,
    /// The bytes written, for the host's budget.
    pub bytes: u64,
    /// Whether a sealed screen went with the manifest.
    pub screen: bool,
}

/// `<sessions dir>/<id>`: where a terminal's checkpoint is.
pub fn dir_in(sessions_dir: &Path, id: &str) -> PathBuf {
    sessions_dir.join(id)
}

fn manifest_name(generation: u64) -> String {
    format!("g{generation:012}.manifest")
}

fn screen_name(generation: u64) -> String {
    format!("g{generation:012}.screen")
}

/// The generation a file name belongs to and whether it is the screen's: `None` for any other name.
fn parse_name(name: &str) -> Option<(u64, bool)> {
    let rest = name.strip_prefix('g')?;
    let (digits, kind) = rest.split_once('.')?;
    if digits.len() != 12 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let screen = match kind {
        "manifest" => false,
        "screen" => true,
        _ => return None,
    };
    Some((digits.parse().ok()?, screen))
}

/// The generations in `dir` that have a manifest, newest first, and the screen files that have none.
fn survey(dir: &Path) -> io::Result<(Vec<u64>, Vec<u64>)> {
    let (mut manifests, mut screens) = (Vec::new(), Vec::new());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let Some((generation, screen)) = entry.file_name().to_str().and_then(parse_name) else { continue };
        if screen { screens.push(generation) } else { manifests.push(generation) }
    }
    manifests.sort_unstable_by(|a, b| b.cmp(a));
    let orphans = screens.into_iter().filter(|g| !manifests.contains(g)).collect();
    Ok((manifests, orphans))
}

/// A `0600` file written whole under a temporary name in its directory and renamed over its name.
fn write_atomic(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let temp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    // A leftover of a crashed write by a process with this pid is replaced.
    let _ = fs::remove_file(&temp);
    let mut file = private_file(OpenOptions::new().write(true).create_new(true)).open(&temp)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let renamed = written.and_then(|()| fs::rename(&temp, dir.join(name)));
    if renamed.is_err() {
        let _ = fs::remove_file(&temp);
    }
    renamed
}

/// A terminal's checkpoint directory, open for writing.
pub struct Store {
    dir: PathBuf,
    id: String,
    next: u64,
}

impl Store {
    /// Open (making it, `0700`) `<sessions dir>/<id>`. The next generation is one past the newest there.
    pub fn open(sessions_dir: &Path, id: &str) -> io::Result<Self> {
        if !valid_id(id) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a host id"));
        }
        let dir = dir_in(sessions_dir, id);
        let mut builder = DirBuilder::new();
        private_dir(builder.recursive(true)).create(&dir)?;
        let (manifests, orphans) = survey(&dir)?;
        let next = manifests.iter().chain(&orphans).max().map_or(1, |g| g + 1);
        Ok(Self { dir, id: id.to_owned(), next })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Write `snapshot` as the next generation and drop the generations past [`KEEP`]. With no `key` only the
    /// manifest is written: the folder and the shell, never a screen, and never an unsealed one.
    pub fn write(&mut self, key: Option<&Key>, snapshot: &Snapshot<'_>) -> io::Result<Written> {
        // The app removes a checkpoint when the buffer is cleared, under a host that goes on writing: the
        // directory comes back with the next generation, and its numbers go on.
        if !self.dir.is_dir() {
            let mut builder = DirBuilder::new();
            private_dir(builder.recursive(true)).create(&self.dir)?;
        }
        let generation = self.next;
        let mut bytes = 0u64;
        let mut screen = None;
        if let (Some(key), Some(vt)) = (key, snapshot.screen) {
            let binding = Binding { id: &self.id, generation, key_id: key.id() };
            let sealed = seal::seal(key, &binding, vt)?;
            write_atomic(&self.dir, &screen_name(generation), &sealed)?;
            bytes += sealed.len() as u64;
            screen = Some(ScreenFile { key_id: key.id(), len: sealed.len() as u64 });
        }
        let manifest = Manifest {
            generation,
            time: snapshot.time,
            cols: snapshot.cols,
            rows: snapshot.rows,
            cwd: snapshot.cwd.map(str::to_owned),
            shell: snapshot.shell.map(str::to_owned),
            title: snapshot.title.map(str::to_owned),
            ended: snapshot.ended.map(str::to_owned),
            screen,
        };
        let text = manifest.render();
        write_atomic(&self.dir, &manifest_name(generation), text.as_bytes())?;
        bytes += text.len() as u64;
        self.next = generation + 1;
        self.prune();
        Ok(Written { generation, bytes, screen: screen.is_some() })
    }

    /// Remove the generations past [`KEEP`], screens whose manifest never came, and temporary files. Best
    /// effort: what cannot be removed is tried again at the next write.
    fn prune(&self) {
        let Ok((manifests, orphans)) = survey(&self.dir) else { return };
        let old = manifests.iter().skip(KEEP).copied();
        let stray = orphans.into_iter().filter(|g| *g < self.next - 1);
        for generation in old.clone().chain(stray.clone()) {
            let _ = fs::remove_file(self.dir.join(screen_name(generation)));
        }
        for generation in old {
            let _ = fs::remove_file(self.dir.join(manifest_name(generation)));
        }
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let leftover = name.to_str().is_some_and(|n| n.starts_with('.') && n.ends_with(".tmp"));
                if leftover && !name.to_string_lossy().contains(&format!(".{}.", std::process::id())) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}

/// Why a checkpoint's screen was not brought back (its manifest may have been).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Issue {
    /// The checkpoint has a sealed screen and the reader has no key.
    NoKey,
    /// It was sealed with another key (the manifest says which): the profile's key changed.
    WrongKey,
    /// The file is missing, cut, changed or not a sealed screen.
    Damaged,
}

/// A checkpoint as read.
#[derive(Debug)]
pub struct Restored {
    pub manifest: Manifest,
    /// The screen's VT bytes, when the generation had one and it opened.
    pub screen: Option<Zeroizing<Vec<u8>>>,
    /// Why there is no screen though a checkpoint was written with one.
    pub issue: Option<Issue>,
}

/// Why nothing was read.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// No checkpoint (no directory, or no generation in it).
    Missing,
    /// The newest generation was written by a build that needs a newer reader (this level).
    Newer(u32),
    /// Generations exist and none reads.
    Unusable,
}

fn read_capped(path: &Path, max: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "too big"));
    }
    Ok(bytes)
}

/// The screen of `manifest`'s generation, opened with `key`.
fn open_screen(
    dir: &Path,
    id: &str,
    manifest: &Manifest,
    key: Option<&Key>,
) -> Result<Zeroizing<Vec<u8>>, Issue> {
    let (Some(file), Some(key)) = (&manifest.screen, key) else { return Err(Issue::NoKey) };
    if file.key_id != key.id() {
        return Err(Issue::WrongKey);
    }
    if file.len > SCREEN_MAX {
        return Err(Issue::Damaged);
    }
    let sealed =
        read_capped(&dir.join(screen_name(manifest.generation)), SCREEN_MAX).map_err(|_| Issue::Damaged)?;
    if sealed.len() as u64 != file.len {
        return Err(Issue::Damaged);
    }
    let binding = Binding { id, generation: manifest.generation, key_id: file.key_id };
    seal::open(key, &binding, &sealed).map_err(|e| match e {
        OpenError::NotSealed | OpenError::Format(_) | OpenError::Refused => Issue::Damaged,
    })
}

/// The newest checkpoint of terminal `id` in `dir` (its checkpoint directory) that can be used: see the
/// module.
pub fn read_latest(dir: &Path, id: &str, key: Option<&Key>) -> Result<Restored, ReadError> {
    let Ok((manifests, _)) = survey(dir) else { return Err(ReadError::Missing) };
    if manifests.is_empty() {
        return Err(ReadError::Missing);
    }
    // The newest manifest that reads, kept in case no generation has a screen that opens.
    let mut best: Option<Restored> = None;
    for generation in manifests {
        let Ok(bytes) = read_capped(&dir.join(manifest_name(generation)), READ_MAX) else { continue };
        let Ok(text) = std::str::from_utf8(&bytes) else { continue };
        let manifest = match Manifest::parse(text) {
            Ok(manifest) => manifest,
            // The newest generation is a newer build's: not read, and left for it.
            Err(ManifestError::NewerFormat(n)) if best.is_none() => return Err(ReadError::Newer(n)),
            Err(_) => continue,
        };
        if manifest.generation != generation {
            continue; // a manifest copied under another name
        }
        if manifest.screen.is_none() {
            // Written without a key (or with nothing to say of the screen): the folder and the shell.
            let restored = Restored { manifest, screen: None, issue: None };
            return Ok(best.unwrap_or(restored));
        }
        match open_screen(dir, id, &manifest, key) {
            Ok(screen) => return Ok(Restored { manifest, screen: Some(screen), issue: None }),
            Err(issue) => {
                if best.is_none() {
                    best = Some(Restored { manifest, screen: None, issue: Some(issue) });
                }
            }
        }
    }
    best.ok_or(ReadError::Unusable)
}

/// Remove a terminal's checkpoint: its files, then its directory. For a terminal that is closed, cleared or
/// forgotten; never for one that is only restored (the checkpoint stays until the new host has its own).
pub fn forget(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|t| t.is_file()) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}
