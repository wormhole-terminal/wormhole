//! How text from another terminal's files is shown on the review list. The files are not trusted
//! (they may come from a cloned dotfiles repository), and what a person reads before ticking a row
//! must be what it would do: a command is written as a shell reads it, and a character that moves or
//! hides text (a control character, a right-to-left override, a zero-width mark) is spelled out.

use crate::config::ShellProfile;

/// `text` with every control character and every character that changes how text is laid out or
/// hides itself written as `\u{…}`, so a line cannot show one thing and hold another.
pub fn visible(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() || is_invisible(c) {
            out.push_str(&format!("\\u{{{:04x}}}", u32::from(c)));
        } else {
            out.push(c);
        }
    }
    out
}

/// Characters with no glyph of their own, or that reorder the text around them: the bidirectional
/// marks, embeddings, overrides and isolates, the zero-width and joining marks, the line and
/// paragraph separators and the byte order mark.
fn is_invisible(c: char) -> bool {
    matches!(
        u32::from(c),
        0x061C | 0x180E | 0x200B..=0x200F | 0x2028..=0x202E | 0x2060..=0x206F | 0xFEFF
    )
}

/// The command a profile runs, as it would be typed: the program and its arguments, each quoted when
/// it holds whitespace or anything a shell reads, so the words are the ones that run. Anything
/// invisible in it is spelled out ([`visible`]).
pub fn command_line(profile: &ShellProfile) -> String {
    let mut line = quoted(&profile.command);
    for arg in profile.args.iter().flatten() {
        line.push(' ');
        line.push_str(&quoted(arg));
    }
    visible(&line)
}

/// One word as a shell reads it back: as it is when it holds only plain characters, else in single
/// quotes (a single quote inside is `'\''`).
fn quoted(word: &str) -> String {
    let plain = |c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ',' | ':' | '=' | '@' | '%' | '+')
    };
    if !word.is_empty() && word.chars().all(plain) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(command: &str, args: &[&str]) -> ShellProfile {
        ShellProfile {
            name: "x".into(),
            command: command.into(),
            args: Some(args.iter().map(|arg| (*arg).to_string()).collect()),
        }
    }

    #[test]
    fn plain_words_are_shown_as_they_are() {
        assert_eq!(
            command_line(&profile("/opt/homebrew/bin/fish", &["-l", "--init-command=a,b"])),
            "/opt/homebrew/bin/fish -l --init-command=a,b"
        );
        assert_eq!(
            command_line(&ShellProfile { name: "x".into(), command: "htop".into(), args: None }),
            "htop"
        );
    }

    #[test]
    fn a_word_with_spaces_or_shell_syntax_is_quoted_so_the_words_stay_whole() {
        // One argument of three words is one argument; typed back it would still be.
        assert_eq!(command_line(&profile("zsh", &["-c", "echo hi; rm x"])), "zsh -c 'echo hi; rm x'");
        assert_eq!(command_line(&profile("zsh", &["it's", ""])), "zsh 'it'\\''s' ''");
        assert_eq!(command_line(&profile("/Applications/My App/sh", &[])), "'/Applications/My App/sh'");
    }

    #[test]
    fn a_quoted_command_reads_back_as_the_same_words() {
        let original = profile("/bin/zsh", &["-c", "a b", "it's", "$HOME", ""]);
        let shown = command_line(&original);
        let mut words = super::super::shell::words(&shown);
        let program = words.remove(0);
        assert_eq!(program, original.command);
        assert_eq!(Some(words), original.args);
    }

    #[test]
    fn control_and_bidi_characters_are_spelled_out() {
        assert_eq!(visible("a\u{1b}[2Jb"), "a\\u{001b}[2Jb");
        assert_eq!(visible("txt\u{202e}gpj.sh"), "txt\\u{202e}gpj.sh");
        assert_eq!(
            visible("a\u{2066}b\u{2069}c\u{200b}d\u{feff}"),
            "a\\u{2066}b\\u{2069}c\\u{200b}d\\u{feff}"
        );
        assert_eq!(visible("tab\there\nnewline"), "tab\\u{0009}here\\u{000a}newline");
        // Ordinary text, including other scripts and emoji, is left alone.
        assert_eq!(visible("Zeichen é 日本語 🙂"), "Zeichen é 日本語 🙂");
        assert_eq!(command_line(&profile("sh", &["\u{202e}"])), "sh '\\u{202e}'");
    }
}
