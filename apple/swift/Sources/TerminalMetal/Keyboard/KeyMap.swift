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
        98: "f7", 100: "f8", 101: "f9", 109: "f10", 103: "f11", 111: "f12", 105: "f13", 107: "f14",
        113: "f15", 106: "f16", 64: "f17", 79: "f18", 80: "f19", 90: "f20",
    ]

    /// The device-dependent bits for the left and right Option keys (`NX_DEVICELALTKEYMASK`, `NX_DEVICERALTKEYMASK`).
    private static let leftOption: UInt = 0x20
    private static let rightOption: UInt = 0x40

    static func input(from event: NSEvent, optionAsMeta mode: OptionAsMeta) -> KeyInput? {
        let flags = event.modifierFlags
        let name: String
        if let special = named[event.keyCode] {
            name = special
        } else if let base = event.charactersIgnoringModifiers?.lowercased(), base.unicodeScalars.count == 1 {
            name = base
        } else {
            return nil
        }
        let meta = mode.isMeta(left: flags.rawValue & leftOption != 0, right: flags.rawValue & rightOption != 0)
        // Keys that type nothing carry no text (AppKit gives arrows a private-use character), and neither does
        // Control: the engine encodes those from the key's name, as the contract asks.
        let text = flags.contains(.control) || (named.values.contains(name) && name != "space") ? nil : event.characters
        return KeyInput(
            key: name, text: text, shift: flags.contains(.shift), alt: flags.contains(.option),
            control: flags.contains(.control), platform: flags.contains(.command), optionAsMeta: meta)
    }

    /// Keys that type no text: they never go through the input method unless it is composing.
    static func isNonText(_ key: KeyInput) -> Bool {
        named.values.contains(key.key) && key.key != "space"
    }
}

/// Which Option key sends ESC before the key (the setting `option_as_meta`); the other types the layout's
/// characters (Option-E for an accent).
public enum OptionAsMeta: Sendable, Equatable {
    case off, left, right, both

    /// The setting's name (`option_as_meta`: off, left, right, both).
    public init?(setting name: String) {
        switch name {
        case "off": self = .off
        case "left": self = .left
        case "right": self = .right
        case "both": self = .both
        default: return nil
        }
    }

    /// The rule of core's `OptionAsMeta::is_meta`: when the side cannot be told (a synthetic event) any
    /// setting but off counts.
    func isMeta(left: Bool, right: Bool) -> Bool {
        switch self {
        case .off: false
        case .both: true
        case _ where !left && !right: true
        case .left: left
        case .right: right
        }
    }
}
