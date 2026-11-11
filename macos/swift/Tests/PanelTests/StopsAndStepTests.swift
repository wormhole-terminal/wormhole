import PanelModel
import XCTest

@testable import PanelUI

/// The panel's Tab stops (one per section) and an open diff that follows its file.
@MainActor
final class StopsAndStepTests: XCTestCase {
    private func settle() async throws { try await Task.sleep(for: .milliseconds(200)) }

    func testEachSectionIsATabStopAndEnteringPutsTheCursorOnItsBand() async throws {
        let c = PanelController(host: MockPanelHost(), backends: .mock(), notes: PanelSamples.notes)
        c.follow(folder: "/p")
        try await settle()
        let changes = c.changes
        let bands = changes.walkRows.filter(\.isBand)
        XCTAssertEqual(changes.listStarts.count, bands.count)
        XCTAssertGreaterThanOrEqual(bands.count, 1)
        changes.enter(list: 99)
        XCTAssertEqual(changes.cursor, bands.last?.id, "past the last stop is the last one")
        XCTAssertEqual(changes.listAt, bands.count - 1)
        changes.enter(list: 0)
        XCTAssertEqual(changes.cursor, bands.first?.id)
    }

    func testTheFilesPathButtonIsTheFirstRowAndAStopNotAFolder() async throws {
        let c = PanelController(host: MockPanelHost(), backends: .mock(), notes: PanelSamples.notes)
        c.follow(folder: "/p")
        try await settle()
        let rows = c.files.walkRows
        XCTAssertEqual(rows.first?.isBand, true)
        XCTAssertEqual(rows.first?.id, FilesStore.headID)
        XCTAssertEqual(c.files.listStarts, [0])
        c.files.fold(rows[0])
        XCTAssertEqual(c.files.walkRows, rows, "Space on the head folds nothing")
    }

    func testAnOpenDiffIsReadAgainInPlaceAndKeepsItsRowsUnderOutOfDate() async throws {
        let session = DiffSession(highlighter: LexerHighlighter())
        let file = PanelSamples.files[0]
        let diff = PanelSamples.diffs[file.path] ?? DiffFile(newPath: file.path)
        session.open(file) { _ in diff }
        try await settle()
        XCTAssertNotNil(session.parsed)
        session.markGone()
        XCTAssertTrue(session.gone, "the status no longer lists it: the diff stays, badged")
        session.reread(file, stamp: "2") // a read that fails keeps what showed
        session.open(file) { _ in diff }
        try await settle()
        XCTAssertFalse(session.gone)
        XCTAssertNil(session.refreshError)
    }
}
