//! What another terminal runs in a new terminal, as Wormhole's shell settings.
//!
//! A plain path to the login shell or to a shell `/etc/shells` lists is the default shell, and is
//! ticked. Anything else (arguments, a bare program name, a path the system does not list as a
//! shell) is a command Wormhole would run in every new terminal it opens, so it becomes a shell
//! profile that is not ticked until the person says so.

use super::env::Env;
use super::proposal::{Proposal, Setting, SettingValue};
use super::source::Source;
use crate::config::ShellProfile;
use crate::terminal::shells::program_name;

/// Propose the command `words` (a program and its arguments) for `source`, written as `from`.
pub(crate) fn propose(proposal: &mut Proposal, env: &Env, source: Source, from: &str, words: &[String]) {
    let Some((program, args)) = words.split_first() else { return };
    if program.trim().is_empty() {
        return;
    }
    // A control character in a command (an escape, a line break) is never one a person typed into a
    // config; it cannot be shown as it runs, so it is not offered.
    if words.iter().any(|word| word.chars().any(char::is_control)) {
        proposal.skip(from, super::proposal::Reason::UnsupportedValue);
        return;
    }
    let is_system_shell = program == &env.login_shell || env.system_shells.contains(program);
    if args.is_empty() && is_system_shell {
        proposal.propose(Setting::DefaultShell, from, SettingValue::Shell(program.clone()));
        return;
    }
    let profile = ShellProfile {
        name: format!("{} ({})", program_name(program), source.brand()),
        command: program.clone(),
        // A shell runs as a login shell when it is given no arguments; a program that is not a shell
        // takes none, since `-l` means something else to it.
        args: if args.is_empty() { (!is_shell(program)).then(Vec::new) } else { Some(args.to_vec()) },
    };
    proposal.propose_unticked(Setting::ShellProfile, from, SettingValue::Profile(profile));
}

/// Whether a program is named like a shell, which Wormhole starts as a login shell.
fn is_shell(program: &str) -> bool {
    const SHELLS: [&str; 12] =
        ["sh", "bash", "zsh", "fish", "dash", "ksh", "tcsh", "csh", "nu", "elvish", "xonsh", "pwsh"];
    SHELLS.contains(&program_name(program))
}

/// The words of a command line, when it is only words: a program and its arguments. A line that
/// pipes, redirects, expands or chains is shell syntax, which a profile cannot hold.
pub(crate) fn plain_words(text: &str) -> Option<Vec<String>> {
    let syntax = |c: char| matches!(c, '|' | '&' | ';' | '<' | '>' | '$' | '`' | '(' | ')' | '*' | '?');
    let unquoted = {
        // Quoted text may hold any of them; only what is outside quotes is syntax.
        let mut outside = String::new();
        let mut quote: Option<char> = None;
        for c in text.chars() {
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '\'' || c == '"' => quote = Some(c),
                None => outside.push(c),
            }
        }
        outside
    };
    (!unquoted.chars().any(syntax)).then(|| words(text))
}

/// Where `text` splits into the words a command line is made of: whitespace, with `'…'`, `"…"` and
/// `\` quoting. Nothing is expanded and nothing runs.
pub(crate) fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => match chars.next() {
                Some(next @ ('"' | '\\')) => current.push(next),
                Some(next) => {
                    current.push('\\');
                    current.push(next);
                }
                None => current.push('\\'),
            },
            (Some(_), c) => current.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, '\\') => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    started = true;
                }
            }
            (None, c) if c.is_whitespace() => {
                if started || !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            (None, c) => {
                current.push(c);
                started = true;
            }
        }
    }
    if started || !current.is_empty() {
        out.push(current);
    }
    out
}
