//! The old format, for where older releases must still read the file.
//!
//! Releases before version 1 reject any key they do not know, so a version 1 file fails to
//! load in them. A project's `.wormhole/theme.json`, Export and Copy JSON therefore keep
//! writing the old format until those releases are gone: standalone (no `extends`, so no
//! theme to be found by name on another machine) and flat (every value a color, the rules
//! of this release having been applied).

use crate::theme::tokens::ThemeTokens;

/// `tokens` as a standalone version 0 file's text: every terminal color, every UI and syntax
/// color, every metric, no `extends`.
pub fn write_standalone_v0(tokens: &ThemeTokens) -> String {
    tokens.to_theme_file(None).to_json_pretty()
}

#[cfg(test)]
mod tests {
    use super::write_standalone_v0;
    use crate::theme::tokens::test_support::{builtin, no_lookup, snapshot};
    use crate::theme::tokens::{parse_theme_file, resolve};

    #[test]
    fn the_file_is_old_format_standalone_and_resolves_to_the_same_theme() {
        for name in ["Nord", "Wormhole Light", "GitHub Light"] {
            let tokens = builtin(name);
            let text = write_standalone_v0(&tokens);
            for key in ["\"version\"", "\"extends\"", "\"base\"", "\"id\""] {
                assert!(!text.contains(key), "{name}: {key}");
            }
            // The keys older releases know: the underscore spelling.
            assert!(text.contains("\"selection_background\"") && !text.contains("\"cursor.text\""));
            let back = resolve(&parse_theme_file(&text).unwrap(), &no_lookup).unwrap();
            assert_eq!(snapshot(&back), snapshot(&tokens), "{name}");
        }
    }
}
