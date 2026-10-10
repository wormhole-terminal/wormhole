//! Unpacking a verified update archive without trusting it.
//!
//! The archive is signature-checked before this runs, so what follows is defense in depth: a
//! bug in the signing pipeline, or a stolen key, must not become a write outside the staging
//! directory.

use std::fs;
use std::io::{BufReader, Seek as _, SeekFrom};
use std::path::{Component, Path, PathBuf};

use super::UpdateError;

/// Generous for an app bundle; guards the disk against a decompression bomb.
const MAX_UNPACKED_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 200_000;

/// Unpacks the verified archive into `dest` and returns the single top-level `.app` in it.
///
/// The archive is already signature-checked, so this is defense in depth: it refuses anything
/// that could write outside `dest` (absolute paths, `..`, escaping symlinks, links created below
/// other links, hard links, device nodes) and anything that isn't one `*.app` tree. On an error
/// `dest` holds a partial tree: the caller throws it away (the staging directory).
pub(crate) fn extract(archive: &fs::File, dest: &Path) -> Result<PathBuf, UpdateError> {
    let mut file = archive;
    file.seek(SeekFrom::Start(0))?;
    fs::create_dir_all(dest)?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(BufReader::new(file)));
    tar.set_overwrite(false);
    // Plain rwx bits only: setuid/setgid from an archive have no business in an app bundle.
    tar.set_preserve_permissions(false);

    let bad = |msg: String| UpdateError::Install(format!("archive rejected: {msg}"));
    let mut top: Option<PathBuf> = None;
    let (mut total, mut count) = (0u64, 0usize);
    for entry in tar.entries().map_err(|e| bad(e.to_string()))? {
        let mut entry = entry.map_err(|e| bad(e.to_string()))?;
        // Some tar writers emit a global pax header first; it carries metadata, not a file.
        if entry.header().entry_type() == tar::EntryType::XGlobalHeader {
            continue;
        }
        let raw = entry.path().map_err(|e| bad(e.to_string()))?.into_owned();
        let rel = sanitize_entry_path(&raw).ok_or_else(|| bad(format!("unsafe path {}", raw.display())))?;
        let Some(first) = rel.components().next() else { continue }; // the "./" entry
        let first = PathBuf::from(first.as_os_str());
        if first.extension().is_none_or(|e| e != "app") {
            return Err(bad(format!("{} is outside an .app bundle", raw.display())));
        }
        match &top {
            Some(t) if *t != first => return Err(bad("more than one top-level item".into())),
            Some(_) => {}
            None => top = Some(first),
        }
        // AppleDouble files appear when an archive is made with macOS tar without
        // COPYFILE_DISABLE=1; they'd land inside the bundle and break its code seal.
        if rel.file_name().is_some_and(|n| n.as_encoded_bytes().starts_with(b"._")) {
            tracing::debug!(path = %rel.display(), "skipping AppleDouble entry");
            continue;
        }

        let kind = entry.header().entry_type();
        if kind.is_symlink() {
            let target = entry.link_name().map_err(|e| bad(e.to_string()))?.unwrap_or_default();
            if let Some(through) = symlink_in_parents(dest, &rel) {
                return Err(bad(format!(
                    "symlink {} would be created through another symlink ({})",
                    rel.display(),
                    through.display()
                )));
            }
            if !symlink_stays_inside(&rel, &target) {
                return Err(bad(format!(
                    "symlink {} -> {} escapes the bundle",
                    rel.display(),
                    target.display()
                )));
            }
        } else if !(kind.is_file() || kind.is_dir()) {
            return Err(bad(format!("unsupported entry type {kind:?} at {}", rel.display())));
        }

        count += 1;
        total = total.saturating_add(entry.header().size().unwrap_or(0));
        if count > MAX_ENTRIES || total > MAX_UNPACKED_BYTES {
            return Err(bad("unpacked size exceeds the limit".into()));
        }
        // unpack_in re-checks containment (including through symlinks already on disk) and
        // returns false if it refused.
        if !entry.unpack_in(dest).map_err(|e| bad(format!("{}: {e}", rel.display())))? {
            return Err(bad(format!("unsafe path {}", raw.display())));
        }
    }
    let top = top.ok_or_else(|| bad("archive is empty".into()))?;
    Ok(dest.join(top))
}

/// Drops `.` components; `None` for anything absolute or containing `..`.
fn sanitize_entry_path(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Normal(p) => out.push(p),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

/// The first symlink among the directories above `rel` (as already extracted under `dest`).
///
/// [`symlink_stays_inside`] reads a link's place off its path, which is only true while no
/// directory above it is itself a link: with `up -> ..` extracted, `up/esc` is created one level
/// higher than it looks, and a target the lexical check calls "inside" leaves the bundle. Real
/// archives list a link and never what is below it, so such an entry is refused outright rather
/// than resolved. Directories not extracted yet are the real ones `unpack_in` will create.
fn symlink_in_parents(dest: &Path, rel: &Path) -> Option<PathBuf> {
    let mut at = dest.to_path_buf();
    let mut seen = PathBuf::new();
    for component in rel.parent()?.components() {
        at.push(component);
        seen.push(component);
        match fs::symlink_metadata(&at) {
            Ok(meta) if meta.file_type().is_symlink() => return Some(seen),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    None
}

/// Lexically resolves `target` relative to the link's directory and requires the result to stay
/// within the top-level bundle directory (framework `Versions/Current -> A` links are fine).
fn symlink_stays_inside(link: &Path, target: &Path) -> bool {
    if target.as_os_str().is_empty() || target.is_absolute() {
        return false;
    }
    let mut depth = link.parent().map_or(0, |p| p.components().count());
    for c in target.components() {
        match c {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            // Depth 1 is the .app dir itself; going above it leaves the bundle.
            Component::ParentDir if depth > 1 => depth -= 1,
            _ => return false,
        }
    }
    depth >= 1
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{sanitize_entry_path, symlink_stays_inside};

    #[test]
    fn entry_paths() {
        assert_eq!(sanitize_entry_path(Path::new("./A.app/x")), Some(PathBuf::from("A.app/x")));
        assert_eq!(sanitize_entry_path(Path::new("A.app/../../etc")), None);
        assert_eq!(sanitize_entry_path(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn symlink_containment() {
        let link = Path::new("A.app/Contents/Frameworks/X.framework/Versions/Current");
        assert!(symlink_stays_inside(link, Path::new("A")));
        assert!(symlink_stays_inside(Path::new("A.app/Contents/x"), Path::new("../Resources/y")));
        assert!(!symlink_stays_inside(Path::new("A.app/Contents/x"), Path::new("../../y")));
        assert!(!symlink_stays_inside(Path::new("A.app/x"), Path::new("/etc/passwd")));
        assert!(!symlink_stays_inside(Path::new("A.app/x"), Path::new("..")));
        assert!(!symlink_stays_inside(Path::new("A.app/x"), Path::new("")));
    }
}
