//! What a window says about its terminals without knowing who listens: a terminal is in front of the
//! person (or gone), so whatever was said about it is stale; or a terminal's status changed and the
//! ones who watch statuses should look now. The notifications listen (`notifications::init`); the window
//! only tells, so it names nothing above itself.
//!
//! Told from inside a window's update, heard when that update ends, so a listener may read the windows.

use gpui_kit::*;

/// What a window tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Attention {
    /// Terminal `tab` is in front of the person, or was closed: its notification, if one is up, goes.
    Seen(i64),
    /// A terminal's status changed (an agent's hook): whoever watches statuses looks now.
    Changed,
}

/// The one place the events are emitted from.
struct Bus;

impl EventEmitter<Attention> for Bus {}

struct Channel(Entity<Bus>);

impl Global for Channel {}

fn channel(cx: &mut App) -> Entity<Bus> {
    if let Some(channel) = cx.try_global::<Channel>() {
        return channel.0.clone();
    }
    let bus = cx.new(|_| Bus);
    cx.set_global(Channel(bus.clone()));
    bus
}

/// Say `event`.
pub(crate) fn tell(event: Attention, cx: &mut App) {
    channel(cx).update(cx, |_, cx| cx.emit(event));
}

/// Hear every event from now on, for as long as the app runs.
pub(crate) fn listen(cx: &mut App, mut heard: impl FnMut(Attention, &mut App) + 'static) {
    let bus = channel(cx);
    cx.subscribe(&bus, move |_, event: &Attention, cx| heard(*event, cx)).detach();
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gpui_kit::TestAppContext;

    use super::{Attention, listen, tell};

    #[gpui_kit::test]
    fn what_a_window_tells_reaches_whoever_listens(cx: &mut TestAppContext) {
        let heard = Rc::new(RefCell::new(Vec::new()));
        let sink = heard.clone();
        cx.update(|cx| listen(cx, move |event, _| sink.borrow_mut().push(event)));
        cx.update(|cx| {
            tell(Attention::Seen(4), cx);
            tell(Attention::Changed, cx);
        });
        assert_eq!(*heard.borrow(), [Attention::Seen(4), Attention::Changed]);
    }

    #[gpui_kit::test]
    fn telling_with_nobody_listening_is_harmless(cx: &mut TestAppContext) {
        cx.update(|cx| tell(Attention::Changed, cx));
    }
}
