//! A view GPUI keeps between frames.
//!
//! `.cached(style)` makes a frame that changes nothing in a view replay what it drew last time, which saves
//! building, laying out and painting it. GPUI also replays the view's accessibility nodes and their action
//! listeners (the patched `gpui-pre`, see `Cargo.toml`), so a view kept between frames stays in the tree
//! while an app has asked for it: the chrome's rows and buttons, the pane bar's clipboard Allow / Allow once
//! / Deny among them, and the content of a popover opened inside one (drawn through `deferred`). Every view of the window that is cached goes through [`view`].

use gpui_kit::*;

/// `view` laid out at `style`, replayed from the last frame while nothing in it changed.
pub(super) fn view<V: Render>(view: &Entity<V>, style: StyleRefinement) -> AnyElement {
    view.clone().cached(style).into_any_element()
}
