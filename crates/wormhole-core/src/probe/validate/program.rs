//! The spelling of a bare program name in `permissions.process` and in an argv.
//!
//! macOS volumes are case-insensitive by default, so `GIT` starts the same `git` as `git` does.
//! Every check that keys on a name (`git` runs read-only through the app, a shell is a red line)
//! compares exact text, so a name that could be spelt another way would walk around them. A bare
//! name is therefore one spelling only: lower case, and not a program that is `git` under another
//! name.

/// Why `name` cannot be a bare program name, if it cannot.
pub(crate) fn bare_name_problem(name: &str) -> Option<String> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '+' | '-'));
    if !(first_ok && rest_ok) {
        return Some(format!(
            "`{name}` is not a bare program name: use lower case a-z, 0-9 and `.`, `_`, `+`, `-`, starting with a letter or digit \
             (the disk does not tell `GIT` from `git`, so only one spelling is accepted)"
        ));
    }
    // `git-upload-pack`, `git-shell` and the like are git's own programs, run with none of the
    // checks `git` gets; `scalar` is git's wrapper.
    if name.starts_with("git-") || name == "scalar" {
        return Some(format!("`{name}` is part of git; list `git` and use its read-only subcommands"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_name_has_one_spelling() {
        for ok in ["git", "say", "jq", "python3.12", "g++", "c_x", "7z", "node22"] {
            assert_eq!(bare_name_problem(ok), None, "{ok}");
        }
        for bad in ["GIT", "Git", "gIt", "", "-rf", ".hidden", "a b", "a/b", "é", "git\u{200b}", "a\u{202e}b"]
        {
            assert!(bare_name_problem(bad).is_some(), "{bad:?}");
        }
    }

    #[test]
    fn git_s_other_programs_are_refused_and_point_at_git() {
        for bad in
            ["git-upload-pack", "git-receive-pack", "git-shell", "git-upload-archive", "git-lfs", "scalar"]
        {
            let why = bare_name_problem(bad).unwrap_or_else(|| panic!("{bad}"));
            assert!(why.contains("`git`"), "{why}");
        }
        assert_eq!(bare_name_problem("gitk"), None, "not git-*");
    }
}
