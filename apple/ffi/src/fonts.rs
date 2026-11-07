//! The font pickers' rules for Swift: which families a picker lists and in what order (core's
//! `grouped_font_choices`), the user's font files (core's scan of the fonts folder) and the bundled
//! family's name. The installed families and their traits come from CoreText on the Swift side, which is
//! where GPUI gets them too; everything that is a rule stays core's.

use std::collections::HashSet;
use std::path::Path;

use wormhole_core::theme::fonts::custom::{self, Scan};
use wormhole_core::theme::fonts::{FontSource, bundled, grouped_font_choices};

/// Where a family in a picker comes from, in the order pickers list them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FontOrigin {
    Bundled,
    Custom,
    Installed,
}

impl From<FontSource> for FontOrigin {
    fn from(source: FontSource) -> Self {
        match source {
            FontSource::Bundled => Self::Bundled,
            FontSource::Custom => Self::Custom,
            FontSource::Installed => Self::Installed,
        }
    }
}

/// One family a picker lists.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FontChoiceRecord {
    pub origin: FontOrigin,
    pub name: String,
}

/// A family from the user's font files, and whether all its faces are fixed-pitch.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CustomFontFamilyRecord {
    pub name: String,
    pub monospace: bool,
}

/// A font file that parsed, with the families its faces belong to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CustomFontFileRecord {
    pub path: String,
    pub families: Vec<CustomFontFamilyRecord>,
}

/// What the fonts folder holds: the usable files, every family once (sorted), and the files that are not fonts.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct CustomFontsRecord {
    pub files: Vec<CustomFontFileRecord>,
    pub families: Vec<CustomFontFamilyRecord>,
    pub skipped: Vec<String>,
}

/// The families a picker shows for `query`: the bundled, custom and installed ones grouped in that order,
/// each group in search order (`theme::fonts::grouped_font_choices`). `monospace` is the set of fixed-pitch
/// families, nil while they are still being found (everything is listed then); it must include the bundled and
/// custom ones.
#[uniffi::export]
pub fn font_choices(
    installed: Vec<String>,
    bundled: Vec<String>,
    custom: Vec<String>,
    monospace: Option<Vec<String>>,
    query: String,
    monospace_only: bool,
) -> Vec<FontChoiceRecord> {
    let monospace: Option<HashSet<String>> = monospace.map(|names| names.into_iter().collect());
    grouped_font_choices(&installed, &bundled, &custom, monospace.as_ref(), &query, monospace_only)
        .into_iter()
        .map(|(source, name)| FontChoiceRecord { origin: source.into(), name })
        .collect()
}

/// The font that ships inside the app (JetBrains Mono).
#[uniffi::export]
pub fn font_bundled_family() -> String {
    bundled::FAMILY.to_string()
}

/// Reads the fonts folder `dir` (core's capped scan, a few levels deep, `.ttf .otf .ttc .otc`). A file that is
/// not a font is named in `skipped` and never hides the others. A folder that is missing has no fonts.
#[uniffi::export]
pub fn font_custom_scan(dir: String) -> CustomFontsRecord {
    records(custom::scan(Path::new(&dir)))
}

fn records(scan: Scan) -> CustomFontsRecord {
    let family =
        |f: &custom::FontFamily| CustomFontFamilyRecord { name: f.name.clone(), monospace: f.monospace };
    CustomFontsRecord {
        families: custom::families(&scan.files).iter().map(family).collect(),
        files: scan
            .files
            .iter()
            .map(|file| CustomFontFileRecord {
                path: file.path.to_string_lossy().into_owned(),
                families: file.families.iter().map(family).collect(),
            })
            .collect(),
        skipped: scan.skipped.iter().map(|(path, _)| path.to_string_lossy().into_owned()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::files::{self, LinkPolicy};

    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn choices_are_cores_grouped_and_ranked() {
        let found = font_choices(
            names(&["Menlo", "JetBrains Mono", "Arial", "Fira Code"]),
            names(&["JetBrains Mono"]),
            names(&["Fira Code"]),
            Some(names(&["JetBrains Mono", "Fira Code", "Menlo"])),
            "m".into(),
            true,
        );
        let listed: Vec<_> = found.iter().map(|c| (c.origin, c.name.as_str())).collect();
        assert_eq!(listed, [(FontOrigin::Bundled, "JetBrains Mono"), (FontOrigin::Installed, "Menlo")]);
    }

    #[test]
    fn nothing_is_filtered_while_the_monospace_set_is_unknown() {
        let found = font_choices(names(&["Arial", "Menlo"]), vec![], vec![], None, String::new(), true);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn a_missing_folder_has_no_fonts_and_a_bad_file_is_skipped() {
        assert!(font_custom_scan("/nonexistent/Wormhole-swift-fonts".into()).files.is_empty());
        let dir = tempfile::tempdir().expect("temp dir");
        files::write_atomic(&dir.path().join("Broken.ttf"), b"not a font", LinkPolicy::Follow)
            .expect("write");
        let scan = font_custom_scan(dir.path().to_string_lossy().into_owned());
        assert!(scan.files.is_empty() && scan.families.is_empty());
        assert_eq!(scan.skipped.len(), 1);
    }

    #[test]
    fn the_bundled_family_is_jetbrains_mono() {
        assert_eq!(font_bundled_family(), "JetBrains Mono");
    }
}
