//! What the app tells those who watch it from the command line (`wh events`): the one way a window or
//! a terminal says that something happened, so that nothing needs to be built, formatted or
//! allocated for it while nobody listens.
//!
//! The bus itself is core's (`wormhole_core::ipc::bus`); this is the app's handle on it, a global
//! made at launch beside the hook socket's task. A global rather than a field of `AppState`, whose
//! fields are a ceiling that only drops. Callers ask [`wants`] before they build anything, or give
//! [`publish`] a closure that runs only when somebody listens: with the flag off, or with nobody
//! subscribed, that is one global lookup and one relaxed atomic load.
//!
//! What terminals print is told through the bus too, without the UI reading any of it: the window only
//! *pulses* (`printed`), and the bus's worker thread looks at the terminal's screen (core's `ipc::bus`).
//!
//! Low in the layers: it uses only core and GPUI, and nothing above it, so the terminal view and the
//! window can publish without knowing who listens. The app root (`app::control`) is the one that
//! makes subscriptions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui_kit::*;
use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::event::{Event, Topic};
use wormhole_term::ScreenReader;

/// The bus the socket's streams listen on, and whether the command line may read every terminal,
/// as the last tick saw it.
struct Automation {
    bus: Arc<Bus>,
    reads: AtomicBool,
}

impl Global for Automation {}

/// Keep `bus` as the one events are published on. Once, at launch.
pub(crate) fn install(bus: Arc<Bus>, cx: &mut App) {
    cx.set_global(Automation { bus, reads: AtomicBool::new(false) });
}

/// The bus, for whoever makes a subscription.
pub(crate) fn bus(cx: &App) -> Option<Arc<Bus>> {
    cx.try_global::<Automation>().map(|automation| automation.bus.clone())
}

/// Whether somebody listens for `topic`. Ask before building what would be published.
pub(crate) fn wants(cx: &App, topic: Topic) -> bool {
    cx.try_global::<Automation>().is_some_and(|automation| automation.bus.wants(topic))
}

/// Whether somebody wants to hear that terminals print: a stream of `output`, or a `wait` that looks at
/// a screen. Ask before emitting anything about a print.
pub(crate) fn wants_prints(cx: &App) -> bool {
    cx.try_global::<Automation>().is_some_and(|automation| automation.bus.wants_prints())
}

/// Terminal `terminal` of `window` printed: note it for the output worker if a stream asked for what
/// it prints and may read it. `reader` makes the terminal's handle and is called only when the note
/// is new; with nobody asking it is a global lookup and an atomic load.
pub(crate) fn printed(cx: &App, window: i64, terminal: i64, reader: impl FnOnce() -> Option<ScreenReader>) {
    if let Some(automation) = cx.try_global::<Automation>() {
        automation.bus.pulse(window, terminal, || reader().map(|reader| reader.downgrade()));
    }
}

/// Whether the command line may read every terminal changed: when it turned off, the streams that
/// needed it are closed (a terminal's own caller keeps what it reads of itself). Once a tick.
pub(crate) fn sync_grants(cx: &App, reads_allowed: bool) {
    if let Some(automation) = cx.try_global::<Automation>()
        && automation.reads.swap(reads_allowed, Ordering::Relaxed)
        && !reads_allowed
    {
        automation.bus.revoke_reads();
    }
}

/// Tell the listeners of `topic` what `build` makes; `build` does not run when nobody listens.
pub(crate) fn publish(cx: &App, topic: Topic, build: impl FnOnce() -> Event) {
    if let Some(automation) = cx.try_global::<Automation>() {
        automation.bus.publish(topic, build);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui_kit::TestAppContext;
    use wormhole_core::ipc::bus::{Bus, Grant, Next};
    use wormhole_core::ipc::control::event::{Body, Event, EventsSpec, Topic};

    use super::{install, publish, wants};

    fn bell() -> Event {
        Event { window: Some(1), terminal: Some(2), body: Body::Bell }
    }

    #[gpui_kit::test]
    fn nothing_is_built_until_somebody_listens(cx: &mut TestAppContext) {
        let built = Cell::new(0);
        let count = |cx: &gpui_kit::App| {
            publish(cx, Topic::Bell, || {
                built.set(built.get() + 1);
                bell()
            });
        };
        // Before the bus is installed (a test of something else, a window with no app).
        cx.update(|cx| {
            assert!(!wants(cx, Topic::Bell));
            count(cx);
        });
        let bus = Bus::new();
        cx.update(|cx| install(bus.clone(), cx));
        cx.update(|cx| {
            assert!(!wants(cx, Topic::Bell));
            count(cx);
        });
        assert_eq!(built.get(), 0, "with nobody subscribed the closure never runs");
        let filter =
            EventsSpec { topics: vec!["bell".into()], ..EventsSpec::default() }.filter().expect("valid");
        let (subscription, _) = bus.subscribe(filter, Grant::default(), None).expect("room");
        cx.update(|cx| {
            assert!(wants(cx, Topic::Bell) && !wants(cx, Topic::Status));
            count(cx);
        });
        assert_eq!(built.get(), 1);
        assert!(matches!(subscription.next(std::time::Duration::ZERO), Next::Event(_)));
        drop(subscription);
        cx.update(|cx| assert!(!wants(cx, Topic::Bell), "the last subscription gone, the interest is too"));
    }
}
