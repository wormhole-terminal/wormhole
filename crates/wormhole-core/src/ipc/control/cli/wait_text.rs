//! The reply to `wait` and `run` as a person reads it. `run`'s text is the command's own output, then
//! a note about how it went: the two are kept apart ([`run_parts`]) so the command line can print the
//! output on stdout, where a script wants it, and the note on stderr.

use std::time::Duration;

use crate::ipc::control::printable::for_terminal;
use crate::ipc::control::{Met, Ran, Waited};

/// How long, as the readout of a block says it ("0.4s", "12s", "1m 5s").
fn took(ms: u64) -> String {
    wormhole_term::took_text(Duration::from_millis(ms))
}

pub(super) fn waited(waited: &Waited) -> String {
    let after = took(waited.after_ms);
    let terminal = waited.terminal;
    match waited.met {
        Met::Status => {
            let state = waited.status.map_or("in the state asked for", |status| status.word());
            format!("terminal {terminal} is {state} (after {after})\n")
        }
        Met::Text => {
            let found = waited.matched.as_deref().map(for_terminal).unwrap_or_default();
            format!("terminal {terminal} shows \"{found}\" (after {after})\n")
        }
        Met::Gone => format!("terminal {terminal}: the text is gone (after {after})\n"),
        Met::Stable => format!("terminal {terminal} has gone quiet (after {after})\n"),
        Met::BlockDone => {
            let block =
                waited.block.map(|block| format!("block {block}")).unwrap_or_else(|| "a command".into());
            let code = waited.exit_code.map(|code| format!(" with exit {code}")).unwrap_or_default();
            format!("terminal {terminal}: {block} finished{code} (after {after})\n")
        }
        Met::Other => format!("terminal {terminal}: done (after {after})\n"),
    }
}

/// `run`'s output as the lines it printed, and the note about how it ended.
pub(in crate::ipc) fn run_parts(ran: &Ran) -> (String, String) {
    let mut out = String::new();
    if let Some(output) = &ran.output {
        for line in output.lines() {
            out.push_str(&for_terminal(line));
            out.push('\n');
        }
    }
    let code = ran.exit_code.map_or_else(|| "no exit code".to_string(), |code| format!("exit {code}"));
    let mut note = format!("{code} in {} (block {})", took(ran.duration_ms), ran.block);
    if ran.truncated {
        note.push_str("; only the newest of the output is shown");
    }
    if let Some(why) = &ran.output_hidden {
        note.push_str(&format!("; output not shown: {}", for_terminal(why)));
    }
    note.push('\n');
    (out, note)
}

/// Both, for a caller that wants one string.
pub(super) fn ran(ran: &Ran) -> String {
    let (out, note) = run_parts(ran);
    format!("{out}{note}")
}

#[cfg(test)]
mod tests {
    use super::{ran, run_parts, waited};
    use crate::ipc::control::{Met, Ran, TerminalState, Waited};

    fn base(met: Met) -> Waited {
        Waited {
            terminal: 3,
            met,
            after_ms: 1800,
            status: None,
            matched: None,
            block: None,
            exit_code: None,
            revision: None,
        }
    }

    #[test]
    fn each_condition_is_a_line_that_says_which_met() {
        let status = Waited { status: Some(TerminalState::NeedsYou), ..base(Met::Status) };
        assert_eq!(waited(&status), "terminal 3 is needs you (after 1.8s)\n");
        let text = Waited { matched: Some("tests ok".into()), ..base(Met::Text) };
        assert_eq!(waited(&text), "terminal 3 shows \"tests ok\" (after 1.8s)\n");
        assert_eq!(waited(&base(Met::Gone)), "terminal 3: the text is gone (after 1.8s)\n");
        assert_eq!(waited(&base(Met::Stable)), "terminal 3 has gone quiet (after 1.8s)\n");
        let done = Waited { block: Some(58), exit_code: Some(101), ..base(Met::BlockDone) };
        assert_eq!(waited(&done), "terminal 3: block 58 finished with exit 101 (after 1.8s)\n");
    }

    #[test]
    fn what_the_screen_said_is_escaped() {
        let evil = Waited { matched: Some("x\u{1b}[2Jy".into()), ..base(Met::Text) };
        assert!(!waited(&evil).contains('\u{1b}'));
    }

    #[test]
    fn a_run_is_its_output_and_a_note_kept_apart() {
        let done = Ran {
            terminal: 3,
            block: 58,
            exit_code: Some(101),
            duration_ms: 3512,
            output: Some("test result: FAILED\n\u{1b}[31mred".into()),
            truncated: false,
            output_hidden: None,
        };
        let (out, note) = run_parts(&done);
        assert_eq!(out, "test result: FAILED\n\\u{1b}[31mred\n");
        assert_eq!(note, "exit 101 in 3.5s (block 58)\n");
        assert_eq!(ran(&done), format!("{out}{note}"));
        let hidden =
            Ran { output: None, output_hidden: Some("control_read is off".into()), exit_code: None, ..done };
        let (out, note) = run_parts(&hidden);
        assert_eq!(out, "");
        assert_eq!(note, "no exit code in 3.5s (block 58); output not shown: control_read is off\n");
    }
}
