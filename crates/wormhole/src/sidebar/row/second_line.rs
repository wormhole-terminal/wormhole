//! A row's second line: what the terminal has to report, or where it is, and the part of it that moves
//! with time (so a tick knows when to draw again).

use wormhole_core::view::sidebar::RowState;
use wormhole_core::view::sidebar::wording::{agent_detail, ago, clock, took};

use super::super::facts::RowInfo;
use crate::style::RowTone;

/// A row's second line: its words, their color, whether they are a command (code font; a path is
/// in the interface font, as the title row and a pane's header show it) and what sits at the
/// line's right.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::sidebar) struct SecondLine {
    pub(in crate::sidebar) text: Option<String>,
    pub(in crate::sidebar) tone: RowTone,
    pub(in crate::sidebar) mono: bool,
    pub(in crate::sidebar) trail: Option<String>,
}

/// The second line of every regular row: what the terminal has to report comes first; a quiet
/// one says where it is (`whereabouts`), and where even that is unknown it says what it is, so no
/// row is a single line. A compact or dense row has none (`compact`).
pub(in crate::sidebar) fn second_line(
    info: &RowInfo,
    compact: bool,
    whereabouts: impl FnOnce() -> Option<String>,
) -> SecondLine {
    let line = |text: Option<String>, tone, mono, trail| SecondLine { text, tone, mono, trail };
    if compact {
        return line(None, RowTone::Plain, false, None);
    }
    if info.told() {
        // An agent's, or a program's own report: its state in words.
        return match agent_detail(info.state, info.message.as_deref(), info.since()) {
            Some(text) => line(Some(text), tone_of(info.state), false, None),
            None => match whereabouts() {
                Some(place) => line(Some(place), RowTone::Plain, false, None),
                None => line(Some("Idle".to_string()), RowTone::Plain, false, None),
            },
        };
    }
    let trail = if info.unread { Some("new output".to_string()) } else { info.running().map(clock) };
    match (&info.command, &info.failure) {
        (Some(command), _) => line(Some(command.clone()), RowTone::Plain, true, trail),
        // The last command failed: what it was and how it ended, in danger, and when, until the
        // next command starts.
        (None, Some(failure)) => {
            let exit = format!("exit {}", failure.code);
            let text = match &failure.command {
                Some(command) => format!("{command} · {exit}"),
                None => exit,
            };
            line(Some(text), RowTone::Failed, true, Some(ago(failure.since())))
        }
        (None, None) => match whereabouts() {
            Some(place) => line(Some(place), RowTone::Plain, false, None),
            None => line(Some("Shell".to_string()), RowTone::Plain, false, None),
        },
    }
}

/// The part of a row's text that moves with time ("Working · 3m", a command's "1:12"), for
/// knowing when a tick has to redraw.
pub(in crate::sidebar) fn time_text(info: &RowInfo, compact: bool) -> Option<String> {
    if compact {
        return None;
    }
    match (info.told(), info.state) {
        (true, RowState::Working) if info.message.is_none() => took(info.since()),
        (true, RowState::Finished | RowState::Failed) if info.message.is_none() => Some(ago(info.since())),
        (false, _) => info.running().map(clock).or_else(|| info.failure.as_ref().map(|f| ago(f.since()))),
        _ => None,
    }
}

/// The second line's color for an agent's state.
fn tone_of(state: RowState) -> RowTone {
    match state {
        RowState::NeedsYou => RowTone::Attention,
        RowState::Finished => RowTone::Done,
        RowState::Failed => RowTone::Failed,
        RowState::Working | RowState::Idle | RowState::Exited => RowTone::Plain,
    }
}
