// The xterm encoding of a key press, for sources that do not encode keys themselves (the mock). The
// real engine's `send_key` knows the modes the program set (application cursor keys, the kitty keyboard
// protocol); this one assumes the defaults, which is all the mock's echo needs.

import Foundation

public enum KeyEncoder {
    public static func encode(_ input: KeyInput) -> Data? {
        // Command is the app's (menu shortcuts), never the program's.
        if input.platform { return nil }
        let modifier = 1 + (input.shift ? 1 : 0) + (input.alt ? 2 : 0) + (input.control ? 4 : 0)

        if let sequence = special(input.key, modifier: modifier) { return Data(sequence.utf8) }

        switch input.key {
        case "enter": return prefixed(input, "\r")
        case "tab": return input.shift ? Data("\u{1B}[Z".utf8) : prefixed(input, "\t")
        case "escape": return prefixed(input, "\u{1B}")
        case "backspace": return prefixed(input, input.control ? "\u{08}" : "\u{7F}")
        case "space" where input.control: return Data([0])
        default: break
        }

        // The key's own character: a control combination is a byte, option-as-meta is an ESC prefix on the
        // base character, anything else is the text the layout produced.
        let base = input.key == "space" ? " " : input.key
        if input.control, let byte = controlByte(for: base) { return prefixed(input, String(UnicodeScalar(byte))) }
        if input.alt, input.optionAsMeta, base.unicodeScalars.count == 1 {
            return Data(("\u{1B}" + (input.shift ? base.uppercased() : base)).utf8)
        }
        if let text = input.text, !text.isEmpty { return Data(text.utf8) }
        return nil
    }

    /// `ESC` before `text` when option acts as meta.
    private static func prefixed(_ input: KeyInput, _ text: String) -> Data {
        Data(((input.alt && input.optionAsMeta ? "\u{1B}" : "") + text).utf8)
    }

    private static func controlByte(for key: String) -> UInt8? {
        guard let scalar = key.unicodeScalars.first, key.unicodeScalars.count == 1 else { return nil }
        switch scalar.value {
        case 0x61...0x7A: return UInt8(scalar.value - 0x60)
        case 0x40...0x5F: return UInt8(scalar.value - 0x40)
        case 0x20: return 0
        case 0x32: return 0
        case 0x33...0x37: return UInt8(scalar.value - 0x33 + 0x1B)
        default: return nil
        }
    }

    private static func special(_ key: String, modifier: Int) -> String? {
        let cursor: [String: String] = ["up": "A", "down": "B", "right": "C", "left": "D", "home": "H", "end": "F"]
        if let letter = cursor[key] {
            return modifier == 1 ? "\u{1B}[\(letter)" : "\u{1B}[1;\(modifier)\(letter)"
        }
        let tilde: [String: Int] = [
            "insert": 2, "delete": 3, "pageup": 5, "pagedown": 6, "f5": 15, "f6": 17, "f7": 18, "f8": 19, "f9": 20,
            "f10": 21, "f11": 23, "f12": 24,
        ]
        if let code = tilde[key] {
            return modifier == 1 ? "\u{1B}[\(code)~" : "\u{1B}[\(code);\(modifier)~"
        }
        let ss3: [String: String] = ["f1": "P", "f2": "Q", "f3": "R", "f4": "S"]
        if let letter = ss3[key] {
            return modifier == 1 ? "\u{1B}O\(letter)" : "\u{1B}[1;\(modifier)\(letter)"
        }
        return nil
    }
}
