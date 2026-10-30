//! Filesystem primitives for everything that reads or writes user and project files.
//!
//! Two problems recur across the code base and are solved once here:
//!
//! - **Writes must be atomic and must not be steerable.** A fixed temp name next to the target
//!   (`notes.md.tmp`) lets a cloned repository plant a symlink there and have us write through
//!   it; a plain `fs::write` truncates in place and a crash leaves half a file. [`write_atomic`]
//!   creates a uniquely named temp file (exclusive create, so a planted link is never followed),
//!   syncs it, keeps the permissions of the file it replaces and renames over the target.
//! - **Reads from a repository are untrusted input.** [`read_capped`] bounds the size, refuses
//!   anything that is not a regular file (a FIFO would block forever) and, with
//!   [`LinkPolicy::Within`], refuses paths that resolve outside the project root.
//!
//! [`list_folders`] reads a folder's subfolders for a list to pick from, under caps. [`FileStamp`] tells,
//! from metadata alone, whether a file may have changed since it was read, so pollers re-read only what
//! changed.

mod clone;
mod folders;
pub mod listing;
mod prefix;
mod private;
mod rename;
mod viewing;

pub use clone::clone_dir;
pub use folders::{
    Cut, FOLDERS_CAP, Folders, SCAN_CAP, has_folder, hostile_char, list_folders, list_folders_where,
};
pub use prefix::{Prefix, read_prefix_capped};
pub use private::{
    PRIVATE_FILE, create_dir_all_private, create_private_file, ensure_private_dir, make_private,
    make_tree_private,
};
pub use rename::{MAX_NAME_BYTES, NameProblem, RenameError, check_name, rename_within};
pub use viewing::{IMAGE_CAP, ImageKind, Loaded, Problem, TEXT_CAP, load};

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::platform::fs::{self as platform_fs, Stamp};

/// How symlinks are treated.
#[derive(Clone, Copy, Debug)]
pub enum LinkPolicy<'a> {
    /// The user's own files (`config.json`, `~/.claude/settings.json`): a symlinked file is a
    /// dotfiles setup, so writes go through the link and the link stays. A new file, and a folder
    /// made for it, are owner-only (`0600`, `0700`; a program see `write_atomic_executable`); one
    /// that exists keeps its mode.
    Follow,
    /// Files that come from a repository: the path, with all links resolved, must stay inside
    /// this root. Writes replace a symlinked final component instead of following it.
    Within(&'a Path),
}

/// Why a read or write was refused rather than attempted. It travels inside the `io::Error`
/// the functions here return (see [`refusal`]), so callers can tell it from a failing disk and
/// word it for their users.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// With links resolved, the path is outside this [`LinkPolicy::Within`] root.
    OutsideRoot(PathBuf),
    /// Bigger than the cap given to [`read_capped`].
    TooLarge(u64),
    /// A directory, FIFO, socket or device.
    NotRegularFile,
    /// [`read_to_string_capped`] only.
    NotUtf8,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideRoot(root) => write!(f, "resolves outside {}", root.display()),
            Self::TooLarge(cap) if cap % (1 << 20) == 0 => write!(f, "is larger than {} MiB", cap >> 20),
            Self::TooLarge(cap) if cap % (1 << 10) == 0 => write!(f, "is larger than {} KiB", cap >> 10),
            Self::TooLarge(cap) => write!(f, "is larger than {cap} bytes"),
            Self::NotRegularFile => f.write_str("is not a regular file"),
            Self::NotUtf8 => f.write_str("is not valid UTF-8"),
        }
    }
}

/// The error payload behind [`refusal`].
#[derive(Debug)]
struct Refused {
    path: PathBuf,
    refusal: Refusal,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.path.display(), self.refusal)
    }
}

impl std::error::Error for Refused {}

fn refuse(path: &Path, refusal: Refusal) -> io::Error {
    let kind = match refusal {
        Refusal::OutsideRoot(_) => io::ErrorKind::PermissionDenied,
        Refusal::NotRegularFile => io::ErrorKind::InvalidInput,
        Refusal::TooLarge(_) | Refusal::NotUtf8 => io::ErrorKind::InvalidData,
    };
    io::Error::new(kind, Refused { path: path.to_path_buf(), refusal })
}

/// The reason, when `err` is a refusal by this module rather than an I/O failure.
pub fn refusal(err: &io::Error) -> Option<&Refusal> {
    err.get_ref()?.downcast_ref::<Refused>().map(|refused| &refused.refusal)
}

fn escapes(path: &Path, root: &Path) -> io::Error {
    refuse(path, Refusal::OutsideRoot(root.to_path_buf()))
}

/// Replace `path` with `bytes`: readers (and a crash) see the old or the new content, never a mix.
pub fn write_atomic(path: &Path, bytes: &[u8], policy: LinkPolicy<'_>) -> io::Result<()> {
    write_file(path, bytes, policy, false)
}

/// [`write_atomic`] for a script that is run: the new file is `0777` less the umask, whatever the
/// policy, so a command installed for everyone (`/usr/local/bin/wh`) stays runnable by other
/// accounts; one in Wormhole's data folder is kept private by its `0700` folder. A file it
/// replaces keeps its mode and gains execute wherever it could be read. Made
/// executable before the rename, so there is never a moment the name holds a script that can't run.
pub fn write_atomic_executable(path: &Path, bytes: &[u8], policy: LinkPolicy<'_>) -> io::Result<()> {
    write_file(path, bytes, policy, true)
}

fn write_file(path: &Path, bytes: &[u8], policy: LinkPolicy<'_>, executable: bool) -> io::Result<()> {
    let (mut dir, mut target) = locate_for_write(path, policy)?;
    match policy {
        // The user's and Wormhole's own files: a folder made for them is owner-only.
        LinkPolicy::Follow => create_dir_all_private(&dir)?,
        LinkPolicy::Within(_) => fs::create_dir_all(&dir)?,
    }
    // Re-check after creating: a missing directory chain could not be canonicalized before.
    if let LinkPolicy::Within(root) = policy {
        let real_dir = fs::canonicalize(&dir)?;
        if !real_dir.starts_with(fs::canonicalize(root)?) {
            return Err(escapes(path, root));
        }
        // Work in the directory that was checked, not one reached through links again later.
        let name = target.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("{} names no file", path.display()))
        })?;
        target = real_dir.join(name);
        dir = real_dir;
    }

    let mut builder = tempfile::Builder::new();
    builder.prefix(".wormhole-").suffix(".tmp");
    // A new file of the user's or Wormhole's own (`Follow`: settings, profiles, the data
    // folder's files) is owner-only; one in a repository (`Within`) gets the usual
    // 0666 &! umask, as any tool there would make it.
    // A program is `0777` less the umask either way (see `write_atomic_executable`).
    let mode = match (policy, executable) {
        (_, true) => 0o777,
        (LinkPolicy::Follow, false) => private::PRIVATE_FILE,
        (LinkPolicy::Within(_), false) => 0o666,
    };
    builder.permissions(platform_fs::permissions(mode));
    let mut tmp = builder.tempfile_in(&dir)?;
    tmp.write_all(bytes)?;
    // Keep what the user had: a 0600 settings file must not become world-readable. Under
    // `Within` a link is replaced, not followed, so its target's mode is none of our business.
    let existing = match policy {
        LinkPolicy::Follow => fs::metadata(&target),
        LinkPolicy::Within(_) => fs::symlink_metadata(&target),
    };
    if let Ok(existing) = existing
        && existing.is_file()
    {
        let mut mode = existing.permissions();
        if executable {
            let bits = platform_fs::mode(&existing);
            mode = platform_fs::permissions(bits | ((bits & 0o444) >> 2));
        }
        tmp.as_file().set_permissions(mode)?;
    }
    tmp.as_file().sync_all()?;
    tmp.persist(&target).map_err(|e| e.error)?;
    // Best effort: make the rename itself durable.
    if let Ok(dir) = File::open(&dir) {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// The directory to put the temp file in and the path to rename onto.
fn locate_for_write(path: &Path, policy: LinkPolicy<'_>) -> io::Result<(PathBuf, PathBuf)> {
    let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    match policy {
        LinkPolicy::Follow => {
            let target = match fs::symlink_metadata(path) {
                Ok(meta) if meta.file_type().is_symlink() => fs::canonicalize(path)?,
                _ => path.to_path_buf(),
            };
            let dir = target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            Ok((dir.to_path_buf(), target))
        }
        LinkPolicy::Within(root) => {
            // Resolve the deepest existing ancestor so a symlinked directory is caught even
            // before the chain below it exists.
            let mut existing = parent;
            while !existing.exists() {
                match existing.parent() {
                    Some(up) if !up.as_os_str().is_empty() => existing = up,
                    _ => break,
                }
            }
            let real =
                fs::canonicalize(if existing.as_os_str().is_empty() { Path::new(".") } else { existing })?;
            if !real.starts_with(fs::canonicalize(root)?) {
                return Err(escapes(path, root));
            }
            // The rename replaces a symlinked final component rather than following it.
            Ok((parent.to_path_buf(), path.to_path_buf()))
        }
    }
}

/// The real path of the regular file `relative` names inside `root`: for handing a repository's
/// file to another program (an editor), which then opens whatever the path leads to. `relative`
/// must be a plain relative path (no `..`, no root, no NUL); with every link resolved the result
/// must still be inside `root`, so a link a cloned repository planted can't point an editor at a
/// file elsewhere on the disk. The result is the resolved path, which is what was checked.
pub fn resolve_within(root: &Path, relative: &str) -> io::Result<PathBuf> {
    if relative.contains('\0') {
        return Err(escapes(Path::new(relative), root));
    }
    let real = resolve_path_within(root, Path::new(relative))?;
    if !fs::metadata(&real)?.is_file() {
        return Err(refuse(&root.join(relative), Refusal::NotRegularFile));
    }
    Ok(real)
}

/// [`resolve_within`] for whatever `relative` names: a file or a folder. For a path that comes from
/// the disk and need not be UTF-8 (the Files tab's rows). The caller looks at what the real path is.
pub fn resolve_path_within(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    use std::path::Component;
    let plain = !relative.as_os_str().is_empty()
        && relative.components().all(|part| matches!(part, Component::Normal(_)));
    if !plain {
        return Err(escapes(relative, root));
    }
    let wanted = root.join(relative);
    let real = fs::canonicalize(&wanted)?;
    if !real.starts_with(fs::canonicalize(root)?) {
        return Err(escapes(&wanted, root));
    }
    Ok(real)
}

/// Read at most `cap` bytes of a regular file; larger files are an error, not a truncation.
pub fn read_capped(path: &Path, cap: u64, policy: LinkPolicy<'_>) -> io::Result<Vec<u8>> {
    let file = open_regular(path, policy)?;
    let meta = file.metadata()?;
    if meta.len() > cap {
        return Err(too_large(path, cap));
    }
    // The size can change between the check and the read; `take` keeps the cap honest.
    // Pre-size from the file but never beyond a sane bound: the cap may be u64::MAX.
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len().min(1 << 20)).unwrap_or(0));
    file.take(cap.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(too_large(path, cap));
    }
    Ok(bytes)
}

/// Read only the last `cap` bytes of an append-only log, without loading its history.
/// A partial first/last record is left to the caller. The same regular-file and link checks
/// as `read_capped` apply, so an agent-supplied path cannot block a hook on a FIFO.
pub fn read_tail_capped(path: &Path, cap: u64, policy: LinkPolicy<'_>) -> io::Result<Vec<u8>> {
    let mut file = open_regular(path, policy)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(cap)))?;
    let mut bytes = Vec::new();
    file.take(cap).read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn open_regular(path: &Path, policy: LinkPolicy<'_>) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    // Opening a FIFO blocks until a writer shows up.
    platform_fs::no_block(&mut options);
    let open_path = match policy {
        LinkPolicy::Follow => path.to_path_buf(),
        LinkPolicy::Within(root) => {
            let real = fs::canonicalize(path)?;
            if !real.starts_with(fs::canonicalize(root)?) {
                return Err(escapes(path, root));
            }
            // Open what was checked; a link swapped in at the last component since then fails.
            platform_fs::no_block_no_follow(&mut options);
            real
        }
    };
    // Not even opened unless it is a regular file: opening a device or a FIFO can have effects of its own
    // (a tty, a tape, a socket's peer). `O_NONBLOCK` and the look at the open file below stay as the
    // guard for a path swapped for one of those in between.
    if !fs::metadata(&open_path)?.is_file() {
        return Err(refuse(path, Refusal::NotRegularFile));
    }
    let file = options.open(&open_path)?;
    if !file.metadata()?.is_file() {
        return Err(refuse(path, Refusal::NotRegularFile));
    }
    Ok(file)
}

/// Remove the file `path`, a direct child of the real folder `dir`: nothing is removed when `dir`
/// is a link (to somewhere else), when `path` is not directly in it, or when it is a folder. A link
/// at `path` is removed itself, never what it points to.
pub fn remove_file_within(dir: &Path, path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(dir)?.is_dir() {
        return Err(refuse(dir, Refusal::OutsideRoot(dir.to_path_buf())));
    }
    if path.parent() != Some(dir) || fs::symlink_metadata(path)?.is_dir() {
        return Err(refuse(path, Refusal::OutsideRoot(dir.to_path_buf())));
    }
    fs::remove_file(path)
}

/// [`read_capped`] as UTF-8 text.
pub fn read_to_string_capped(path: &Path, cap: u64, policy: LinkPolicy<'_>) -> io::Result<String> {
    String::from_utf8(read_capped(path, cap, policy)?).map_err(|_| refuse(path, Refusal::NotUtf8))
}

fn too_large(path: &Path, cap: u64) -> io::Error {
    refuse(path, Refusal::TooLarge(cap))
}

/// What a path looks like from outside, to notice a change without reading it: identity
/// (device, inode), type, size and both change times, of the path itself and, for a symlink, of
/// what it points to. Take it *before* reading, so a write racing the read shows up as a change
/// next time instead of being missed.
///
/// `ctime` is in it because tools that restore `mtime` (`cp -p`, `rsync -t`, `touch -d`) can't
/// set it, and the inode because `git checkout` and atomic saves replace the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileStamp {
    path: Stamp,
    /// The link's target, for a symlink.
    target: Option<Stamp>,
}

impl FileStamp {
    /// `None` when nothing is there (or it can't be looked at): appearing later is a change too.
    /// Costs one `lstat`, plus a `stat` for a symlink; never opens the file.
    pub fn of(path: &Path) -> Option<Self> {
        let meta = fs::symlink_metadata(path).ok()?;
        let target = if meta.file_type().is_symlink() {
            fs::metadata(path).ok().as_ref().map(Stamp::of)
        } else {
            None
        };
        Some(Self { path: Stamp::of(&meta), target })
    }

    /// [`of`](Self::of) for the path itself only: one `lstat`, and a symlink is the link, never what
    /// it points to. For a repository's files, where git tracks a link by its text and following it
    /// would look at (and depend on) a file outside the project.
    pub fn of_link(path: &Path) -> Option<Self> {
        let meta = fs::symlink_metadata(path).ok()?;
        Some(Self { path: Stamp::of(&meta), target: None })
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::fs::{mode, permissions, symlink};

    use super::{
        FileStamp, LinkPolicy, Refusal, fs, read_capped, read_to_string_capped, refusal, resolve_within,
        write_atomic, write_atomic_executable,
    };

    fn names(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn tail_reads_are_bounded_and_keep_link_checks() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("log.jsonl");
        fs::write(&path, b"old\nnew\n").unwrap();
        assert_eq!(super::read_tail_capped(&path, 4, LinkPolicy::Within(root.path())).unwrap(), b"new\n");
        assert!(super::read_tail_capped(&path, 0, LinkPolicy::Follow).unwrap().is_empty());
        assert_eq!(super::read_tail_capped(&path, 20, LinkPolicy::Follow).unwrap(), b"old\nnew\n");
        let other = tempfile::tempdir().unwrap();
        assert!(super::read_tail_capped(&path, 20, LinkPolicy::Within(other.path())).is_err());
        assert!(super::read_tail_capped(root.path(), 20, LinkPolicy::Follow).is_err());
    }

    #[test]
    fn replaces_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.json");
        write_atomic(&file, b"one", LinkPolicy::Follow).unwrap();
        write_atomic(&file, b"two", LinkPolicy::Follow).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "two");
        assert_eq!(names(dir.path()), ["a.json"]);
    }

    #[test]
    fn keeps_the_mode_of_the_file_it_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, "{}").unwrap();
        fs::set_permissions(&file, permissions(0o600)).unwrap();
        write_atomic(&file, b"{\"k\":1}", LinkPolicy::Follow).unwrap();
        assert_eq!(mode(&fs::metadata(&file).unwrap()) & 0o777, 0o600);
    }

    #[test]
    fn an_executable_write_can_be_run_whether_the_file_is_new_or_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let bits = |file: &std::path::Path| mode(&fs::metadata(file).unwrap()) & 0o777;
        let file = dir.path().join("tool");
        write_atomic_executable(&file, b"#!/bin/sh\n", LinkPolicy::Follow).unwrap();
        assert_eq!(bits(&file) & 0o100, 0o100, "new: executable by its owner");
        // A plain file of the user's, 0640: it keeps who can read it and gains execute for them only.
        let plain = dir.path().join("plain");
        fs::write(&plain, "old").unwrap();
        fs::set_permissions(&plain, permissions(0o640)).unwrap();
        write_atomic_executable(&plain, b"new", LinkPolicy::Follow).unwrap();
        assert_eq!(bits(&plain), 0o750);
        assert_eq!(fs::read_to_string(&plain).unwrap(), "new");
        // The plain write leaves a new file as it was: not executable.
        let data = dir.path().join("data");
        write_atomic(&data, b"x", LinkPolicy::Follow).unwrap();
        assert_eq!(bits(&data) & 0o111, 0);
    }

    #[test]
    fn follow_writes_through_a_symlinked_file_and_keeps_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("dotfiles").join("config.json");
        fs::create_dir_all(real.parent().unwrap()).unwrap();
        fs::write(&real, "old").unwrap();
        let link = dir.path().join("config.json");
        symlink(&real, &link).unwrap();
        write_atomic(&link, b"new", LinkPolicy::Follow).unwrap();
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(&real).unwrap(), "new");
    }

    #[test]
    fn a_planted_temp_name_is_not_a_way_to_write_elsewhere() {
        // The attack on the old fixed `notes.md.tmp`: a repository ships the link, we write the notes.
        let repo = tempfile::tempdir().unwrap();
        let victim_dir = tempfile::tempdir().unwrap();
        let victim = victim_dir.path().join("authorized_keys");
        fs::write(&victim, "original").unwrap();
        symlink(&victim, repo.path().join("notes.md.tmp")).unwrap();
        symlink(&victim, repo.path().join("notes.md.wormhole-tmp")).unwrap();
        write_atomic(&repo.path().join("notes.md"), b"# notes", LinkPolicy::Within(repo.path())).unwrap();
        assert_eq!(fs::read_to_string(&victim).unwrap(), "original");
        assert_eq!(fs::read_to_string(repo.path().join("notes.md")).unwrap(), "# notes");
    }

    #[test]
    fn within_replaces_a_symlinked_final_component_instead_of_following_it() {
        let repo = tempfile::tempdir().unwrap();
        let victim_dir = tempfile::tempdir().unwrap();
        let victim = victim_dir.path().join("secret");
        fs::write(&victim, "original").unwrap();
        symlink(&victim, repo.path().join("notes.md")).unwrap();
        write_atomic(&repo.path().join("notes.md"), b"# notes", LinkPolicy::Within(repo.path())).unwrap();
        assert_eq!(fs::read_to_string(&victim).unwrap(), "original");
        let meta = fs::symlink_metadata(repo.path().join("notes.md")).unwrap();
        assert!(meta.file_type().is_file());
    }

    #[test]
    fn within_does_not_take_over_the_mode_of_a_linked_outside_file() {
        // Replacing the link must not leak the victim's metadata into the repository either.
        let repo = tempfile::tempdir().unwrap();
        let victim_dir = tempfile::tempdir().unwrap();
        let victim = victim_dir.path().join("secret");
        fs::write(&victim, "original").unwrap();
        fs::set_permissions(&victim, permissions(0o604)).unwrap();
        symlink(&victim, repo.path().join("notes.md")).unwrap();
        write_atomic(&repo.path().join("notes.md"), b"x", LinkPolicy::Within(repo.path())).unwrap();
        write_atomic(&repo.path().join("fresh.md"), b"x", LinkPolicy::Within(repo.path())).unwrap();
        let bits = |name: &str| mode(&fs::metadata(repo.path().join(name)).unwrap()) & 0o777;
        assert_eq!(bits("notes.md"), bits("fresh.md"));
        assert_eq!(mode(&fs::metadata(&victim).unwrap()) & 0o777, 0o604);
    }

    #[test]
    fn within_refuses_a_directory_that_leaves_the_root() {
        let repo = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), repo.path().join(".wormhole")).unwrap();
        let err =
            write_atomic(&repo.path().join(".wormhole/notes.md"), b"x", LinkPolicy::Within(repo.path()))
                .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(names(outside.path()).is_empty());
    }

    #[test]
    fn within_creates_missing_directories_inside_the_root() {
        let repo = tempfile::tempdir().unwrap();
        let file = repo.path().join(".wormhole/deep/notes.md");
        write_atomic(&file, b"x", LinkPolicy::Within(repo.path())).unwrap();
        assert_eq!(fs::read_to_string(file).unwrap(), "x");
    }

    #[test]
    fn reads_are_capped() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("big");
        fs::write(&file, vec![b'a'; 11]).unwrap();
        assert_eq!(read_capped(&file, 11, LinkPolicy::Follow).unwrap().len(), 11);
        let err = read_capped(&file, 10, LinkPolicy::Follow).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(refusal(&err), Some(&Refusal::TooLarge(10)));
        assert!(err.to_string().ends_with("big is larger than 10 bytes"), "{err}");
        assert_eq!(Refusal::TooLarge(1 << 20).to_string(), "is larger than 1 MiB");
        assert_eq!(Refusal::TooLarge(256 << 10).to_string(), "is larger than 256 KiB");
        // A plain I/O error is not a refusal.
        let missing = read_capped(&dir.path().join("missing"), 10, LinkPolicy::Follow).unwrap_err();
        assert_eq!(refusal(&missing), None);
        fs::write(&file, [0xff, 0xfe]).unwrap();
        let err = read_to_string_capped(&file, 10, LinkPolicy::Follow).unwrap_err();
        assert_eq!(refusal(&err), Some(&Refusal::NotUtf8));
    }

    #[test]
    fn an_unbounded_cap_does_not_overflow() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("small");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(read_capped(&file, u64::MAX, LinkPolicy::Follow).unwrap(), b"abc");
    }

    #[test]
    fn within_refuses_a_symlink_to_a_file_outside_the_root() {
        let repo = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let key = outside.path().join("id_ed25519");
        fs::write(&key, "private").unwrap();
        symlink(&key, repo.path().join("notes.md")).unwrap();
        let err = read_to_string_capped(&repo.path().join("notes.md"), 1024, LinkPolicy::Within(repo.path()))
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert_eq!(refusal(&err), Some(&Refusal::OutsideRoot(repo.path().to_path_buf())));
        // A link that stays inside the root is fine.
        fs::write(repo.path().join("real.md"), "ok").unwrap();
        symlink(repo.path().join("real.md"), repo.path().join("alias.md")).unwrap();
        assert_eq!(
            read_to_string_capped(&repo.path().join("alias.md"), 1024, LinkPolicy::Within(repo.path()))
                .unwrap(),
            "ok"
        );
    }

    #[test]
    fn resolve_within_gives_the_real_path_of_a_file_inside_the_root_only() {
        let repo = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(repo.path().join("src")).unwrap();
        fs::write(repo.path().join("src/a.rs"), "x").unwrap();
        fs::write(outside.path().join("secret"), "x").unwrap();
        let real = fs::canonicalize(repo.path()).unwrap();
        assert_eq!(resolve_within(repo.path(), "src/a.rs").unwrap(), real.join("src/a.rs"));

        let refused = |relative: &str| resolve_within(repo.path(), relative).unwrap_err();
        // Not a plain relative path: nothing is even looked at.
        for bad in ["", "/etc/hosts", "../x", "src/../../x", "src/\0a.rs", "./../x"] {
            assert_eq!(
                refusal(&refused(bad)),
                Some(&Refusal::OutsideRoot(repo.path().to_path_buf())),
                "{bad:?}"
            );
        }
        // A link out of the root, to a file or a folder, however it is reached.
        symlink(outside.path().join("secret"), repo.path().join("link")).unwrap();
        symlink(outside.path(), repo.path().join("dir-link")).unwrap();
        assert_eq!(refusal(&refused("link")), Some(&Refusal::OutsideRoot(repo.path().to_path_buf())));
        assert_eq!(
            refusal(&refused("dir-link/secret")),
            Some(&Refusal::OutsideRoot(repo.path().to_path_buf()))
        );
        // A link that stays inside is followed to the file it names.
        symlink(repo.path().join("src/a.rs"), repo.path().join("alias.rs")).unwrap();
        assert_eq!(resolve_within(repo.path(), "alias.rs").unwrap(), real.join("src/a.rs"));
        // Gone, and not a file.
        assert_eq!(refused("src/missing.rs").kind(), std::io::ErrorKind::NotFound);
        assert_eq!(refusal(&refused("src")), Some(&Refusal::NotRegularFile));
    }

    #[test]
    fn a_fifo_is_refused_instead_of_blocking() {
        let dir = tempfile::tempdir().unwrap();
        let fifo = dir.path().join("pipe");
        let made = std::process::Command::new("mkfifo").arg(&fifo).status();
        if !made.is_ok_and(|s| s.success()) {
            return; // no mkfifo on this machine
        }
        let err = read_capped(&fifo, 1024, LinkPolicy::Follow).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert_eq!(refusal(&err), Some(&Refusal::NotRegularFile));
    }

    #[test]
    fn stamps_change_with_content_identity_and_links() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("actions.json");
        assert_eq!(FileStamp::of(&file), None);
        fs::write(&file, "[]").unwrap();
        let first = FileStamp::of(&file).unwrap();
        assert_eq!(FileStamp::of(&file), Some(first), "stable while nothing changes");

        fs::write(&file, "[1]").unwrap();
        let edited = FileStamp::of(&file).unwrap();
        assert_ne!(edited, first, "an edit in place");
        // Same size, contents swapped by an atomic save (new inode), mtime put back.
        let mtime = fs::metadata(&file).unwrap().modified().unwrap();
        write_atomic(&file, b"[2]", LinkPolicy::Follow).unwrap();
        fs::File::options().write(true).open(&file).unwrap().set_modified(mtime).unwrap();
        assert_ne!(FileStamp::of(&file).unwrap(), edited, "a replaced file with its old mtime");

        // A link: retargeting it, or editing what it points to, is a change.
        let (a, b) = (dir.path().join("a.md"), dir.path().join("b.md"));
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();
        let link = dir.path().join("link.md");
        symlink(&a, &link).unwrap();
        let linked = FileStamp::of(&link).unwrap();
        fs::write(&a, "aa").unwrap();
        let target_edited = FileStamp::of(&link).unwrap();
        assert_ne!(target_edited, linked);
        fs::remove_file(&link).unwrap();
        symlink(&b, &link).unwrap();
        assert_ne!(FileStamp::of(&link).unwrap(), target_edited);

        fs::remove_file(&file).unwrap();
        assert_eq!(FileStamp::of(&file), None);
    }

    #[test]
    fn a_link_stamp_without_its_target_ignores_what_the_link_points_to() {
        let dir = tempfile::tempdir().unwrap();
        let (target, link) = (dir.path().join("target.md"), dir.path().join("link.md"));
        fs::write(&target, "a").unwrap();
        symlink(&target, &link).unwrap();
        let before = FileStamp::of_link(&link).unwrap();
        fs::write(&target, "longer").unwrap();
        assert_eq!(FileStamp::of_link(&link), Some(before), "the target is not looked at");
        // A dangling link is still a link.
        fs::remove_file(&target).unwrap();
        assert_eq!(FileStamp::of_link(&link), Some(before));
        fs::remove_file(&link).unwrap();
        assert_eq!(FileStamp::of_link(&link), None);
    }
}
