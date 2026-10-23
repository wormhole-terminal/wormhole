//! How the app talks to a shell: a word quoted so that it arrives as one word, a script run with `-c`, and
//! the person's login shell asked to run a line (a profile's bare program name, `claude`, runs through it:
//! an app started from the Finder has a short `PATH`, and the login shell's startup files set the real one).
//! The POSIX forms are in `unix/`; a port to another shell family writes a sibling.

use std::process::Command;

use super::unix::shell as os;

/// `word` as one word of a command line: as it is when every character is safe, else single-quoted.
pub fn quote(word: &str) -> String {
    os::quote(word)
}

/// `word` in single quotes always, even when it needs none (a path typed at a prompt reads as one word
/// whatever it holds).
pub fn single_quote(word: &str) -> String {
    os::single_quote(word)
}

/// `words` as one command line, each word quoted as it needs.
pub fn quote_all<I, S>(words: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    os::quote_all(words)
}

/// A shell that runs `script` and exits; the arguments added to the command after it are the script's
/// own (`$0`, `$1`, ...).
pub fn script(script: &str) -> Command {
    os::script(script)
}

/// The program and arguments that run `line` in the login shell `shell`.
pub fn login_command(shell: &str, line: &str) -> (String, Vec<String>) {
    os::login_command(shell, line)
}

/// What goes before a script for a shell that starts the way a terminal's does (a login shell, interactive).
pub fn interactive_login_args() -> [&'static str; 3] {
    os::interactive_login_args()
}
