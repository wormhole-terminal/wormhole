// A key and the modifiers held with it, as the core keymap writes it (`keys/keyspec.rs`): the key is lowercase
// (`t`, `]`, `down`, `f6`), and a US keyboard's shifted character is the same key as ⇧ and its base.

import Foundation

public struct KeySpec: Hashable, Sendable, Codable {
    public var mods: KeyMods
    public var key: String

    public init(_ mods: KeyMods, _ key: String) {
        self.mods = mods
        self.key = key.lowercased()
    }

    /// The glyph Apple writes for each named key.
    static let glyphs: [(name: String, glyph: String)] = [
        ("enter", "↩"), ("tab", "⇥"), ("escape", "⎋"), ("backspace", "⌫"), ("delete", "⌦"),
        ("up", "↑"), ("down", "↓"), ("left", "←"), ("right", "→"),
    ]

    /// What a US keyboard reports for ⇧ and the key: `⇧]` arrives as `}`.
    static let usShifted: [(base: String, shifted: String)] = [
        ("`", "~"), ("1", "!"), ("2", "@"), ("3", "#"), ("4", "$"), ("5", "%"), ("6", "^"), ("7", "&"), ("8", "*"),
        ("9", "("), ("0", ")"), ("-", "_"), ("=", "+"), ("[", "{"), ("]", "}"), ("\\", "|"), (";", ":"),
        ("'", "\""), (",", "<"), (".", ">"), ("/", "?"),
    ]

    public var isFunctionKey: Bool {
        guard key.hasPrefix("f"), let n = Int(key.dropFirst()) else { return false }
        return (1...20).contains(n)
    }

    /// Apple's notation, modifiers in ⌃⌥⇧⌘ order: `⇧⌘R`, `⌥Space`, `⌘↩`, `F6`.
    public var glyphText: String {
        let mods = String(KeyMods.table.filter { self.mods.contains($0.mods) }.map(\.glyph))
        if key.count == 1 { return mods + key.uppercased() }
        if let named = Self.glyphs.first(where: { $0.name == key }) { return mods + named.glyph }
        return mods + key.prefix(1).uppercased() + key.dropFirst()
    }

    /// With a shifted character written as ⇧ and its base: `⌘}` is `⇧⌘]`.
    public var canonical: KeySpec {
        guard let match = Self.usShifted.first(where: { $0.shifted == key }) else { return self }
        return KeySpec(mods.union(.shift), match.base)
    }

    /// Whether two specs name the same key press, written either way.
    public func sameKey(_ other: KeySpec) -> Bool { canonical == other.canonical }
}
