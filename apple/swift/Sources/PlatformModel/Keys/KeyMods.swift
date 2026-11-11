// The four modifiers of a key spec (`keys/keyspec.rs` `KeyMods`), written as the keymap's JSON names them.

import Foundation

public struct KeyMods: OptionSet, Hashable, Sendable, Codable {
    public let rawValue: Int
    public init(rawValue: Int) { self.rawValue = rawValue }

    public static let control = KeyMods(rawValue: 1)
    public static let option = KeyMods(rawValue: 2)
    public static let shift = KeyMods(rawValue: 4)
    public static let command = KeyMods(rawValue: 8)

    /// Apple's order, with the glyph each is written as and the name the JSON and the text grammar use.
    static let table: [(mods: KeyMods, glyph: Character, name: String)] = [
        (.control, "⌃", "ctrl"), (.option, "⌥", "alt"), (.shift, "⇧", "shift"), (.command, "⌘", "cmd"),
    ]

    public init(from decoder: any Decoder) throws {
        let names = try decoder.singleValueContainer().decode([String].self)
        self = KeyMods(names.compactMap { name in Self.table.first { $0.name == name }?.mods })
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(Self.table.filter { contains($0.mods) }.map(\.name))
    }
}
