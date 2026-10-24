//! A view GPUI keeps between frames, except while the accessibility tree is built.
//!
//! `.cached(style)` makes a frame that changes nothing in a view replay what it drew last time, which saves
//! building, laying out and painting it. GPUI fills the accessibility tree during every frame's prepaint and
//! replays none of a reused view's nodes, so a view replayed while the tree is built drops out of it: the
//! chrome's rows and buttons, the pane bar's clipboard Allow / Allow once / Deny among them. While the tree is
//! built (`platform::a11y::tree_built`, which on macOS stays so for the window's life once any app has asked
//! for it) a view is therefore a plain child in a box of the same style, built in every frame like the rest
//! of the window. Every view of the window that is cached goes through [`view`].

use gpui_kit::*;

/// `view` laid out at `style`: replayed from the last frame while nothing in it changed, or, while the
/// accessibility tree is built, a plain child of a box with that style.
pub(super) fn view<V: Render>(view: &Entity<V>, style: StyleRefinement, window: &Window) -> AnyElement {
    if crate::platform::a11y::tree_built(window) {
        let mut shell = div();
        *shell.style() = style;
        shell.child(view.clone()).into_any_element()
    } else {
        view.clone().cached(style).into_any_element()
    }
}
