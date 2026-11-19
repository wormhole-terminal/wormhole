// A shortcut as `config.json` spells it (`ctrl-alt-shift-cmd-key`, `KeySpec`'s Display) and as it is shown
// (⌃⌥⇧⌘ and the key). Pure text, so the recorder's result and the key caps are tested without a window.

import AppKit

enum KeySpecText {
    private static let glyphs: [(name: String, glyph: String)] = [("ctrl", "⌃"), ("alt", "⌥"), ("shift", "⇧"), ("cmd", "⌘")]
    private static let namedKeys: [UInt16: String] = [
        36: "enter", 53: "escape", 49: "space", 48: "tab", 51: "backspace", 117: "delete", 126: "up", 125: "down",
        123: "left", 124: "right", 115: "home", 119: "end", 116: "pageup", 121: "pagedown",
        122: "f1", 120: "f2", 99: "f3", 118: "f4", 96: "f5", 97: "f6", 98: "f7", 100: "f8", 101: "f9", 109: "f10",
        103: "f11", 111: "f12",
    ]
    private static let keyGlyphs = [
        "enter": "↩", "escape": "⎋", "space": "Space", "tab": "⇥", "backspace": "⌫", "delete": "⌦", "up": "↑", // text: the key's name as a key cap reads
        "down": "↓", "left": "←", "right": "→", "home": "↖", "end": "↘", "pageup": "⇞", "pagedown": "⇟",
    ]

    /// The spec a key event records, nil for a modifier alone.
    static func spec(keyCode: UInt16, characters: String?, modifiers: NSEvent.ModifierFlags) -> String? {
        let key = namedKeys[keyCode] ?? characters?.lowercased().first.map(String.init)
        guard let key, !key.isEmpty else { return nil }
        let flags = modifiers.intersection(.deviceIndependentFlagsMask)
        let held: [(Bool, String)] = [
            (flags.contains(.control), "ctrl"), (flags.contains(.option), "alt"), (flags.contains(.shift), "shift"),
            (flags.contains(.command), "cmd"),
        ]
        return (held.filter(\.0).map(\.1) + [key]).joined(separator: "-")
    }

    /// The parts of a spec as key caps: ["⌥", "⌘", "J"].
    static func caps(_ spec: String) -> [String] {
        guard !spec.isEmpty else { return [] }
        var parts = spec.split(separator: "-", omittingEmptySubsequences: false).map(String.init)
        // A literal minus is the last part: "cmd--" splits to ["cmd", "", ""].
        if spec.hasSuffix("--") { parts = Array(parts.dropLast(2)) + ["-"] }
        return parts.map { part in
            glyphs.first { $0.name == part }?.glyph ?? keyGlyphs[part] ?? part.uppercased()
        }
    }

    /// Whether the shortcut can work as a global or in-app shortcut: a modifier besides Shift, or an F-key
    /// (the Rust `hotkey_accepts` / `jump_accepts` rules; the FFI's validator replaces this).
    static func isUsable(_ spec: String) -> Bool {
        let parts = spec.split(separator: "-").map(String.init)
        guard let key = parts.last else { return false }
        let mods = Set(parts.dropLast())
        return !mods.isDisjoint(with: ["ctrl", "alt", "cmd"]) || (key.hasPrefix("f") && Int(key.dropFirst()) != nil)
    }
}
