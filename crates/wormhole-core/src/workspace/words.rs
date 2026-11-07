//! What a row says: its second line, how long until that changes, and what VoiceOver reads. The
//! rules are the sidebar's (the app's `sidebar/row/second_line.rs` and `label.rs` hold the same
//! ones, drawn); here they are data a shell shows as it is.

use std::path::Path;
use std::time::Duration;

use crate::view::sidebar::spoken::{self, RowSpeech, SplitSpeech};
use crate::view::sidebar::wording::{agent_detail, ago, clock, place_detail, until_next};
use crate::view::sidebar::{RowState, SplitMarks};

use super::facts::{Millis, TerminalFacts};

/// How many characters of a path fit on a row's second line before it is cut from the front.
const PLACE_CHARS: usize = 38;

/// What the second line's words mean, for the colour a shell gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Plain,
    Attention,
    Done,
    Failed,
}

/// A row's second line: its words, their tone, whether they are a command (a code font; a path is in
/// the interface font) and what sits at the line's right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondLine {
    pub text: Option<String>,
    pub tone: Tone,
    pub mono: bool,
    pub trail: Option<String>,
}

fn since(now: Millis, at: Millis) -> Duration {
    Duration::from_millis(now.saturating_sub(at))
}

/// The second line of a regular row: what the terminal has to report comes first; a quiet one says
/// where it is, and where even that is unknown it says what it is, so no row is a single line.
pub(super) fn second_line(facts: &TerminalFacts, now: Millis, home: Option<&Path>) -> SecondLine {
    let line = |text: Option<String>, tone, mono, trail| SecondLine { text, tone, mono, trail };
    let whereabouts = || place_detail(facts.cwd.as_deref(), home, PLACE_CHARS);
    let state = facts.row_state();
    if facts.told() {
        // An agent's, or a program's own report: its state in words.
        return match agent_detail(state, facts.message.as_deref(), since(now, facts.since)) {
            Some(text) => line(Some(text), tone_of(state), false, None),
            // text: the line of a row that has neither a report nor a folder.
            None => line(Some(whereabouts().unwrap_or_else(|| "Idle".to_string())), Tone::Plain, false, None),
        };
    }
    let running = facts.running_since.map(|at| since(now, at));
    let trail = if facts.unread { Some("new output".to_string()) } else { running.map(clock) };
    match (&facts.command, &facts.failure) {
        (Some(command), _) => line(Some(command.clone()), Tone::Plain, true, trail),
        // The last command failed: what it was and how it ended, and when, until the next starts.
        (None, Some(failure)) => {
            let exit = format!("exit {}", failure.code);
            let text = match &failure.command {
                Some(command) => format!("{command} · {exit}"),
                None => exit,
            };
            line(Some(text), Tone::Failed, true, Some(ago(since(now, failure.at))))
        }
        (None, None) => {
            line(Some(whereabouts().unwrap_or_else(|| "Shell".to_string())), Tone::Plain, false, None)
        }
    }
}

/// How long until the words of `second_line` that move with time read differently: the next minute of
/// a status or failure they count, or the next second of a command's clock. `None` when none does,
/// so an idle window sets no timer.
pub(super) fn next_change(facts: &TerminalFacts, now: Millis) -> Option<Duration> {
    let minute = Duration::from_secs(60);
    match (facts.told(), facts.row_state()) {
        (true, RowState::Working | RowState::Finished | RowState::Failed) if facts.message.is_none() => {
            Some(until_next(since(now, facts.since), minute))
        }
        (false, _) => match facts.running_since {
            Some(at) => Some(until_next(since(now, at), Duration::from_secs(1))),
            None => facts.failure.as_ref().map(|failure| until_next(since(now, failure.at), minute)),
        },
        _ => None,
    }
}

fn tone_of(state: RowState) -> Tone {
    match state {
        RowState::NeedsYou => Tone::Attention,
        RowState::Finished => Tone::Done,
        RowState::Failed => Tone::Failed,
        RowState::Working | RowState::Idle | RowState::Exited => Tone::Plain,
    }
}

/// What VoiceOver says of a row: its name, what it is doing, its split and its place in the list.
pub(super) fn row_label(
    name: &str,
    facts: &TerminalFacts,
    now: Millis,
    place: spoken::Place,
    marks: SplitMarks,
) -> String {
    let state = facts.row_state();
    let speech = if facts.told() {
        RowSpeech::Agent { state, detail: facts.message.as_deref().filter(|_| state != RowState::Idle) }
    } else {
        RowSpeech::Plain {
            command: facts.command.as_deref(),
            running: facts.running_since.map(|at| since(now, at)),
            bell: facts.bell,
            unread: facts.unread,
            failed: facts.failure.as_ref().map(|failure| failure.code),
        }
    };
    spoken::row_label_in_split(name, speech, place, SplitSpeech::of(marks))
}
