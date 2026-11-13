// The notes' text rules, which core owns (`view::notes::paragraphs`): the paragraph ⌘↵ sends and a note's title. The app
// installs core's at launch (`FilesBridge.installRules()`); until then, in previews and tests, a plain stand-in
// answers: the whole text, and the first line.

public enum NoteRules {
    // why: written once on the main thread at launch, before any window reads them; every later access is a read.
    nonisolated(unsafe) public static var paragraphAt: @Sendable (_ text: String, _ cursorUTF8: Int) -> String = { text, _ in text }
    /// What closing a terminal says of its open notes and tasks ("notes and tasks are discarded."), nil when none are open.
    nonisolated(unsafe) public static var closeStake: @Sendable (_ openItems: Int) -> String? = { _ in nil }
    nonisolated(unsafe) public static var title: @Sendable (_ text: String) -> String? = { text in
        text.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.first { !$0.isEmpty }
    }
    /// What an open note shows under its title (`paragraphs::note_body`); nil for a note that is its title alone.
    nonisolated(unsafe) public static var body: @Sendable (_ text: String) -> String? = { text in
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false).drop(while: { $0.allSatisfy(\.isWhitespace) })
        let body = lines.dropFirst().joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
        return body.isEmpty ? nil : body
    }
    /// How many of an open note's wrapped lines show (`view::panel::note_cap`); nil for all of them.
    nonisolated(unsafe) public static var cap: @Sendable (_ lines: Int, _ editing: Bool, _ showAll: Bool) -> Int? = { lines, editing, showAll in
        !editing && !showAll && lines > 12 ? 12 : nil
    }
    /// Where a key moves the item being written, -1 up and 1 down (`view::notes::keys::item_key`, the keymap's
    /// ⌥⌘↑ and ⌥⌘↓ unless the person changed them); nil for every other key. `mods` are `KeyMods`' bits (⌃ 1, ⌥ 2,
    /// ⇧ 4, ⌘ 8), `key` the core's name of the key (`up`, `down`, a letter).
    nonisolated(unsafe) public static var moveBy: @Sendable (_ key: String, _ mods: Int) -> Int? = { key, mods in
        guard mods == 10 else { return nil }
        return key == "up" ? -1 : key == "down" ? 1 : nil
    }
}
