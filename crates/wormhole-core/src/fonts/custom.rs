//! The user's own fonts: font files dropped into `<data dir>/fonts/` (folders
//! inside it too, e.g. an unzipped release). This finds and reads them; the
//! app registers them with the system for this process only, so they need no
//! installing and go away with the files.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, bail};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::{FileRef, ReadError, TableProvider};
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

use crate::profile::data_dir;

/// TrueType, OpenType (TrueType or CFF outlines) and collections of either.
pub const EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

/// How deep folders inside the fonts folder are searched. An unzipped font
/// release often nests its files (`Family-1.0/fonts/ttf/…`); the cap keeps a
/// symlink loop from running away.
const MAX_DEPTH: usize = 4;

/// Where the user puts their fonts.
pub fn user_dir() -> PathBuf {
    data_dir().join("fonts")
}

/// A family as a font file names it, and whether its faces are fixed-pitch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamily {
    pub name: String,
    pub monospace: bool,
}

/// A font file that parsed, with the families its faces belong to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFile {
    pub path: PathBuf,
    pub families: Vec<FontFamily>,
}

/// The fonts folder's usable files, and the ones that aren't (path, reason).
#[derive(Debug, Default)]
pub struct Scan {
    pub files: Vec<FontFile>,
    pub skipped: Vec<(PathBuf, String)>,
}

/// Every family in `files`, once, sorted; monospace if all its faces are.
pub fn families(files: &[FontFile]) -> Vec<FontFamily> {
    let mut families: Vec<FontFamily> = Vec::new();
    for family in files.iter().flat_map(|file| &file.families) {
        match families.iter_mut().find(|f| f.name == family.name) {
            Some(known) => known.monospace &= family.monospace,
            None => families.push(family.clone()),
        }
    }
    families.sort_by_key(|f| f.name.to_lowercase());
    families
}

/// Whether `path` looks like a font file we can load (by extension; hidden
/// files, like macOS's `._` resource forks, are skipped).
pub fn is_font_file(path: &Path) -> bool {
    let hidden = path.file_name().is_none_or(|n| n.to_string_lossy().starts_with('.'));
    let known =
        path.extension().is_some_and(|ext| EXTENSIONS.iter().any(|known| ext.eq_ignore_ascii_case(known)));
    !hidden && known
}

/// Most folder entries one scan looks at. The folder is polled every couple of
/// seconds on the UI thread, so a link to some huge folder must not be walked
/// in full each time.
const MAX_ENTRIES: usize = 10_000;

/// The font files under `dir`, sorted. A missing folder has none.
///
/// Linked folders are followed (a shared font collection), except a link to
/// a folder that contains the fonts folder itself, such as the home folder,
/// which would walk everything; and each real folder is listed once.
pub fn font_files(dir: &Path) -> Vec<PathBuf> {
    font_files_within(dir, MAX_ENTRIES)
}

fn font_files_within(dir: &Path, max_entries: usize) -> Vec<PathBuf> {
    let mut walk =
        Walk { root: dir.canonicalize().ok(), seen: HashSet::new(), left: max_entries, files: Vec::new() };
    walk.collect(dir, 0);
    walk.files.sort();
    walk.files
}

struct Walk {
    root: Option<PathBuf>,
    /// Real folders already listed.
    seen: HashSet<PathBuf>,
    /// Entries this scan may still look at.
    left: usize,
    files: Vec<PathBuf>,
}

impl Walk {
    fn collect(&mut self, dir: &Path, depth: usize) {
        if !dir.canonicalize().is_ok_and(|real| self.seen.insert(real)) {
            return;
        }
        // In name order, so which of two links to one folder is listed doesn't vary.
        let mut entries: Vec<_> =
            std::fs::read_dir(dir).into_iter().flatten().flatten().take(self.left).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if self.left == 0 {
                return;
            }
            self.left -= 1;
            let path = entry.path();
            let hidden = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
            let Ok(kind) = entry.file_type() else { continue };
            let is_dir = if kind.is_symlink() {
                let target =
                    std::fs::metadata(&path).ok().filter(|m| m.is_dir()).and(path.canonicalize().ok());
                match target {
                    Some(target) if self.root.as_ref().is_some_and(|root| root.starts_with(&target)) => {
                        continue;
                    }
                    Some(_) => true,
                    None => false,
                }
            } else {
                kind.is_dir()
            };
            if is_dir {
                if !hidden && depth < MAX_DEPTH {
                    self.collect(&path, depth + 1);
                }
            } else if path.is_file() && is_font_file(&path) {
                self.files.push(path);
            }
        }
    }
}

/// Read every font file under `dir`. Files that aren't fonts (or are broken)
/// end up in `skipped`, so one bad file never hides the others.
pub fn scan(dir: &Path) -> Scan {
    let mut scan = Scan::default();
    for path in font_files(dir) {
        match read_font_file(&path) {
            Ok(file) => scan.files.push(file),
            Err(err) => scan.skipped.push((path, format!("{err:#}"))),
        }
    }
    scan
}

pub fn read_font_file(path: &Path) -> anyhow::Result<FontFile> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let families = parse_font(&bytes)?;
    Ok(FontFile { path: path.to_path_buf(), families })
}

/// The families in a font file (several for a collection), in file order.
pub fn parse_font(bytes: &[u8]) -> anyhow::Result<Vec<FontFamily>> {
    let file = FileRef::new(bytes).map_err(|err| anyhow::anyhow!("not a usable font: {err}"))?;
    let mut families: Vec<FontFamily> = Vec::new();
    let mut last_error: Option<ReadError> = None;
    for font in file.fonts() {
        let font = match font {
            Ok(font) => font,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };
        let Some(name) = family_name(&font) else { continue };
        let monospace = is_monospace(&font);
        match families.iter_mut().find(|f| f.name == name) {
            Some(known) => known.monospace &= monospace,
            None => families.push(FontFamily { name, monospace }),
        }
    }
    if families.is_empty() {
        match last_error {
            Some(err) => bail!("not a usable font: {err}"),
            None => bail!("the font has no family name"),
        }
    }
    Ok(families)
}

/// The family name the system groups the face under: the typographic family
/// (name ID 16) when there is one, as CoreText does – "Fira Code", not the
/// legacy "Fira Code Medium" – else the family (ID 1). English wins.
fn family_name(font: &FontRef) -> Option<String> {
    let lookup = |id: StringId| {
        font.localized_strings(id)
            .english_or_first()
            .map(|name| name.to_string().trim().to_string())
            .filter(|name| !name.is_empty())
    };
    lookup(StringId::TYPOGRAPHIC_FAMILY_NAME).or_else(|| lookup(StringId::FAMILY_NAME))
}

/// The fixed-pitch flag, or equal advances for `iMW0` (some coding fonts
/// don't set the flag) – the same test the app runs on installed fonts.
fn is_monospace(font: &FontRef) -> bool {
    font.post().is_ok_and(|post| post.is_fixed_pitch() != 0) || has_equal_advances(font)
}

fn has_equal_advances(font: &FontRef) -> bool {
    let charmap = font.charmap();
    let metrics = font.glyph_metrics(Size::unscaled(), LocationRef::default());
    let advances: Option<Vec<f32>> =
        "iMW0".chars().map(|c| charmap.map(c).and_then(|glyph| metrics.advance_width(glyph))).collect();
    advances.is_some_and(|a| a[0] > 0.0 && a.iter().all(|&w| w == a[0]))
}

/// Cheap change detector for the fonts folder: paths, sizes and mtimes.
pub type Fingerprint = Vec<(PathBuf, u64, Option<SystemTime>)>;

pub fn fingerprint(dir: &Path) -> Fingerprint {
    font_files(dir)
        .into_iter()
        .map(|path| {
            let meta = std::fs::metadata(&path).ok();
            let len = meta.as_ref().map_or(0, |m| m.len());
            (path, len, meta.and_then(|m| m.modified().ok()))
        })
        .collect()
}

/// What changed between two fingerprints: the files to drop (gone or
/// changed) and the ones to load (new or changed).
pub fn changes(old: &Fingerprint, new: &Fingerprint) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let removed = old.iter().filter(|entry| !new.contains(entry)).map(|(path, ..)| path.clone()).collect();
    let added = new.iter().filter(|entry| !old.contains(entry)).map(|(path, ..)| path.clone()).collect();
    (removed, added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::bundled;

    /// A fresh, empty folder under the temp dir for one test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hd-fonts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    /// A TrueType collection of `fonts`: the TTC header, then each font with
    /// its table offsets moved to count from the start of the collection.
    fn collection(fonts: &[&[u8]]) -> Vec<u8> {
        let header = 12 + 4 * fonts.len();
        let mut out = b"ttcf".to_vec();
        out.extend(0x0001_0000u32.to_be_bytes());
        out.extend((fonts.len() as u32).to_be_bytes());
        let mut offset = header;
        for font in fonts {
            out.extend((offset as u32).to_be_bytes());
            offset += font.len();
        }
        let mut start = header;
        for font in fonts {
            let mut font = font.to_vec();
            let tables = u16::from_be_bytes([font[4], font[5]]) as usize;
            for table in 0..tables {
                let at = 12 + 16 * table + 8;
                let old = u32::from_be_bytes([font[at], font[at + 1], font[at + 2], font[at + 3]]);
                font[at..at + 4].copy_from_slice(&(old + start as u32).to_be_bytes());
            }
            start += font.len();
            out.extend(font);
        }
        out
    }

    #[test]
    fn only_font_files_are_picked_up_recursively() {
        let dir = temp_dir("list");
        let nested = dir.join("Family-1.0").join("fonts").join("ttf");
        std::fs::create_dir_all(&nested).expect("nested");
        for name in ["A.ttf", "B.OTF", "C.ttc", "notes.txt", ".hidden.ttf", "._A.ttf", "font.woff2"] {
            std::fs::write(dir.join(name), b"x").expect("write");
        }
        std::fs::write(nested.join("Deep.otf"), b"x").expect("write");
        let names: Vec<String> = font_files(&dir)
            .iter()
            .map(|p| p.strip_prefix(&dir).expect("under dir").to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["A.ttf", "B.OTF", "C.ttc", "Family-1.0/fonts/ttf/Deep.otf"]);
        assert!(font_files(&dir.join("missing")).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn linked_folders_are_followed_once_but_never_back_up() {
        use std::os::unix::fs::symlink;
        let home = temp_dir("links");
        let dir = home.join("data").join("fonts");
        let shared = home.join("Shared Fonts");
        std::fs::create_dir_all(&dir).expect("fonts dir");
        std::fs::create_dir_all(&shared).expect("shared dir");
        std::fs::write(shared.join("Shared.ttf"), b"x").expect("write");
        std::fs::write(home.join("Stray.ttf"), b"x").expect("write");
        std::fs::write(dir.join("Own.ttf"), b"x").expect("write");
        symlink(&shared, dir.join("shared")).expect("link");
        symlink(&shared, dir.join("shared again")).expect("link");
        // A link to a folder holding the fonts folder (like home) would walk all of it.
        symlink(&home, dir.join("home")).expect("link");
        symlink(&dir, dir.join("loop")).expect("link");
        let names: Vec<String> = font_files(&dir)
            .iter()
            .map(|p| p.strip_prefix(&dir).expect("under dir").to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["Own.ttf", "shared/Shared.ttf"]);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_scan_looks_at_a_bounded_number_of_entries() {
        let dir = temp_dir("bounded");
        for i in 0..10 {
            std::fs::write(dir.join(format!("F{i}.ttf")), b"x").expect("write");
        }
        assert_eq!(font_files_within(&dir, 4).len(), 4);
        assert_eq!(font_files(&dir).len(), 10);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broken_files_are_skipped_not_fatal() {
        let dir = temp_dir("scan");
        std::fs::write(dir.join("Good.ttf"), bundled::FACES[0]).expect("write");
        std::fs::write(dir.join("Broken.ttf"), b"definitely not a font").expect("write");
        std::fs::write(dir.join("Truncated.otf"), &bundled::FACES[1][..2000]).expect("write");
        let scan = scan(&dir);
        assert_eq!(scan.files.len(), 1);
        assert_eq!(scan.files[0].families, [FontFamily { name: bundled::FAMILY.into(), monospace: true }]);
        let skipped: Vec<_> = scan.skipped.iter().filter_map(|(p, _)| p.file_name()).collect();
        assert_eq!(skipped, ["Broken.ttf", "Truncated.otf"]);
        std::fs::write(dir.join("Copy.ttf"), bundled::FACES[2]).expect("write");
        assert_eq!(families(&super::scan(&dir).files).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn equal_advances_detect_monospace_without_the_flag() {
        let font = FontRef::new(bundled::FACES[0]).expect("bundled face parses");
        assert!(has_equal_advances(&font));
    }

    #[test]
    fn collections_list_their_families_once() {
        let ttc = collection(&[bundled::FACES[0], bundled::FACES[1]]);
        assert!(matches!(FileRef::new(&ttc), Ok(FileRef::Collection(c)) if c.len() == 2));
        let families = parse_font(&ttc).expect("collection parses");
        assert_eq!(families, [FontFamily { name: bundled::FAMILY.into(), monospace: true }]);
    }

    #[test]
    fn fingerprint_changes_name_files_to_reload() {
        let dir = temp_dir("changes");
        std::fs::write(dir.join("Keep.ttf"), b"one").expect("write");
        std::fs::write(dir.join("Gone.ttf"), b"one").expect("write");
        let before = fingerprint(&dir);
        std::fs::remove_file(dir.join("Gone.ttf")).expect("remove");
        std::fs::write(dir.join("New.otf"), b"two").expect("write");
        let after = fingerprint(&dir);
        let (removed, added) = changes(&before, &after);
        assert_eq!(removed, [dir.join("Gone.ttf")]);
        assert_eq!(added, [dir.join("New.otf")]);
        assert_eq!(changes(&after, &after), (vec![], vec![]));

        // A replaced file (different size) is dropped and loaded again.
        let replaced = vec![(dir.join("Keep.ttf"), 99, None)];
        let (removed, added) = changes(&after[..1].to_vec(), &replaced);
        assert_eq!((removed, added), (vec![dir.join("Keep.ttf")], vec![dir.join("Keep.ttf")]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
