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

    public var modifiers: Modifiers
    /// What the key cap shows: `T`, `]`, `↓`, `/`.
    public var key: String

    public init(_ modifiers: Modifiers, _ key: String) {
        self.modifiers = modifiers
        self.key = key
    }

    /// One cap per modifier and one for the key, so the views can draw each on its own.
    public var parts: [String] {
        let glyphs: [(Modifiers, String)] = [(.control, "⌃"), (.option, "⌥"), (.shift, "⇧"), (.command, "⌘")]
        return glyphs.filter { modifiers.contains($0.0) }.map(\.1) + [key]
    }

    /// `⇧⌘P`: what the sheet's filter matches, so "⌘W" finds Close terminal.
    public var glyphs: String { parts.joined() }
}
