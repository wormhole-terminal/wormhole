//! The question before quitting: how it is asked (a plain Quit and Cancel, or with the box that keeps the
//! terminals running in the background) and what it says, from the facts of the terminals that run something.
//! The sentences are the catalog's (`dialog.ftl`), with plurals chosen there; what still runs, listed the
//! way the questions before closing list it, is `session::still_running`'s.

use wormhole_text::{keys, text, whole};

use super::session::still_running;
use crate::terminal_view::Status;

/// What the question before quitting says: the terminals come back, what runs in them doesn't.
pub(crate) fn quit_detail(
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
) -> String {
    quit_detail_with(text(keys::DIALOG_QUIT_ENDS).to_string(), running, programs, other_profiles)
}

/// [`quit_detail`] under another opening sentence.
fn quit_detail_with(
    head: String,
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
) -> String {
    let mut elsewhere = String::new();
    for (profile, count) in other_profiles {
        elsewhere.push(' ');
        elsewhere.push_str(&keys::dialog_quit_other_profile(whole(*count), profile));
    }
    format!("{head}{}{elsewhere}", still_running(running, programs))
}

/// How the question before quitting is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitAsk {
    /// Quit / Cancel only: the sheet GPUI draws.
    Plain,
    /// With the box "Keep terminals running in the background". `local_running`: some terminal
    /// that runs something can't be kept (it runs in the app's own process), so it ends either way.
    KeepBox { local_running: bool },
}

/// How ⌘Q asks, given how many terminals that run something are hosted (`hosted_running`) or local
/// (`local_running`), and whether any terminal at all is hosted (`any_hosted`: the box is about
/// those, idle ones too). `None`: nothing runs, so quit without asking. Quit and End All Terminals
/// (`end_hosts`) ends everything, so it has nothing to choose.
pub(crate) fn quit_ask(
    end_hosts: bool,
    hosted_running: usize,
    local_running: usize,
    any_hosted: bool,
) -> Option<QuitAsk> {
    if hosted_running + local_running == 0 {
        return None;
    }
    Some(if end_hosts || !any_hosted {
        QuitAsk::Plain
    } else {
        QuitAsk::KeepBox { local_running: local_running > 0 }
    })
}

/// Whether quitting ends the session hosts too: the box "Keep terminals running in the background"
/// left unticked.
pub(crate) fn quit_ends_hosts(keep_running: bool) -> bool {
    !keep_running
}

/// What the question before quitting says when it has the box: what runs in terminals that are
/// kept goes on, what runs in the others ends. Without a kept terminal that runs something there is
/// nothing to add to [`quit_detail`], which stays true whichever way the box is.
pub(crate) fn quit_detail_keeping(
    running: &[(Status, bool)],
    programs: &[String],
    other_profiles: &[(String, usize)],
    hosted_running: usize,
    local_running: usize,
) -> String {
    if hosted_running == 0 {
        return quit_detail(running, programs, other_profiles);
    }
    // Said from the counts: what is kept goes on, and what runs inside the app ends, however the box is.
    let head = if local_running == 0 {
        text(keys::DIALOG_QUIT_KEEPS_ALL).to_string()
    } else {
        keys::dialog_quit_keeps_some(whole(hosted_running), whole(local_running))
    };
    quit_detail_with(head, running, programs, other_profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(programs: &[&str]) -> Vec<String> {
        programs.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn quitting_says_the_terminals_come_back_but_their_programs_end() {
        assert_eq!(
            quit_detail(&[(Status::Idle, true)], &names(&["claude", "caffeinate"]), &[]),
            "Your terminals come back next time, but what runs in them ends. Still running: claude, \
             caffeinate."
        );
    }

    #[test]
    fn quitting_names_the_other_profiles_that_still_run_something() {
        assert_eq!(
            quit_detail(&[(Status::Busy, false)], &[], &[("Work".into(), 1), ("Client".into(), 3)]),
            "Your terminals come back next time, but what runs in them ends. An agent is still working. \
             1 terminal in Work is running an agent or a command. \
             3 terminals in Client are running an agent or a command."
        );
    }

    #[test]
    fn quitting_with_the_box_says_what_goes_on_and_what_ends() {
        let running = [(Status::Idle, true)];
        let programs = names(&["claude"]);
        // Nothing kept runs something: the plain wording is true either way.
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[], 0, 1),
            quit_detail(&running, &programs, &[])
        );
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[], 2, 0),
            "Your terminals come back next time and what runs in them keeps going. Untick the box to end \
             them. Still running: claude."
        );
        assert_eq!(
            quit_detail_keeping(&running, &programs, &[("Work".into(), 1)], 2, 1),
            "Your terminals come back next time. 2 terminals keep running; 1 terminal ends because it runs \
             inside Wormhole. Untick the box to end the others too. Still running: claude. \
             1 terminal in Work is running an agent or a command."
        );
        assert!(
            quit_detail_keeping(&running, &programs, &[], 1, 3)
                .contains("1 terminal keeps running; 3 terminals end because they run inside Wormhole.")
        );
    }

    #[test]
    fn the_box_shows_when_a_terminal_is_hosted_and_something_runs() {
        use QuitAsk::{KeepBox, Plain};
        // Nothing runs: no question, hosted or not.
        assert_eq!(quit_ask(false, 0, 0, true), None);
        assert_eq!(quit_ask(true, 0, 0, true), None);
        // No hosted terminal at all: today's question.
        assert_eq!(quit_ask(false, 0, 2, false), Some(Plain));
        // A hosted terminal that runs something, or an idle one beside a local one that does.
        assert_eq!(quit_ask(false, 1, 0, true), Some(KeepBox { local_running: false }));
        assert_eq!(quit_ask(false, 1, 2, true), Some(KeepBox { local_running: true }));
        assert_eq!(quit_ask(false, 0, 1, true), Some(KeepBox { local_running: true }));
        // Quit and End All Terminals has no choice to offer.
        assert_eq!(quit_ask(true, 1, 1, true), Some(Plain));
    }

    #[test]
    fn the_box_ticked_keeps_the_hosts_and_unticked_ends_them() {
        assert!(!quit_ends_hosts(true));
        assert!(quit_ends_hosts(false));
    }

    /// The plurals are the catalog's: one terminal and several, on each side of the semicolon.
    #[test]
    fn the_counts_choose_the_singular_or_the_plural() {
        let said = |kept, ended| quit_detail_keeping(&[], &[], &[], kept, ended);
        assert!(said(1, 1).contains("1 terminal keeps running; 1 terminal ends because it runs inside"));
        assert!(said(3, 2).contains("3 terminals keep running; 2 terminals end because they run inside"));
    }
}
