//! Where a menu opens from: under a button (GPUI knows an element's bounds only while it paints, so a
//! probe laid over the button remembers them for the click that comes later), or at a right-click
//! on a part of the window that has buttons of its own.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::*;

/// The bounds a probe last painted.
#[derive(Clone, Default)]
pub struct Anchor(Rc<Cell<Bounds<Pixels>>>);

impl Anchor {
    /// An element to put in the button's wrapper (which must be `relative`): it fills it, draws
    /// nothing and takes no mouse events.
    pub fn probe(&self) -> AnyElement {
        let bounds = self.0.clone();
        canvas(move |at, _, _| bounds.set(at), |_, _, _, _| {})
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element()
    }

    /// The element's bounds as last painted: where a popover's arrow points.
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.0.get()
    }

    /// The button's bottom left corner, where its menu starts.
    pub fn below(&self) -> Point<Pixels> {
        self.0.get().bottom_left()
    }
}

/// An element to put last in a `relative` wrapper: a right-click anywhere over the wrapper calls
/// `open` with the pointer's place, even over a button inside it. (Setting `on_mouse_down` on the
/// wrapper wouldn't do: a button that blocks the pointer hides everything behind it, the wrapper
/// included, from the hit test; this hitbox lies over the button and blocks nothing.)
pub fn on_right_click(open: impl Fn(Point<Pixels>, &mut Window, &mut App) + 'static) -> AnyElement {
    let open = Rc::new(open);
    canvas(
        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
        move |_, hitbox, window, _| {
            let open = open.clone();
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Right && hitbox.is_hovered(window) {
                    cx.stop_propagation();
                    open(event.position, window, cx);
                }
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}
