import PanelModel
import WormholeBindings
import XCTest

@testable import FilesBridge

/// The notes as the sync sees them, and a merged document laid back over the model.
final class NotesMappingTests: XCTestCase {
    private func model() -> NotesModel {
        var model = NotesModel(
            notes: [Note(text: "  keep me  ", collapsed: true), Note(text: ""), Note(text: "edited")],
            tasks: TaskList([PanelTask(text: "ship", done: true)]))
        model.stepsCollapsed = true
        return model
    }

    func testTheSavedRecordKeepsEverythingIncludingFolds() {
        let saved = model().saved
        XCTAssertEqual(saved.notes.map(\.body), ["  keep me  ", "", "edited"], "Rust drops blanks when it saves")
        XCTAssertTrue(saved.tasksCollapsed)
        let back = NotesModel(saved: saved)
        XCTAssertEqual(back.notes.map(\.text), ["  keep me  ", "", "edited"])
        XCTAssertEqual(back.notes.map(\.collapsed), [true, false, false])
        XCTAssertEqual(back.tasks.items.map(\.text), ["ship"])
        XCTAssertTrue(back.stepsCollapsed)
    }

    func testTheDocumentHoldsTheStepsAndTheNotesWithSomethingInThemTrimmed() {
        let document = model().document
        XCTAssertEqual(document.notes, ["keep me", "edited"])
        XCTAssertEqual(document.tasks, [TaskDocRecord(text: "ship", done: true)])
    }

    func testAMergedDocumentKeepsTheIdentityOfNotesThatStayAndTheBlankOnesStillBeingWritten() throws {
        let before = model()
        let outcome = NotesSyncOutcome(
            apply: NotesDocRecord(
                tasks: [TaskDocRecord(text: "ship", done: false), TaskDocRecord(text: "new step", done: false)],
                notes: ["from the file", "keep me", "edited, then merged"]),
            noteSlots: [nil, 0, 1])
        let after = try XCTUnwrap(before.applying(outcome))
        XCTAssertEqual(after.tasks.items.map(\.text), ["ship", "new step"])
        XCTAssertEqual(after.tasks.items[0].id, before.tasks.items[0].id, "a step whose words stay keeps its row, unticked by the file")
        XCTAssertEqual(after.notes.map(\.text), ["from the file", "  keep me  ", "edited, then merged", ""])
        XCTAssertEqual(after.notes[1].id, before.notes[0].id, "the note that stayed keeps its identity")
        XCTAssertTrue(after.notes[1].collapsed, "and its fold")
        XCTAssertEqual(after.notes[2].id, before.notes[2].id, "an edited note keeps its editor")
        XCTAssertEqual(after.notes[3].id, before.notes[1].id, "a blank note is still being written: it stays, last")
        XCTAssertTrue(after.stepsCollapsed, "the folds are untouched")
    }

    func testNothingToApplyChangesNothing() {
        XCTAssertNil(model().applying(NotesSyncOutcome(apply: nil, noteSlots: [])))
    }
}
