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
}
