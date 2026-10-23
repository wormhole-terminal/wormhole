//! Text inserted into the terminal when files are dropped on it.
//!
//! Like Terminal.app, each path is escaped for the shell and followed by a space, so several
//! files arrive as separate arguments and you can keep typing. Coding agents (Claude Code,
//! Codex, …) recognise a dropped image by that pasted path, so the escaping has to match what
//! they expect (`My\ File.png`, not `'My File.png'`).

use std::path::Path;

/// The text to insert for `paths`: every path escaped, each followed by one space.
pub fn dropped_text<P: AsRef<Path>>(paths: &[P]) -> String {
    let mut text = String::new();
    for path in paths {
        text.push_str(&shell_escape(&path.as_ref().to_string_lossy()));
        text.push(' ');
    }
    text
}

/// Escape `path` so a POSIX-style shell (sh, bash, zsh, fish) reads it back as one word.
///
/// Anything outside a small safe set gets a backslash. Non-ASCII characters stay as they are
/// (no shell treats them specially, and macOS screenshots put a narrow no-break space in their
/// names that agents match literally). A backslash can't express a control character (a
/// backslash-newline vanishes), so those paths are single-quoted instead.
pub(crate) fn shell_escape(path: &str) -> String {
    if path.chars().any(char::is_control) {
        return format!("'{}'", path.replace('\'', r"'\''"));
    }
    let mut out = String::with_capacity(path.len() + 8);
    for c in path.chars() {
        if !(c.is_ascii_alphanumeric() || !c.is_ascii() || "/._-+,:@%".contains(c)) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{dropped_text, shell_escape};

    #[test]
    fn plain_paths_are_left_alone() {
        assert_eq!(
            shell_escape("/Users/me/Pictures/cat-1_final.v2.png"),
            "/Users/me/Pictures/cat-1_final.v2.png"
        );
    }

    #[test]
    fn spaces_and_shell_metacharacters_are_backslashed() {
        assert_eq!(shell_escape("/tmp/My File (1).png"), r"/tmp/My\ File\ \(1\).png");
        assert_eq!(shell_escape("/tmp/a&b;c$d`e"), r"/tmp/a\&b\;c\$d\`e");
        assert_eq!(
            shell_escape(r#"/tmp/it's "x" \ !*?[]{}<>|#~=^"#),
            r#"/tmp/it\'s\ \"x\"\ \\\ \!\*\?\[\]\{\}\<\>\|\#\~\=\^"#
        );
    }

    #[test]
    fn non_ascii_text_is_kept_verbatim() {
        // macOS screenshots: U+202F (narrow no-break space) before AM/PM.
        let shot = "/Users/me/Desktop/Screenshot 2026-09-30 at 10.15.32\u{202f}AM.png";
        assert_eq!(
            shell_escape(shot),
            "/Users/me/Desktop/Screenshot\\ 2026-09-30\\ at\\ 10.15.32\u{202f}AM.png"
        );
        assert_eq!(shell_escape("/tmp/café/日本語.txt"), "/tmp/café/日本語.txt");
    }

    #[test]
    fn control_characters_switch_to_single_quotes() {
        assert_eq!(shell_escape("/tmp/a\nb"), "'/tmp/a\nb'");
        assert_eq!(shell_escape("/tmp/it's\ta"), "'/tmp/it'\\''s\ta'");
    }

    #[test]
    fn each_path_is_followed_by_a_space() {
        let paths = [PathBuf::from("/tmp/a b.png"), PathBuf::from("/tmp/c.txt")];
        assert_eq!(dropped_text(&paths), r"/tmp/a\ b.png /tmp/c.txt ");
        assert_eq!(dropped_text::<PathBuf>(&[]), "");
    }

    /// The real check: a shell must read every escaped path back as exactly the original.
    #[test]
    fn a_shell_reads_the_escaped_path_back_unchanged() {
        let names = [
            "/tmp/plain.png",
            "/tmp/My File (1).png",
            "/tmp/it's \"quoted\" & $HOME `x` \\ !*?[]{}<>|#~=^;",
            "/tmp/tab\there",
            "/tmp/new\nline",
            "/tmp/quote's\nmix",
            "/tmp/Screenshot 10.15.32\u{202f}AM.png",
            "/tmp/café 日本語",
        ];
        for name in names {
            let script = format!("printf %s {}", shell_escape(name));
            let out = std::process::Command::new("/bin/sh").args(["-c", &script]).output().expect("run sh");
            assert!(out.status.success(), "sh rejected {script:?}");
            assert_eq!(String::from_utf8_lossy(&out.stdout), name, "script: {script:?}");
        }
    }
}
