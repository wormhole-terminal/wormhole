//! A terminal opened beside another on any of the four sides, as the GPUI app's `control_split` does: the model
//! makes it standing alone, then it joins the target's pane on a drop zone's side (`Action::JoinSplit`, which
//! refuses, changing nothing, where either pane would end up under the minimum size). The model's own
//! `SplitBeside` only knows right and below.

use std::path::PathBuf;

use wormhole_core::ipc::control::Side;
use wormhole_core::view::panes::Zone;
use wormhole_core::workspace::{Action, Event};

use crate::workspace::Shared;

/// The side of the target's pane a new terminal goes, as a drop zone.
pub(super) fn zone_of(side: Side) -> Zone {
    match side {
        Side::Right => Zone::Right,
        Side::Down => Zone::Bottom,
        Side::Left => Zone::Left,
        Side::Up => Zone::Top,
    }
}

/// A new terminal in window `shared` on `side` of `anchor`, in `folder` or where `anchor` is: its id and folder, with the events
/// handed to Swift, which starts its program. `None` when there was no room: nothing was made, and Swift hears
/// of nothing but the selection coming back to where it was.
pub(super) fn open_beside(
    shared: &Shared,
    anchor: i64,
    side: Side,
    folder: Option<String>,
) -> Option<(i64, Option<String>)> {
    let zone = zone_of(side);
    let (events, made) = shared.with_model(|model| {
        let before = model.selected();
        let start = model.facts(anchor).and_then(|facts| facts.cwd.clone());
        let mut events = model.apply(Action::NewTerminal);
        let Some(id) = events.iter().find_map(|event| match event {
            Event::Opened { id, .. } => Some(*id),
            _ => None,
        }) else {
            return (events, None);
        };
        // The new terminal starts where the target is (`NewTerminal` says where the selected one is).
        let cwd: Option<PathBuf> = folder.map(PathBuf::from).or(start);
        for event in &mut events {
            if let Event::Opened { id: opened, cwd: at } = event
                && *opened == id
            {
                at.clone_from(&cwd);
            }
        }
        if let Some(cwd) = &cwd {
            events.extend(model.update_facts(id, |facts| facts.cwd = Some(cwd.clone())));
        }
        let joined = model.apply(Action::JoinSplit { id, target: anchor, zone });
        let no_room = joined.iter().any(|event| matches!(event, Event::Refused(_)));
        events.extend(joined);
        if !no_room {
            return (events, Some((id, cwd.map(|dir| dir.to_string_lossy().into_owned()))));
        }
        // The terminal made for nothing goes again before Swift starts a program for it.
        events.extend(model.apply(Action::Close(id)));
        if let Some(before) = before {
            events.extend(model.apply(Action::Select(before)));
        }
        events.retain(|event| !matches!(event, Event::Opened { id: gone, .. } | Event::Closed { id: gone } if *gone == id));
        (events, None)
    });
    shared.post(events);
    shared.pump.kick();
    made
}
