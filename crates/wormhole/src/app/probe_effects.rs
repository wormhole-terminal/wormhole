//! What a Probe's hook decided, carried out (`wormhole_core::probe::active::Effect`): a notification, a
//! status item, a tint on a block, a file offered, a program started, a hook paused.
//!
//! Nothing here opens, runs or types anything by itself. A file is only *offered*, on a chip, and opened
//! when the user clicks it; a program starts on a thread of its own (`active::execute`: from the approved
//! copy, no shell, scrubbed) and tells the hook feed when it ended, which is where its output becomes
//! effects of its own.
//!
//! Part of the app root, beside `probes`.

use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui_kit::*;
use wormhole_core::probe::active::{self, Effect, RunEnd, RunJob, RunTicket};
use wormhole_core::probe::events::EventData;
use wormhole_core::probe::hidden::reveal;
use wormhole_core::probe::manifest::HookEvent;
use wormhole_core::probe::open_path::PathRef;
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::view::notice::{Notice, Warning};
use wormhole_text::keys;

use crate::app_state::AppState;
use crate::probe_runtime::{self, StatusItem};

/// What the hook feed's task hears: an event (several lines at once, for printed output) and the end of a
/// program a hook started.
pub(super) enum Feed {
    Event(HookEvent, Vec<EventData>),
    Ended(RunTicket, RunEnd),
}

/// Characters of a Probe's name or a file's name a chip says: both are text somebody else chose.
const SHOWN: usize = 60;

/// Numbers one chip from the next, so a new offer is not "the same one again".
static SERIAL: AtomicU32 = AtomicU32::new(0);

fn serial() -> u32 {
    SERIAL.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
}

fn shown(text: &str) -> String {
    reveal(text).chars().take(SHOWN).collect()
}

/// Do what `effects` say. `feed` is where a program started here reports its end.
pub(super) fn carry_out(effects: Vec<Effect>, feed: &smol::channel::Sender<Feed>, cx: &mut App) {
    for effect in effects {
        match effect {
            Effect::Notify { probe, probe_name, title, body, .. } => {
                crate::notifications::probe_notice(&probe, &probe_name, &title, &body, cx);
            }
            Effect::Status { probe, probe_name, item, text, tone, tooltip, .. } => {
                probe_runtime::set_status(StatusItem { probe, probe_name, item, text, tone, tooltip }, cx);
            }
            Effect::Highlight { tone, terminal, block, .. } => {
                if let Ok(block) = u64::try_from(block) {
                    probe_runtime::mark_block(terminal, block, tone, cx);
                }
            }
            Effect::OpenPath { probe_name, terminal, path, line, .. } => {
                offer_file(&probe_name, terminal, path, line, cx);
            }
            Effect::Run { ticket, probe, argv, cwd, timeout, env, terminal } => {
                let folder = terminal.and_then(|id| crate::window::terminal_folder(id, cx));
                start(ticket, RunJob { probe, argv, cwd, folder, timeout, env, terminal }, feed);
            }
            Effect::Paused { probe_name, hook } => {
                tracing::warn!(probe = %probe_name, hook, "a Probe's hook failed too often and was paused");
                let Some(front) = AppState::front_window(cx) else { continue };
                let words = keys::notice_warned_probe_paused(&shown(&probe_name));
                let notice = Notice::Warned(Warning::ProbePaused(serial()));
                crate::notice::post_to_words(front.handle, notice, words, None, cx);
            }
        }
    }
}

/// Put a chip in the terminal's window that offers `path`: a click on it looks for the file in that terminal's
/// folder and opens it. Never opened here: an event can fire with nobody there.
fn offer_file(probe_name: &str, terminal: i64, path: String, line: Option<u32>, cx: &mut App) {
    let Some(window) = crate::window::window_of(terminal, cx) else { return };
    let text = PathRef { path, line, column: None }.text();
    let words = keys::notice_warned_probe_offer(&shown(probe_name), &shown(&text));
    crate::notice::post_offer(
        window.handle,
        crate::notice::Offer {
            notice: Notice::Warned(Warning::ProbeOffer(serial())),
            words,
            act: Rc::new(move |_, cx| crate::window::open_offered_file(terminal, text.clone(), cx)),
            closed: Rc::new(|_| {}),
        },
        cx,
    );
}

/// Run `job` on a thread of its own and tell `feed` how it ended. A thread that cannot be had is an end too,
/// or the hook would wait for it for ever.
fn start(ticket: RunTicket, job: RunJob, feed: &smol::channel::Sender<Feed>) {
    let root = DataRoot::current();
    let report = feed.clone();
    let spawned = std::thread::Builder::new().name("wormhole-probe-run".into()).spawn(move || {
        let end = active::execute(&root, &job);
        // Blocking is fine here, and a closed feed (the Probes were loaded again) just means nobody waits.
        let _ = report.send_blocking(Feed::Ended(ticket, end));
    });
    if let Err(err) = spawned {
        let _ = feed.try_send(Feed::Ended(ticket, RunEnd::NotRun(err.to_string())));
    }
}
