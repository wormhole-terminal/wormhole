//! The shell rule: a plain path to a system shell is the default shell and ticked; anything else is a
//! command that would run in every new terminal, so it is a profile and not ticked.

use crate::config::ShellProfile;
use crate::import::shell::{plain_words, propose};
use crate::import::{Env, Proposal, Setting, SettingValue, Source};

fn env() -> Env {
    let mut env = Env::new("/home/x".into());
    env.login_shell = "/bin/zsh".into();
    env.system_shells = vec!["/bin/zsh".into(), "/bin/bash".into(), "/opt/homebrew/bin/fish".into()];
    env
}

fn words(line: &str) -> Vec<String> {
    plain_words(line).expect("plain words")
}

fn proposed(line: &str) -> Proposal {
    let mut proposal = Proposal::default();
    propose(&mut proposal, &env(), Source::Ghostty, line, &words(line));
    proposal
}

#[test]
fn the_login_shell_and_a_listed_shell_are_the_default_shell_and_ticked() {
    for path in ["/bin/zsh", "/bin/bash", "/opt/homebrew/bin/fish"] {
        let p = proposed(path);
        let shell = p.get(Setting::DefaultShell).unwrap_or_else(|| panic!("{path}"));
        assert!(shell.default_on, "{path}");
        assert_eq!(shell.value, SettingValue::Shell(path.into()));
        assert!(p.get(Setting::ShellProfile).is_none());
    }
}

#[test]
fn arguments_make_a_profile_that_is_not_ticked() {
    let p = proposed("/bin/zsh -l -c 'tmux new -A'");
    assert!(p.get(Setting::DefaultShell).is_none());
    let profile = p.get(Setting::ShellProfile).expect("profile");
    assert!(!profile.default_on);
    assert_eq!(
        profile.value,
        SettingValue::Profile(ShellProfile {
            name: "zsh (Ghostty)".into(),
            command: "/bin/zsh".into(),
            args: Some(vec!["-l".into(), "-c".into(), "tmux new -A".into()]),
        })
    );
}

#[test]
fn a_program_that_is_not_a_listed_shell_is_a_profile_and_only_a_shell_runs_as_a_login_shell() {
    let p = proposed("/usr/local/bin/nu");
    let SettingValue::Profile(nu) = &p.get(Setting::ShellProfile).expect("profile").value else {
        panic!("not a profile")
    };
    assert!(!p.get(Setting::ShellProfile).expect("profile").default_on);
    // A shell by name starts as a login shell, as the shells always have.
    assert_eq!(nu.args, None);

    let p = proposed("claude");
    let SettingValue::Profile(claude) = &p.get(Setting::ShellProfile).expect("profile").value else {
        panic!("not a profile")
    };
    // `-l` means something else to a program that is not a shell.
    assert_eq!(claude.args, Some(Vec::new()));
}

#[test]
fn shell_syntax_is_not_words() {
    assert!(plain_words("sh -c 'a | b'").is_some());
    for line in ["a | b", "a && b", "echo $HOME", "a > b", "`x`", "(x)", "a; b", "ls *"] {
        assert!(plain_words(line).is_none(), "{line}");
    }
    assert_eq!(words(r#"a "b c" d\ e 'f g'"#), ["a", "b c", "d e", "f g"]);
}

#[test]
fn nothing_is_proposed_for_no_words() {
    let mut proposal = Proposal::default();
    propose(&mut proposal, &env(), Source::Kitty, "", &[]);
    propose(&mut proposal, &env(), Source::Kitty, "", &[" ".to_string()]);
    assert!(proposal.is_empty());
}
