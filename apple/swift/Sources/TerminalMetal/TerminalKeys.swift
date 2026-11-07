// NSEvent to the app's key vocabulary ("enter", "left", "f5", "a"), which is what `KeyInput` and the
// engine's `send_key` take. The names of keys that type nothing come from the virtual key code; every
// other key is named by the character it has with no modifiers but shift.

import AppKit
import TerminalCore

enum KeyMap {
    /// US-layout virtual key codes (`kVK_*`) of the keys that are not characters.
    private static let named: [UInt16: String] = [
        36: "enter", 76: "enter", 48: "tab", 49: "space", 51: "backspace", 117: "delete", 53: "escape",
        123: "left", 124: "right", 125: "down", 126: "up", 115: "home", 119: "end", 116: "pageup",
        121: "pagedown", 114: "insert", 122: "f1", 120: "f2", 99: "f3", 118: "f4", 96: "f5", 97: "f6",
        98: "f7", 100: "f8", 101: "f9", 109: "f10", 103: "f11", 111: "f12",
    ]

    /// The device-dependent bit for the left Option key (`NX_DEVICELALTKEYMASK`): option acts as meta only
    /// on the left one, so the right one still types the layout's characters.
    private static let leftOption: UInt = 0x20

    static func input(from event: NSEvent, leftOptionAsMeta: Bool) -> KeyInput? {
        let flags = event.modifierFlags
        let name: String
        if let special = named[event.keyCode] {
            name = special
        } else if let base = event.charactersIgnoringModifiers?.lowercased(), base.unicodeScalars.count == 1 {
            name = base
        } else {
            return nil
        }
        let meta = leftOptionAsMeta && flags.rawValue & leftOption != 0
        return KeyInput(
            key: name, text: event.characters, shift: flags.contains(.shift), alt: flags.contains(.option),
            control: flags.contains(.control), platform: flags.contains(.command), optionAsMeta: meta)
    }

    /// Keys that type no text: they never go through the input method unless it is composing.
    static func isNonText(_ key: KeyInput) -> Bool {
        named.values.contains(key.key) && key.key != "space"
    }
}
