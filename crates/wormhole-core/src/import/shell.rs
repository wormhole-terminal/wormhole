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
    (!unquoted.chars().any(syntax)).then(|| super::units::words(text))
}
