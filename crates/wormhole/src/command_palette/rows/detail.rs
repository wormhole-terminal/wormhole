//! The words under a terminal's name in the palette.

use std::time::Instant;

use wormhole_core::notify::{self, State};

use super::items::TerminalItem;

/// A terminal's detail in the palette: its window and folder in a scope ("Window 1 · terminal ·
/// 4:12"), else what it is doing ("Needs input · Approve cargo test?", "terminal · Editing …").
pub(crate) fn terminal_detail(item: &TerminalItem, scoped: bool, now: Instant) -> String {
    let terminal = &item.terminal;
    let elapsed = notify::elapsed(terminal.state, now.saturating_duration_since(terminal.since));
    if scoped {
        let window = format!("Window {}", item.window_number);
        return [Some(window), terminal.folder.clone(), elapsed]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
    }
    let doing = match terminal.state {
        State::NeedsYou => Some("Needs input".to_string()),
        State::Done => Some("Finished".to_string()),
        State::Failed => Some("Failed".to_string()),
        State::Working | State::Idle | State::Exited => terminal.folder.clone(),
    };
    let exit = terminal.exit.map(|code| format!("exit {code}"));
    let words = match terminal.state {
        State::NeedsYou | State::Working => terminal.message.clone().or_else(|| terminal.command.clone()),
        // What went wrong: a program's own words, else the command and how it exited.
        State::Failed => terminal.message.clone().or_else(|| {
            let parts: Vec<String> = [terminal.command.clone(), exit].into_iter().flatten().collect();
            (!parts.is_empty()).then(|| parts.join(" · "))
        }),
        _ => None,
    };
    [doing, words].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

/// The line under a terminal that waits for an answer, in the Terminals scope.
pub(crate) fn question(item: &TerminalItem) -> Option<String> {
    let terminal = &item.terminal;
    if terminal.state != State::NeedsYou {
        return None;
    }
    let message = terminal.message.as_deref()?;
    Some(match &terminal.who {
        Some(who) => format!("{who} is asking: {message}"),
        None => message.to_string(),
    })
}
