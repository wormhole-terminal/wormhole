// A key chord as the keymap writes it: Apple's notation, modifiers in ⌃⌥⇧⌘ order (`keys/keyspec.rs` `glyphs`).

import Foundation

public struct Shortcut: Sendable, Equatable {
    public struct Modifiers: OptionSet, Sendable {
        public let rawValue: Int
        public init(rawValue: Int) { self.rawValue = rawValue }
        public static let control = Modifiers(rawValue: 1)
        public static let option = Modifiers(rawValue: 2)
        public static let shift = Modifiers(rawValue: 4)
        public static let command = Modifiers(rawValue: 8)
    }

    private static let glyphOrder: [(Modifiers, Character)] = [
        (.control, "⌃"), (.option, "⌥"), (.shift, "⇧"), (.command, "⌘"),
    ]

    public var modifiers: Modifiers
    /// What follows the modifiers: `T`, `]`, `↓`, `/`.
    public var key: String

    public init(_ modifiers: Modifiers, _ key: String) {
        self.modifiers = modifiers
        self.key = key
    }

    /// `⇧⌘P`: the text of the one key cap, and what the sheet's filter matches ("⌘W" finds Close terminal).
    public var glyphs: String {
        String(Self.glyphOrder.filter { modifiers.contains($0.0) }.map(\.1)) + key
    }

    /// The chord `text` writes, in any order of modifiers; nil without a key or with one modifier twice.
    public init?(glyphs text: String) {
        var modifiers: Modifiers = []
        var rest = Substring(text)
        while let first = rest.first, let found = Self.glyphOrder.first(where: { $0.1 == first }) {
            guard !modifiers.contains(found.0) else { return nil }
            modifiers.insert(found.0)
            rest = rest.dropFirst()
        }
        guard !rest.isEmpty, !modifiers.isEmpty || rest.count > 1 else { return nil }
        self.init(modifiers, rest.uppercased())
    }
}
