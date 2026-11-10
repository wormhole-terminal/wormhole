import PanelModel
import XCTest

@testable import PanelUI

/// Behaviour the GPUI app has and the panel gained in round S: what a diff line shows for characters a person cannot
/// see, git's problems in words, and the writing flows of the Notes tab.
@MainActor
final class PanelParityTests: XCTestCase {
    func testInvisibleCharactersAreSpelledOutAndLongLinesCut() {
        let shown = DiffCode.expandTabs("if x\u{202E} \u{2066}{\u{1B}[31m\t.", tabWidth: 4)
        XCTAssertEqual(shown.text, "if x<U+202E> <U+2066>{\u{241B}[31m .")
        XCTAssertEqual(shown.marks.count, 3)
        XCTAssertEqual(DiffCode.expandTabs("\u{85}\u{2028}\u{7F}").text, "<U+0085><U+2028>\u{2421}")
        let long = String(repeating: "a", count: DiffCode.maxRowCharacters + 50)
        let cut = DiffCode.expandTabs(long)
        XCTAssertEqual(cut.text.count, DiffCode.maxRowCharacters + 1)
        XCTAssertTrue(cut.text.hasSuffix("\u{2026}"))
    }

    func testGitProblemsSayWhatToDo() {
        let missing = GitProblem(message: "git is not installed", tab: .history)
        XCTAssertEqual(missing.headline, "History needs git, and it isn't installed.")
        XCTAssertEqual(missing.detail, "xcode-select --install")
        XCTAssertNil(missing.copy)
        let owner = GitProblem(message: "fatal: detected dubious ownership in repository at '/srv/repo'\nmore", tab: .changes)
        XCTAssertEqual(owner.detail, "git config --global --add safe.directory '/srv/repo'")
        let quoted = GitProblem(message: "fatal: detected dubious ownership in repository at '/srv/it's'", tab: .changes)
        XCTAssertEqual(quoted.headline, "Couldn't read this repository.", "a quote in the folder's name leaves the generic wording")
        XCTAssertEqual(quoted.copy, "fatal: detected dubious ownership in repository at '/srv/it's'")
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
