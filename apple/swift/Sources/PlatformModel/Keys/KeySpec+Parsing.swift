// Reading a key spec back: the glyph notation menus show, and the text grammar of config (`cmd-shift-r`,
// `cmd--`, `ctrl+alt+t`), which the hotkey setting and project action keys are stored in (`keyspec.rs`).

import Foundation

extension KeySpec {
    /// Apple's notation: modifier glyphs in any order, then the key (`⌥⌘1`, `⇧⌘]`, `⌃⌘↓`, `⇧F6`).
    public init?(glyphs text: String) {
        var mods: KeyMods = []
        var rest = Substring(text.trimmingCharacters(in: .whitespaces))
        while let first = rest.first, let found = KeyMods.table.first(where: { $0.glyph == first }) {
            guard !mods.contains(found.mods) else { return nil }
            mods.insert(found.mods)
            rest = rest.dropFirst()
        }
        guard !rest.isEmpty else { return nil }
        let name = Self.glyphs.first { $0.glyph == rest }?.name ?? rest.lowercased()
        let spec = KeySpec(mods, name)
        guard spec.hasKnownKey else { return nil }
        self = spec
    }

    /// A single character, a named key or F1…F20: anything else is a typo that would never match.
    public var hasKnownKey: Bool {
        key.count == 1 || isFunctionKey || Self.namedKeys.contains(key)
    }

    static let namedKeys: Set<String> = [
        "enter", "escape", "space", "tab", "backspace", "delete", "up", "down", "left", "right", "home", "end",
        "pageup", "pagedown",
    ]

    public enum ParseError: Error, Equatable {
        case empty, missingKey, unknownModifier(String), duplicateModifier(String)
    }

    private static let aliases = ["return": "enter", "esc": "escape", "backtick": "`", "grave": "`"]
    private static let modifierNames: [String: KeyMods] = [
        "cmd": .command, "command": .command, "super": .command, "meta": .command, "platform": .command,
        "⌘": .command, "ctrl": .control, "control": .control, "⌃": .control, "alt": .option, "opt": .option,
        "option": .option, "⌥": .option, "shift": .shift, "⇧": .shift,
    ]

    /// The text grammar: modifiers first, the key last, `-` or `+` between; a separator as the key follows one.
    public init(parsing text: String) throws {
        let spec = text.trimmingCharacters(in: .whitespaces)
        guard !spec.isEmpty else { throw ParseError.empty }
        let separators = Set<Character>("-+")
        var modsPart = Substring(), key = Substring(spec)
        if let last = spec.last, separators.contains(last), spec.count > 1,
            separators.contains(spec[spec.index(spec.endIndex, offsetBy: -2)])
        {
            (modsPart, key) = (spec.dropLast(2), spec.suffix(1))
        } else if let cut = spec.lastIndex(where: separators.contains), cut != spec.startIndex {
            (modsPart, key) = (spec[..<cut], spec[spec.index(after: cut)...])
        }
        guard !key.trimmingCharacters(in: .whitespaces).isEmpty else { throw ParseError.missingKey }
        var mods: KeyMods = []
        for token in modsPart.split(whereSeparator: separators.contains).map({ $0.trimmingCharacters(in: .whitespaces) }) {
            guard let flag = Self.modifierNames[token.lowercased()] else { throw ParseError.unknownModifier(token) }
            guard !mods.contains(flag) else { throw ParseError.duplicateModifier(token) }
            mods.insert(flag)
        }
        let name = key.trimmingCharacters(in: .whitespaces).lowercased()
        self.init(mods, Self.aliases[name] ?? name)
    }

    /// The canonical text form, `ctrl-alt-shift-cmd-<key>`, which `init(parsing:)` reads back.
    public var text: String {
        KeyMods.table.filter { mods.contains($0.mods) }.map { $0.name + "-" }.joined() + key
    }
}
