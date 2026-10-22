//! What tasks and notes share as items of the Notes tab: the keys their editors handle
//! and the pieces of their rows. Both are drawn with `style::item_row`.
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};

use super::Workspace;

/// A key an item's editor handles the same way for every kind of item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ItemKey {
    /// Escape: back to the terminal. Leaving an item empty throws it away.
    Leave,
    /// Backspace in an empty editor: delete the item and carry on in the one above.
    DeleteEmpty,
    /// ⌥⌘↑ / ⌥⌘↓: move the item one place up (-1) or down (1).
    Move(isize),
}

pub(super) fn item_key(keystroke: &Keystroke, empty: bool) -> Option<ItemKey> {
    let m = &keystroke.modifiers;
    let plain = !m.modified();
    match keystroke.key.as_str() {
        "escape" if plain => Some(ItemKey::Leave),
        "backspace" if plain && empty => Some(ItemKey::DeleteEmpty),
        // ⌥⌘↑ / ⌥⌘↓, as the keymap has them.
        _ => match keymap::list_action(&crate::terminal_view::keystroke_spec(keystroke)) {
            Some(ActionId::MoveItemUp) => Some(ItemKey::Move(-1)),
            Some(ActionId::MoveItemDown) => Some(ItemKey::Move(1)),
            _ => None,
        },
    }
}

impl Workspace {
    pub(super) fn focus_terminal(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.active_terminal().focus_handle(cx), cx);
    }

    /// The list key of this window's tasks or notes, so a dragged item only drops among its
    /// own kind in its own window.
    pub(super) fn reorder_list(&self, kind: &str, cx: &Context<Self>) -> SharedString {
        format!("{kind}-{}", cx.entity_id()).into()
    }
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

#[cfg(test)]
mod tests {
    use super::{ItemKey, item_key};
    use gpui_kit::Keystroke;

    fn key(text: &str, empty: bool) -> Option<ItemKey> {
        item_key(&Keystroke::parse(text).expect("keystroke"), empty)
    }

    #[test]
    fn editors_share_their_keys() {
        assert_eq!(key("escape", false), Some(ItemKey::Leave));
        assert_eq!(key("backspace", true), Some(ItemKey::DeleteEmpty));
        assert_eq!(key("backspace", false), None, "only an empty editor deletes its item");
        assert_eq!(key("alt-cmd-up", false), Some(ItemKey::Move(-1)));
        assert_eq!(key("alt-cmd-down", true), Some(ItemKey::Move(1)));
        assert_eq!(key("cmd-up", false), None);
        assert_eq!(key("shift-escape", false), None);
    }
}
