import DesignSystem
import PanelModel
import XCTest

@testable import PanelUI

/// Behaviour the GPUI app has and the panel gained in round S: what a diff line shows for characters a person cannot
/// see, git's problems in words, and the writing flows of the Notes tab.
@MainActor
final class PanelParityTests: XCTestCase {
    /// A repository's line is hostile input: whatever the rule answers, and wherever a highlighter says a span runs, the drawn
    /// text is coloured inside itself (the offsets are scalars, clamped), so opening a diff can never trap.
    func testADrawnLineNeverPutsAnOffsetPastItsEnd() {
        let saved = VisibleRules.line
        defer { VisibleRules.line = saved }
        // A control picture followed by a combining mark (one Character in the drawn text, two in the original), with offsets
        // and marks a stale or hostile answer could hold.
        VisibleRules.line = { _, _ in VisibleLine(text: "# \u{2401}\u{301}", offsets: [0, 1, 2, 3, 99], marks: [2..<99]) }
        let drawn = DrawnLine("# \u{1}\u{301}", spans: [SyntaxSpan(range: 0..<4, key: "comment"), SyntaxSpan(range: 3..<400, key: "x")], tabWidth: 4, limit: 5_000)
        XCTAssertEqual(drawn.scalars, 4)
        XCTAssertTrue(drawn.spans.allSatisfy { $0.range.upperBound <= drawn.scalars })
        XCTAssertTrue(drawn.marks.allSatisfy { $0.upperBound <= drawn.scalars })
        let code = DiffCode(tokens: ThemeStore.shared.tokens)
        _ = code.text("# \u{1}\u{301}", spans: [SyntaxSpan(range: 0..<4, key: "comment")])
        // The tab and the override with a mark after them, and a line cut at the limit.
        VisibleRules.line = { text, _ in VisibleLine(text: text) }
        _ = code.text("#\t\u{301}", spans: [SyntaxSpan(range: 0..<3, key: "comment")])
        _ = code.text("a\u{202E}\u{301}b", spans: [SyntaxSpan(range: 0..<4, key: "comment")])
        let cut = DrawnLine(String(repeating: "a", count: DiffCode.maxRowCharacters + 50), spans: [], tabWidth: 4, limit: DiffCode.maxRowCharacters)
        XCTAssertEqual(cut.scalars, DiffCode.maxRowCharacters + 1)
        XCTAssertTrue(cut.text.hasSuffix("\u{2026}"))
        XCTAssertEqual(cut.marks.last, DiffCode.maxRowCharacters..<DiffCode.maxRowCharacters + 1)
    }

    func testAStepOpensTheNextOneBelowItAndAnEmptyNoteIsThrownAway() {
        var list = TaskList()
        let first = list.add("one")!, third = list.add("three")!
        let second = list.add("two", after: first.id)
        XCTAssertEqual(list.items.map(\.text), ["one", "two", "three"])
        XCTAssertNotNil(second)
        XCTAssertNil(list.add("  ", after: third.id), "blank adds nothing")
        let store = NotesStore(model: NotesModel(notes: [Note(text: "keep"), Note(text: " \n")]))
        store.finishWriting(store.model.notes[1])
        XCTAssertEqual(store.model.notes.map(\.text), ["keep"])
        store.newNote()
        XCTAssertNotNil(store.freshNote, "a new note opens for writing at once")
        let last = store.model.notes[1]
        store.removeEmpty(last)
        XCTAssertEqual(store.freshNote, store.model.notes[0].id, "the note above opens")
    }
}
