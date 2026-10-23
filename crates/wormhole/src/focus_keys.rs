//! The keys that move the keyboard focus and walk a list, which the sidebar and the window share: ⇥ and
//! ⇧⇥ between stops, the arrows and ↩ / Space inside a list, ⌃↩ for a row's menu, and whether the
//! window is the one in front as far as the focus ring goes. What the window does with them (F6 from
//! area to area, the Esc ladder, the ring) is `window::focus`; this holds only what needs no window.

use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::focus::list::Key;

use crate::actions;
use crate::style;

/// How an item is pressed: ↩ or Space.
pub(crate) use wormhole_core::view::focus::list::Press as Activate;

actions!(focus_area, [NextStop, PreviousStop]);

/// The key context of an area's container: where ⇥ / ⇧⇥ step from stop to stop.
pub(crate) const AREA_CONTEXT: &str = "FocusArea";

/// Bind ⇥ and ⇧⇥ inside the areas. They are the conventions for moving through a window, like
/// a text field's own keys, not shortcuts, so they aren't in core's key table. The terminal takes
/// its keys before any binding, so it keeps them; GPUI Kit's window root binds them to step through
/// its controls, and this deeper context wins over it.
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextStop, Some(AREA_CONTEXT)),
        KeyBinding::new("shift-tab", PreviousStop, Some(AREA_CONTEXT)),
    ]);
}

/// Whether `window` is the one in front, as far as the ring goes. devctl can say so in place of
/// macOS (`window-front`), to see the ring without taking the front from the app the person uses.
pub(crate) fn in_front(window: &Window) -> bool {
    #[cfg(feature = "devctl")]
    if let Some(front) = FRONT.get() {
        return front;
    }
    window.is_window_active()
}

#[cfg(feature = "devctl")]
thread_local! {
    static FRONT: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// devctl's `window-front on|off|system`.
#[cfg(feature = "devctl")]
pub(crate) fn set_front(front: Option<bool>) {
    FRONT.set(front);
}

/// The arrow keys a list walks with (⌥↑ / ⌥↓: the first and the last item).
pub(crate) fn list_key(keystroke: &Keystroke) -> Option<Key> {
    let m = &keystroke.modifiers;
    if m.control || m.platform || m.shift || m.function && !is_arrow(&keystroke.key) {
        return None;
    }
    Some(match (keystroke.key.as_str(), m.alt) {
        ("up", false) => Key::Up,
        ("down", false) => Key::Down,
        ("left", false) => Key::Left,
        ("right", false) => Key::Right,
        ("up", true) => Key::First,
        ("down", true) => Key::Last,
        _ => return None,
    })
}

/// macOS reports the arrows with the function modifier set.
fn is_arrow(key: &str) -> bool {
    matches!(key, "up" | "down" | "left" | "right")
}

/// Return and Space, with no modifier: what presses the item the keys are on.
pub(crate) fn activation(keystroke: &Keystroke) -> Option<Activate> {
    if keystroke.modifiers.modified() {
        return None;
    }
    match keystroke.key.as_str() {
        "enter" => Some(Activate::Return),
        "space" => Some(Activate::Space),
        _ => None,
    }
}

/// Whether `keystroke` is the row menu's key (⌃↩ in a list opens the menu of the row the keys are on,
/// the one a right-click shows, under that row as macOS's lists do under Full Keyboard Access).
pub(crate) fn is_menu_key(keystroke: &Keystroke) -> bool {
    keymap::list_action(&crate::terminal_view::keystroke_spec(keystroke)) == Some(ActionId::RowMenu)
}

/// Where a menu opened from the keys starts: under the row, in from its edge by the list's inset
/// (a flush row starts at the window's edge, where the menu's corners would be cut off), where a
/// right-click on its lead column would put it.
pub(crate) fn key_menu_origin(row: Bounds<Pixels>) -> Point<Pixels> {
    point(row.left() + px(style::SIDEBAR_INSET), row.bottom())
}

#[cfg(test)]
mod tests {
    use gpui_kit::Keystroke;

    use super::{Activate, activation, list_key};
    use wormhole_core::view::focus::list::Key;

    fn key(text: &str) -> Keystroke {
        Keystroke::parse(text).expect("a keystroke")
    }

    #[test]
    fn the_arrows_walk_and_option_jumps_to_the_ends() {
        assert_eq!(list_key(&key("up")), Some(Key::Up));
        assert_eq!(list_key(&key("right")), Some(Key::Right));
        assert_eq!(list_key(&key("alt-up")), Some(Key::First));
        assert_eq!(list_key(&key("alt-down")), Some(Key::Last));
        assert_eq!(list_key(&key("cmd-up")), None, "⌘↑ is an app key");
        assert_eq!(list_key(&key("shift-down")), None);
        assert_eq!(list_key(&key("a")), None);
    }

    #[test]
    fn return_and_space_press_without_modifiers() {
        assert_eq!(activation(&key("enter")), Some(Activate::Return));
        assert_eq!(activation(&key("space")), Some(Activate::Space));
        assert_eq!(activation(&key("cmd-enter")), None);
        assert_eq!(activation(&key("tab")), None);
    }
}
