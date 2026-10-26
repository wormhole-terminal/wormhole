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
//! Low in the layers: it uses only core and GPUI, and nothing above it, so the terminal view and the
//! window can publish without knowing who listens. The app root (`app::control`) is the one that
//! makes subscriptions.

use std::sync::Arc;

use gpui_kit::*;
use wormhole_core::ipc::bus::Bus;
use wormhole_core::ipc::control::event::{Event, Topic};

/// The bus the socket's streams listen on.
struct Automation(Arc<Bus>);

impl Global for Automation {}

/// Keep `bus` as the one events are published on. Once, at launch.
pub(crate) fn install(bus: Arc<Bus>, cx: &mut App) {
    cx.set_global(Automation(bus));
}

/// The bus, for whoever makes a subscription.
pub(crate) fn bus(cx: &App) -> Option<Arc<Bus>> {
    cx.try_global::<Automation>().map(|automation| automation.0.clone())
}

/// Whether somebody listens for `topic`. Ask before building what would be published.
pub(crate) fn wants(cx: &App, topic: Topic) -> bool {
    cx.try_global::<Automation>().is_some_and(|automation| automation.0.wants(topic))
}

/// Tell the listeners of `topic` what `build` makes; `build` does not run when nobody listens.
pub(crate) fn publish(cx: &App, topic: Topic, build: impl FnOnce() -> Event) {
    if let Some(automation) = cx.try_global::<Automation>() {
        automation.0.publish(topic, build);
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
