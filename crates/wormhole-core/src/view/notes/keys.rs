//! What the editor of a note or a next step does with a key, the same for both.

use crate::keys::keymap::ActionId;

/// A key an item's editor handles the same way for every kind of item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKey {
    /// Escape: back to the terminal. Leaving an item empty throws it away.
    Leave,
    /// Backspace in an empty editor: delete the item and carry on in the one above.
    DeleteEmpty,
    /// ⌥⌘↑ / ⌥⌘↓: move the item one place up (-1) or down (1).
    Move(isize),
}

/// What `key` does in an item's editor. `plain` is that no modifier is held; `empty` that the
/// editor holds nothing; `list_action` is asked only when the key is neither of the first two (the
/// keymap's answer for the key in a list: ⌥⌘↑ and ⌥⌘↓ move the item).
pub fn item_key(
    key: &str,
    plain: bool,
    empty: bool,
    list_action: impl FnOnce() -> Option<ActionId>,
) -> Option<ItemKey> {
    match key {
        "escape" if plain => Some(ItemKey::Leave),
        "backspace" if plain && empty => Some(ItemKey::DeleteEmpty),
        _ => match list_action() {
            Some(ActionId::MoveItemUp) => Some(ItemKey::Move(-1)),
            Some(ActionId::MoveItemDown) => Some(ItemKey::Move(1)),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{ItemKey, item_key};
    use crate::keys::keymap;
    use crate::keys::keyspec::KeySpec;

    fn key(text: &str, empty: bool) -> Option<ItemKey> {
        let spec: KeySpec = text.parse().expect("a key spec");
        let plain = spec.mods == Default::default();
        item_key(&spec.key, plain, empty, || keymap::list_action(&spec))
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
