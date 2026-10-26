//! What starts a motion. Both kinds count their starts in state kept for the icon, and the motion
//! plays under an id made of that count, so each start plays it anew and nothing plays at the
//! count 0, which is how an icon appears: a window opening or a panel coming back shows icons at
//! rest. The state lives as long as the icon is drawn, and goes with it.
//!
//! * A click ([`Clicks`]): the button's click handler counts it.
//! * A change ([`watch`]): the drawing sees a value differ from the one it saw in its last frame
//!   (the icon of a zoom button, whether a menu is open, whether a tab is the active one). That
//!   happens during a draw, so it never notifies: the frame it is in is the one that shows it.
//!
//! The Animate icons setting being off keeps the count at 0, so turning it on later doesn't play
//! what was missed.
//!
//! The state is found by the icon's id, so two icons with one id among the same siblings would share
//! it: a click on one would play both, and a `watch` seeing two values a frame would count a change
//! in every frame. A button's id must be its own among its siblings, as GPUI asks of any id.

use gpui_kit::*;

use crate::platform::Menu;

/// The keys of the state kept for an icon, within the icon's own id (which, pushed on the id stack
/// rather than joined to the key, costs no allocation: this runs for every button in every frame).
const KEY: &str = "icon-motion";
const MENU_KEY: &str = "menu-open";

/// The clicks one button has counted.
#[derive(Clone)]
pub(in crate::style) struct Clicks {
    count: Entity<u64>,
    enabled: bool,
}

impl Clicks {
    /// The clicks of the icon `scope`, counted in state of its own.
    pub(super) fn keyed(scope: &ElementId, enabled: bool, window: &mut Window, cx: &mut App) -> Self {
        let count = window.with_id(scope.clone(), |window| window.use_keyed_state(KEY, cx, |_, _| 0u64));
        if !enabled && *count.read(cx) != 0 {
            count.update(cx, |count, _| *count = 0);
        }
        Clicks { count, enabled }
    }

    /// How many clicks have started the motion, 0 when none has.
    pub(super) fn run(&self, cx: &App) -> u64 {
        *self.count.read(cx)
    }

    /// A click, from the button's handler. The entity is notified, so the view that draws the
    /// button draws it again, now with a new run.
    pub(in crate::style) fn start(&self, cx: &mut App) {
        if self.enabled {
            self.count.update(cx, |count, cx| {
                *count += 1;
                cx.notify();
            });
        }
    }
}

/// What a drawing saw last.
struct Seen<V> {
    value: V,
    before: V,
    run: u64,
}

/// A value's change, as seen by a drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Change<V> {
    /// How many changes have been seen, 0 when there has been none.
    pub run: u64,
    /// What the value was before its last change (itself when it has not changed).
    pub before: V,
}

/// `value` as drawn now against what it was in the last frame this was drawn in. The first frame
/// only takes note.
pub(super) fn watch<V: Copy + PartialEq + 'static>(
    scope: &ElementId,
    value: V,
    enabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> Change<V> {
    let seen = window.with_id(scope.clone(), |window| {
        window.use_keyed_state(KEY, cx, move |_, _| Seen { value, before: value, run: 0 })
    });
    // Read first: most frames see the value they saw last, and a zoom button's runs in every one.
    let now = seen.read(cx);
    if now.value == value && (enabled || now.run == 0) {
        return Change { run: now.run, before: now.before };
    }
    seen.update(cx, |seen, _| {
        if seen.value != value {
            (seen.before, seen.value) = (seen.value, value);
            seen.run += 1;
        }
        if !enabled {
            seen.run = 0;
        }
        Change { run: seen.run, before: seen.before }
    })
}

/// Whether the menu of a button is open, for the chevron beside Open in… that points up while it is.
/// The code that opens the menu hands it through [`MenuMark::track`], which marks it open until it
/// closes.
#[derive(Clone)]
pub struct MenuMark {
    open: Entity<bool>,
}

impl MenuMark {
    pub(in crate::style) fn keyed(scope: &ElementId, window: &mut Window, cx: &mut App) -> Self {
        MenuMark {
            open: window.with_id(scope.clone(), |window| window.use_keyed_state(MENU_KEY, cx, |_, _| false)),
        }
    }

    pub(in crate::style) fn is_open(&self, cx: &App) -> bool {
        *self.open.read(cx)
    }

    /// `menu`, marked open from now until it closes (an item chosen or not). A menu with nothing
    /// to show never opens, so it is not marked.
    pub fn track(&self, menu: Menu, cx: &mut App) -> Menu {
        if menu.is_empty() {
            return menu;
        }
        self.set(true, cx);
        let (mark, before) = (self.clone(), menu.on_close.clone());
        menu.on_close(move |window, cx| {
            mark.set(false, cx);
            if let Some(run) = &before {
                run(window, cx);
            }
        })
    }

    fn set(&self, open: bool, cx: &mut App) {
        self.open.update(cx, |state, cx| {
            if *state != open {
                *state = open;
                cx.notify();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui_kit::{AnyWindowHandle, Context, IntoElement, Render, TestAppContext, Window, div};

    use super::{Change, Clicks, MenuMark, watch};
    use crate::platform::Menu;

    /// A view that watches a value and keeps a mark for a menu, as a button does while it draws.
    struct Probe {
        now: Rc<Cell<(u8, bool)>>,
        seen: Rc<RefCell<Vec<Change<u8>>>>,
        mark: Rc<RefCell<Option<MenuMark>>>,
        clicks: Rc<RefCell<Option<Clicks>>>,
    }

    impl Render for Probe {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let (value, enabled) = self.now.get();
            let change = watch(&"watched".into(), value, enabled, window, cx);
            self.seen.borrow_mut().push(change);
            *self.mark.borrow_mut() = Some(MenuMark::keyed(&"menu-mark".into(), window, cx));
            *self.clicks.borrow_mut() = Some(Clicks::keyed(&"clicks".into(), enabled, window, cx));
            div()
        }
    }

    struct Harness {
        window: AnyWindowHandle,
        now: Rc<Cell<(u8, bool)>>,
        seen: Rc<RefCell<Vec<Change<u8>>>>,
        mark: Rc<RefCell<Option<MenuMark>>>,
        clicks: Rc<RefCell<Option<Clicks>>>,
    }

    impl Harness {
        fn open(cx: &mut TestAppContext) -> Self {
            let (seen, mark) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(None)));
            let clicks = Rc::new(RefCell::new(None));
            let now = Rc::new(Cell::new((1, true)));
            let window = {
                let (now, seen, mark, clicks) = (now.clone(), seen.clone(), mark.clone(), clicks.clone());
                cx.add_window(move |_, _| Probe { now, seen, mark, clicks })
            };
            Harness { window: window.into(), now, seen, mark, clicks }
        }

        /// Draw a frame with the probe's value and switch as given; what it saw in it.
        fn frame(&self, cx: &mut TestAppContext, value: u8, enabled: bool) -> Change<u8> {
            self.now.set((value, enabled));
            cx.update(|cx| {
                self.window
                    .update(cx, |_, window, cx| window.draw(cx).clear(cx))
                    .expect("the window is open");
            });
            *self.seen.borrow().last().expect("a frame was drawn")
        }
    }

    /// The first frame only notes the value; a different one in a later frame is a run, and the
    /// run goes back to 0 while the motion is off.
    #[gpui_kit::test]
    fn a_change_is_a_run_from_the_second_frame_on(cx: &mut TestAppContext) {
        let harness = Harness::open(cx);
        assert_eq!(harness.frame(cx, 1, true), Change { run: 0, before: 1 });
        assert_eq!(harness.frame(cx, 1, true), Change { run: 0, before: 1 });
        assert_eq!(harness.frame(cx, 2, true), Change { run: 1, before: 1 });
        assert_eq!(harness.frame(cx, 2, true), Change { run: 1, before: 1 });
        assert_eq!(harness.frame(cx, 1, true), Change { run: 2, before: 2 });
        assert_eq!(harness.frame(cx, 2, false).run, 0);
        // Turned on again, nothing that happened meanwhile plays.
        assert_eq!(harness.frame(cx, 2, true).run, 0);
    }

    /// Each click is a run of its own, and none counts while the motion is off.
    #[gpui_kit::test]
    fn each_click_is_a_run_and_none_counts_while_the_motion_is_off(cx: &mut TestAppContext) {
        let harness = Harness::open(cx);
        harness.frame(cx, 1, true);
        let clicks = harness.clicks.borrow().clone().expect("drawn");
        assert_eq!(cx.update(|cx| clicks.run(cx)), 0);
        cx.update(|cx| clicks.start(cx));
        cx.update(|cx| clicks.start(cx));
        harness.frame(cx, 1, true);
        assert_eq!(cx.update(|cx| clicks.run(cx)), 2);
        harness.frame(cx, 1, false);
        let off = harness.clicks.borrow().clone().expect("drawn");
        assert_eq!(cx.update(|cx| off.run(cx)), 0);
        cx.update(|cx| off.start(cx));
        assert_eq!(cx.update(|cx| off.run(cx)), 0);
    }

    #[gpui_kit::test]
    fn a_menu_is_open_until_it_closes_and_its_own_close_still_runs(cx: &mut TestAppContext) {
        let harness = Harness::open(cx);
        harness.frame(cx, 1, true);
        let mark = harness.mark.borrow().clone().expect("drawn");
        let closed = Rc::new(Cell::new(false));
        let menu = {
            let closed = closed.clone();
            Menu::new().header("Open in").on_close(move |_, _| closed.set(true))
        };
        let tracked = cx.update(|cx| mark.track(menu, cx));
        assert!(cx.update(|cx| mark.is_open(cx)));
        cx.update(|cx| {
            harness
                .window
                .update(cx, |_, window, cx| (tracked.on_close.as_ref().expect("a close"))(window, cx))
                .expect("the window is open");
        });
        assert!(!cx.update(|cx| mark.is_open(cx)));
        assert!(closed.get());
        // Nothing to open, nothing to close.
        let empty = cx.update(|cx| mark.track(Menu::new(), cx));
        assert!(empty.on_close.is_none());
        assert!(!cx.update(|cx| mark.is_open(cx)));
    }
}
