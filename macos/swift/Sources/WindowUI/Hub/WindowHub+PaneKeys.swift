// The keys a pane's menu and header tooltips say: the keymap's now (`window/panes/mod.rs` `keymap::key(action)`), so a key the
// person changed is the one shown.

import PanesUI
import PlatformModel
import SidebarUI
import SwiftUI

extension WindowHub {
    /// The pane verbs with the keymap's keys in the menu's items and the header's tooltips.
    func paneActionsWithKeys(keys keymap: Keymap = .shared, appearance: SidebarAppearance) -> PaneActions {
        var actions = paneActionsWithTiles(appearance: appearance)
        actions.keyText = { keymap.key(ActionID(rawValue: $0))?.glyphText }
        actions.shortcut = { keymap.key(ActionID(rawValue: $0)).flatMap(Self.shortcut(for:)) }
        return actions
    }

    /// A key spec as SwiftUI's menu item binds it; nil for a key it can't hold.
    private static func shortcut(for spec: KeySpec) -> KeyboardShortcut? {
        let key: KeyEquivalent
        switch spec.key {
        case "enter": key = .return
        case "space": key = .space
        case "tab": key = .tab
        case "escape": key = .escape
        case "up": key = .upArrow
        case "down": key = .downArrow
        case "left": key = .leftArrow
        case "right": key = .rightArrow
        case "backspace": key = .delete
        case let other where other.count == 1: key = KeyEquivalent(Character(other))
        default: return nil
        }
        var modifiers: EventModifiers = []
        if spec.mods.contains(.command) { modifiers.insert(.command) }
        if spec.mods.contains(.option) { modifiers.insert(.option) }
        if spec.mods.contains(.control) { modifiers.insert(.control) }
        if spec.mods.contains(.shift) { modifiers.insert(.shift) }
        return KeyboardShortcut(key, modifiers: modifiers)
    }
}
