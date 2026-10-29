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
    let mut file = tokens.to_theme_file(None);
    // A palette that sets neither has them follow its background and foreground. An older release
    // has nothing under a standalone file but Tokyo Night, whose own would show instead, so the
    // worked-out colors are written.
    for (node, color) in tokens.terminal.named() {
        let key = &node["terminal.".len()..];
        let missing = match key {
            "cursor_text" => file.terminal.cursor_text.is_none(),
            "selection_foreground" => file.terminal.selection_foreground.is_none(),
            _ => false,
        };
        if missing {
            file.terminal.insert(key, color.to_hex());
        }
    }
    file.to_json_pretty()
}

#[cfg(test)]
mod tests {
    use super::write_standalone_v0;
    use crate::theme::tokens::test_support::{builtin, no_lookup, snapshot};
    use crate::theme::tokens::{parse_theme_file, resolve};

    #[test]
    fn cursor_text_and_selection_foreground_are_written_when_the_palette_has_none() {
        use crate::theme::tokens::ThemeTokens;
        // A palette theme: the cursor text and the selection foreground follow the colors.
        let mut tokens: ThemeTokens = builtin("Nord");
        tokens.terminal.cursor_text = None;
        tokens.terminal.selection_foreground = None;
        let text = write_standalone_v0(&tokens);
        let file = parse_theme_file(&text).unwrap();
        assert_eq!(file.terminal.cursor_text.as_deref(), Some(tokens.terminal.background.to_hex().as_str()));
        assert_eq!(
            file.terminal.selection_foreground.as_deref(),
            Some(tokens.terminal.foreground.to_hex().as_str())
        );
        // As an older release reads it (a version 0 file with nothing under it): the same colors.
        let back = resolve(&file, &no_lookup).unwrap();
        assert_eq!(back.terminal.named(), tokens.terminal.named());
    }

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
