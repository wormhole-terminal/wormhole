//! What tasks and notes share as items of the Notes tab: the keys their editors handle and the
//! editor's wrapper. Both are drawn with `style::item_row`. What a key means is core's
//! (`view::notes::keys`); this reads it off GPUI's keystroke.
use gpui_kit::*;
use wormhole_core::keys::keymap;
use wormhole_core::view::notes::keys::{self, ItemKey};

/// What the keystroke does in an item's editor (`empty`: it holds nothing).
pub(super) fn item_key(keystroke: &Keystroke, empty: bool) -> Option<ItemKey> {
    keys::item_key(&keystroke.key, !keystroke.modifiers.modified(), empty, || {
        // ⌥⌘↑ / ⌥⌘↓, as the keymap has them.
        keymap::list_action(&crate::terminal_view::keystroke_spec(keystroke))
    })
}

/// An item's editor with the keys every item handles ([`ItemKey`]); `on_key` gets the
/// event first and returns whether it handled it.
pub(super) fn with_item_keys(
    editor: impl IntoElement,
    on_key: impl Fn(&KeyDownEvent, &mut Window, &mut App) -> bool + 'static,
) -> Div {
    div()
        .capture_key_down(move |event, window, cx| {
            if on_key(event, window, cx) {
                cx.stop_propagation();
            }
        })
        .child(editor)
}
