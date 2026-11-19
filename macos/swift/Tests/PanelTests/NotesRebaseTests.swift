import PanelModel
import XCTest

/// A merge's answer over what the terminal holds by then (`NotesModel.rebase`). Every model here is built anew, as a load
/// from the database and a merge build theirs, so no two share an id: the rule must tell items apart by what they say.
final class NotesRebaseTests: XCTestCase {
    private func model(steps: [(String, Bool)] = [], notes: [String] = []) -> NotesModel {
        NotesModel(notes: notes.map { Note(text: $0) }, tasks: TaskList(steps.map { PanelTask(text: $0.0, done: $0.1) }))
    }

    private func steps(_ model: NotesModel) -> [String] { model.tasks.items.map { "\($0.done ? "x" : " ") \($0.text)" } }

    func testATerminalNotOnShowIsNotDoubledByAMergeLoadedTwice() {
        // What the coordinator does for a terminal that is not shown: it loads the notes to send, and loads them again when
        // the answer lands. The two loads say the same and share no id.
        let sent = model(steps: [("build", false), ("ship", false)], notes: ["Ask Ann"])
        let now = model(steps: [("build", false), ("ship", false)], notes: ["Ask Ann"])
        let merged = model(steps: [("build", false), ("ship", false), ("from the file", false)], notes: ["Ask Ann", "A file's note"])
        let result = NotesModel.rebase(merged: merged, sent: sent, now: now)
        XCTAssertEqual(steps(result), ["  build", "  ship", "  from the file"])
        XCTAssertEqual(result.notes.map(\.text), ["Ask Ann", "A file's note"])
    }

    func testAStepTickedDuringTheMergeStaysTicked() {
        let sent = model(steps: [("build", false), ("ship", false)])
        var now = sent
        now.tasks = TaskList([PanelTask(text: "build", done: true), PanelTask(text: "ship")])
        // The merge's answer is built from the file's lines: new ids, and it knows nothing of the tick.
        let merged = model(steps: [("build", false), ("ship", false), ("from the file", false)])
        XCTAssertEqual(steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["x build", "  ship", "  from the file"])
    }

    func testTheFilesOwnTickStaysWhenThePersonDidNothing() {
        let sent = model(steps: [("build", false)])
        let now = model(steps: [("build", false)])
        let merged = model(steps: [("build", true)])
        XCTAssertEqual(steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["x build"])
    }

    func testAStepAddedOrDeletedDuringTheMergeStaysSo() {
        let sent = model(steps: [("build", false), ("test", false), ("ship", false)])
        // Meanwhile: "test" deleted, "review" added after "build".
        let now = model(steps: [("build", false), ("review", false), ("ship", false)])
        let merged = model(steps: [("build", false), ("test", false), ("ship", false), ("from the file", false)])
        XCTAssertEqual(
            steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["  build", "  review", "  ship", "  from the file"])
    }

    func testTwoStepsThatSayTheSameAreToldApartByTheirPlace() {
        let sent = model(steps: [("call Ann", false), ("call Ann", false)])
        // The second of the two is ticked meanwhile.
        let now = model(steps: [("call Ann", false), ("call Ann", true)])
        let merged = model(steps: [("call Ann", false), ("call Ann", false)])
        XCTAssertEqual(steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["  call Ann", "x call Ann"])
    }

    func testANoteRewrittenDuringTheMergeKeepsItsNewWordsOnce() {
        let sent = model(notes: ["Ask Ann", "Deploy on Friday"])
        let now = model(notes: ["Ask Ann about the key", "Deploy on Friday"])
        let merged = model(notes: ["Ask Ann", "Deploy on Friday", "A file's note"])
        XCTAssertEqual(
            NotesModel.rebase(merged: merged, sent: sent, now: now).notes.map(\.text),
            ["Ask Ann about the key", "Deploy on Friday", "A file's note"])
    }

    func testAReorderDuringTheMergeIsKeptAndTheItemsKeepTheirIds() {
        let sent = model(steps: [("a", false), ("b", false)], notes: ["one", "two"])
        var now = sent
        now.tasks.move(from: 1, to: 0)
        now.notes.swapAt(0, 1)
        let merged = model(steps: [("a", false), ("b", false), ("c", false)], notes: ["one", "two", "three"])
        let result = NotesModel.rebase(merged: merged, sent: sent, now: now)
        // The person's order; what the file added follows the item it follows in the file.
        XCTAssertEqual(steps(result), ["  b", "  c", "  a"])
        XCTAssertEqual(result.notes.map(\.text), ["two", "three", "one"])
        XCTAssertEqual([result.tasks.items[0].id, result.tasks.items[2].id], now.tasks.items.map(\.id), "an editor stays on its row")
    }

    func testAStepTheFileRemovedStaysWhenThePersonTickedItMeanwhile() {
        let sent = model(steps: [("build", false), ("ship", false)])
        let now = model(steps: [("build", false), ("ship", true)])
        let merged = model(steps: [("build", false)])
        XCTAssertEqual(steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["  build", "x ship"])
    }

    func testAStepThePersonDeletedComesBackWhenTheFileTickedItMeanwhile() {
        // The core's rule (`merge.rs`): deleting a step on one side removes it unless the other side changed it.
        let sent = model(steps: [("build", false), ("ship", false), ("tell Ann", false)])
        // Meanwhile the person deletes "ship" and "tell Ann"; the file ticks "ship" and leaves "tell Ann" as it was.
        let now = model(steps: [("build", false)])
        let merged = model(steps: [("build", false), ("ship", true), ("tell Ann", false)])
        XCTAssertEqual(steps(NotesModel.rebase(merged: merged, sent: sent, now: now)), ["  build", "x ship"])
    }
}
