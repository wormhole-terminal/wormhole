//! Wormhole's own themes. Unlike the imported ones (Ghostty palettes, see [`crate::theme`]) they
//! are theme files: each may set its own primary and other tokens, and some ship more than one
//! version (a light and a dark one). They are read here, unresolved, so `extends` can reach them
//! while the built-in tokens are being derived.

use std::sync::LazyLock;

use super::file::{ThemeFile, parse_theme_file};
use super::variants::variants;

/// The files, in the order the picker lists them (after the imported themes).
const SOURCES: &[(&str, &str)] = &[
    ("Wormhole", include_str!("../themes/own/Wormhole.json")),
    ("Nebula", include_str!("../themes/own/Nebula.json")),
    ("Solar Flare", include_str!("../themes/own/Solar Flare.json")),
    ("Borealis", include_str!("../themes/own/Borealis.json")),
    ("Event Horizon", include_str!("../themes/own/Event Horizon.json")),
    ("Void", include_str!("../themes/own/Void.json")),
    ("Crimson", include_str!("../themes/own/Crimson.json")),
    ("Cyberpunk", include_str!("../themes/own/Cyberpunk.json")),
    ("Matrix", include_str!("../themes/own/Matrix.json")),
    ("Soltari", include_str!("../themes/own/Soltari.json")),
];

/// One of Wormhole's own themes: a file's only theme, or one of its versions.
pub(super) struct Own {
    pub name: String,
    /// The family and the version's own name, for a file that ships several.
    pub family: Option<(String, String)>,
    pub file: ThemeFile,
}

/// Every own theme, each version on its own, in [`SOURCES`] order.
pub(super) fn own() -> &'static [Own] {
    static OWN: LazyLock<Vec<Own>> = LazyLock::new(|| {
        let mut all = Vec::new();
        for (name, src) in SOURCES {
            // These files are part of the app: a mistake in one is a bug the tests catch.
            let file = parse_theme_file(src).unwrap_or_else(|e| panic!("own theme {name}: {e}"));
            let versions = variants(&file, name).unwrap_or_else(|e| panic!("own theme {name}: {e}"));
            if versions.is_empty() {
                all.push(Own { name: (*name).to_string(), family: None, file });
            }
            for v in versions {
                all.push(Own { name: v.name, family: Some(((*name).to_string(), v.short)), file: v.file });
            }
        }
        all
    });
    &OWN
}

/// The own theme a name means: its own name exactly, else in any case, else a family's
/// name, which means the family's first version.
pub(super) fn own_named(name: &str) -> Option<&'static Own> {
    let all = own();
    all.iter()
        .find(|o| o.name == name)
        .or_else(|| all.iter().find(|o| o.name.eq_ignore_ascii_case(name)))
        .or_else(|| all.iter().find(|o| o.family.as_ref().is_some_and(|(f, _)| f.eq_ignore_ascii_case(name))))
}

#[cfg(test)]
mod tests {
    use super::{SOURCES, own, own_named};
    use crate::theme;

    #[test]
    fn every_name_is_its_own() {
        let mut names: Vec<String> = own().iter().map(|o| o.name.to_lowercase()).collect();
        names.extend(theme::builtins().iter().map(|t| t.name.to_lowercase()));
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "two built-in themes share a name");
        for (name, src) in SOURCES {
            assert!(src.contains(&format!("\"name\": \"{name}\"")), "{name}: the file names itself");
        }
    }

    #[test]
    fn a_family_name_means_its_first_version() {
        assert_eq!(own_named("Soltari").map(|o| o.name.as_str()), Some("Soltari Light"));
        assert_eq!(own_named("crimson").map(|o| o.name.as_str()), Some("Crimson Dark"));
        assert_eq!(own_named("matrix").map(|o| o.name.as_str()), Some("Matrix"));
        assert!(own_named("Nord").is_none(), "an imported theme is not one of these");
    }
}
