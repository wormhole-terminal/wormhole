//! POSIX shells: how a word is quoted for `sh`, `bash`, `zsh` and `fish` alike, and how a script or a
//! login shell is started with `-c`.

use std::process::Command;

use wormhole_term::sys::process::command;

/// The shell that runs a script the app brings itself, whatever the person's shell is.
const SCRIPT_SHELL: &str = "/bin/sh";

/// `sh -c script`: the arguments added after it are `$0`, `$1`, ...
pub(in crate::platform) fn script(script: &str) -> Command {
    let mut sh = command(SCRIPT_SHELL);
    sh.arg("-c").arg(script);
    sh
}

/// What runs `line` in a login shell: the shell with `-l -c line`, so the person's startup files have set
/// their real `PATH` first (an app started from the Finder has a short one).
pub(in crate::platform) fn login_command(shell: &str, line: &str) -> (String, Vec<String>) {
    (shell.to_string(), vec!["-l".into(), "-c".into(), line.to_string()])
}

/// The arguments before a script for a shell that starts as a terminal's does: a login shell, interactive,
/// so every startup file runs.
pub(in crate::platform) fn interactive_login_args() -> [&'static str; 3] {
    ["-l", "-i", "-c"]
}

/// `word` as one word of a POSIX shell command line: as it is when every character is safe, else
/// in single quotes, with a quote inside written as `'\''` (the quoting zsh, bash, fish and sh
/// read alike).
pub(in crate::platform) fn quote(word: &str) -> String {
    if !word.is_empty() && word.chars().all(is_safe) {
        return word.to_string();
    }
    single_quote(word)
}

/// `word` in single quotes always, even when it needs none (a path typed at a prompt reads as one
/// word whatever it holds): a quote inside is written as `'\''`.
pub(in crate::platform) fn single_quote(word: &str) -> String {
    let mut out = String::with_capacity(word.len() + 2);
    out.push('\'');
    for c in word.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// `words` as one command line, each word quoted as it needs.
pub(in crate::platform) fn quote_all<I, S>(words: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    words.into_iter().map(|word| quote(word.as_ref())).collect::<Vec<_>>().join(" ")
}

/// A character no shell reads as anything but itself, outside quotes.
fn is_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '@' | '%' | '+' | ',' | '=')
}

#[cfg(test)]
mod tests {
    use super::{quote, quote_all, single_quote};

    #[test]
    fn safe_words_stay_bare() {
        for word in ["claude", "/opt/homebrew/bin/fish", "-l", "--model=opus", "a,b", "v1.2+3", "x@y:z%"] {
            assert_eq!(quote(word), word);
        }
    }

    #[test]
    fn a_single_quoted_word_is_quoted_even_when_it_is_safe() {
        assert_eq!(single_quote("/Users/ben/Projects/tube"), "'/Users/ben/Projects/tube'");
        assert_eq!(single_quote("/tmp/my dir/$HOME"), "'/tmp/my dir/$HOME'");
        assert_eq!(single_quote("/tmp/it's"), r#"'/tmp/it'\''s'"#);
        assert_eq!(single_quote(""), "''");
    }

    #[test]
    fn anything_a_shell_would_read_is_quoted() {
        assert_eq!(quote(""), "''");
        assert_eq!(quote("two words"), "'two words'");
        assert_eq!(quote("$HOME"), "'$HOME'");
        assert_eq!(quote("a;rm -rf"), "'a;rm -rf'");
        assert_eq!(quote("it's"), r#"'it'\''s'"#);
        assert_eq!(quote("*"), "'*'");
        assert_eq!(quote("tab\there"), "'tab\there'");
    }

    #[test]
    fn a_command_line_joins_its_words() {
        assert_eq!(quote_all(["exec", "claude", "--model", "opus 4"]), "exec claude --model 'opus 4'");
        assert_eq!(quote_all(Vec::<String>::new()), "");
        assert_eq!(quote_all([String::from("npm"), String::from("run")]), "npm run");
    }
}
