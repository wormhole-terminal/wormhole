//! The host's events as a local terminal's: the same `PtyEvent`s and the same shared state
//! (`Shared`: the folder report, the prompt state and the command log, progress, program status,
//! the exit), through the same `EventProxy` calls the engine's own events take, so the app reads a
//! hosted terminal as it reads a local one.

use std::path::PathBuf;
use std::sync::Arc;

use wormhole_vt::repaint::Seeds;
use wormhole_wire::{self as wire, EventKind, MarkKind};

use super::link::{Cmd, Link};
use crate::events::{EventProxy, PtyEvent, Reply, TextFormatter};
use crate::hosted::window_op_from_wire;
use crate::osc::{Mark, Notification, Osc, Progress, ProgressState, PromptKind};
use crate::program_status::{self, Parsed};

/// What an event was, for the connection's thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Heard {
    /// Delivered (or nothing to deliver).
    Done,
    /// The counters to seed the replica with before the repaint that follows.
    Seeds(Seeds),
    /// The program exited.
    Exited,
}

/// Deliver `kind` to the app through `proxy`; a clipboard read is answered through `link`.
pub(super) fn deliver(proxy: &EventProxy, link: &Arc<Link>, kind: EventKind<'_>) -> Heard {
    match kind {
        EventKind::Title(title) => proxy.send(PtyEvent::Title(title.to_owned())),
        EventKind::ResetTitle => proxy.send(PtyEvent::ResetTitle),
        EventKind::Bell => proxy.send(PtyEvent::Bell),
        // The host sends only a folder of its machine, which is this one; `osc` checks it is a
        // folder still.
        EventKind::Cwd(path) => proxy.osc(Osc::Cwd { host: String::new(), path: PathBuf::from(path) }),
        EventKind::Mark(wired) => {
            if let Some(mark) = mark_of(wired) {
                proxy.osc(Osc::Mark(mark, wired.block));
            }
        }
        EventKind::Notify { title, body, .. } => {
            proxy.osc(Osc::Notify(Notification { title: title.to_owned(), body: body.to_owned() }))
        }
        EventKind::Progress { state, percent } => {
            let state = match state {
                wire::ProgressState::Remove => None,
                wire::ProgressState::Set => Some(ProgressState::Normal),
                wire::ProgressState::Error => Some(ProgressState::Error),
                wire::ProgressState::Indeterminate => Some(ProgressState::Indeterminate),
                wire::ProgressState::Paused => Some(ProgressState::Paused),
                wire::ProgressState::Unknown(_) => return Heard::Done,
            };
            let progress = state.map(|state| Progress { state, percent: percent.min(100) });
            proxy.osc(Osc::Progress(progress));
        }
        // The host accepted the report; it is parsed here as the engine's would be (a BEL ends it).
        EventKind::ProgramStatus(body) => {
            if let Some(Parsed::Report(report)) = program_status::parse(body.as_bytes(), 1) {
                proxy.osc(Osc::ProgramStatus(report));
            }
        }
        EventKind::ClipboardStore { data, .. } => {
            proxy.send(PtyEvent::ClipboardStore(String::from_utf8_lossy(data).into_owned()))
        }
        EventKind::ClipboardLoad { ticket, .. } => {
            // The host formats the program's answer; the app's text goes to it as it is.
            let format: TextFormatter = Arc::new(|text: &str| text.to_owned());
            let link = link.clone();
            let reply = Reply::hosted(
                u64::from(ticket),
                Arc::new(move |ticket, data| link.send(Cmd::Clipboard { ticket: ticket as u32, data })),
            );
            proxy.send(PtyEvent::ClipboardLoad(format, reply));
        }
        EventKind::WindowOp { op, a, b } => {
            if let Some(op) = window_op_from_wire(op, a, b) {
                proxy.send(PtyEvent::WindowOp(op));
            }
        }
        EventKind::Exited(exit) => {
            let (code, signal) = exit_of(exit);
            proxy.exited(code, signal);
            return Heard::Exited;
        }
        EventKind::FullReset => proxy.osc(Osc::FullReset),
        EventKind::RepaintSeeds { block, link_seq, image_handle } => {
            return Heard::Seeds(Seeds { block, link_seq, image_handle });
        }
        EventKind::Unknown(kind) => tracing::debug!(kind, "an event of a newer host, skipped"),
    }
    Heard::Done
}

/// A wire mark as the shell's mark (a kind of a newer host is none).
fn mark_of(mark: wire::Mark) -> Option<Mark> {
    Some(match mark.kind {
        MarkKind::PromptStart => Mark::PromptStart(PromptKind::Initial),
        MarkKind::CommandStart => Mark::CommandStart,
        MarkKind::OutputStart => Mark::OutputStart,
        MarkKind::CommandEnd => Mark::Finished(mark.exit_code),
        MarkKind::Unknown(_) => return None,
    })
}

/// The exit as a local terminal reports it: the code, or the signal (the host says -1 for "the
/// system could not tell", and a code of 0 beside a signal).
pub(super) fn exit_of(exit: wire::ExitInfo) -> (Option<i32>, Option<i32>) {
    match (exit.code, exit.signal) {
        (_, 0) if exit.code == -1 => (None, None),
        (code, 0) => (Some(code), None),
        (_, signal) => (None, Some(i32::from(signal))),
    }
}
