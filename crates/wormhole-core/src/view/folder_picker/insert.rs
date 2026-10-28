//! The text a picked folder puts at the prompt: its path in the form it was typed, ending in `/`,
//! written so the shell reads it back as one word.

use wormhole_term::input::file_drop::shell_escape;

/// The text for the folder `name` under `prefix` (the typed text up to its last `/`, see
/// [`Target::prefix`](super::Target::prefix)): `prefix` + `name` + `/`, escaped for the shell.
pub fn inserted(prefix: &str, name: &str) -> String {
    shell_form(&format!("{prefix}{name}/"))
}

/// `path` escaped the way a dropped file's is (a backslash before each character a shell would take for
/// something else), except a leading `~/`, which the shell must see to turn it into the home folder: a
/// `\~` would be a folder named `~`.
pub fn shell_form(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => format!("~/{}", shell_escape(rest)),
        None => shell_escape(path),
    }
}

#[cfg(test)]
mod tests {
    use super::{inserted, shell_form};

    #[test]
    fn a_folder_ends_in_a_slash_and_keeps_the_form_it_was_typed_in() {
        assert_eq!(inserted("", "src"), "src/");
        assert_eq!(inserted("src/", "foo"), "src/foo/");
        assert_eq!(inserted("../", "bar"), "../bar/");
        assert_eq!(inserted("~/", "Projects"), "~/Projects/");
        assert_eq!(inserted("/usr/", "local"), "/usr/local/");
    }

    #[test]
    fn spaces_and_shell_characters_are_escaped() {
        assert_eq!(inserted("", "My Folder"), r"My\ Folder/");
        assert_eq!(inserted("~/", "My Folder"), r"~/My\ Folder/");
        assert_eq!(inserted("", "it's $HOME"), r"it\'s\ \$HOME/");
        assert_eq!(inserted("", "a&b;(c)"), r"a\&b\;\(c\)/");
    }

    #[test]
    fn only_a_leading_tilde_slash_is_left_to_the_shell() {
        assert_eq!(shell_form("~/x"), "~/x");
        assert_eq!(shell_form("a/~/x"), r"a/\~/x");
        assert_eq!(shell_form("~x/"), r"\~x/");
    }

    #[test]
    fn a_shell_reads_every_form_back_as_one_word() {
        for (prefix, name) in
            [("", "My Folder"), ("../", "a'b\"c"), ("src/", "$(touch x)"), ("/tmp/", "tab\there")]
        {
            let script = format!("printf %s {}", inserted(prefix, name));
            let out = std::process::Command::new("/bin/sh").args(["-c", &script]).output().unwrap();
            assert_eq!(String::from_utf8_lossy(&out.stdout), format!("{prefix}{name}/"), "{script}");
        }
    }
}
