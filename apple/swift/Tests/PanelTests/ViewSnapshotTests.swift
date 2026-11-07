import PanelModel
import SwiftUI
import XCTest

@testable import PanelUI

@MainActor
final class ViewSnapshotTests: XCTestCase {
    private func both<V: View>(_ name: String, size: CGSize? = nil, _ view: @autoclosure () -> V) throws {
        for dark in [false, true] {
            let url = try Snapshot.write(view(), name: name, dark: dark, size: size ?? CGSize(width: 340, height: 620))
            XCTAssertGreaterThan((try? Data(contentsOf: url).count) ?? 0, 2_000, name)
        }
    }

    func testNotes() throws {
        try both("notes", NotesView(model: .constant(PanelSamples.notes)))
        try both("notes-empty", NotesView(model: .constant(NotesModel())))
    }

    func testChanges() throws {
        let files = PanelSamples.changes.listed
        try both("changes", ChangesView(model: PanelSamples.changes))
        try both("changes-settings", ChangesView(model: PanelSamples.changes, selection: files[2].id))
        try both("changes-binary", ChangesView(model: PanelSamples.changes, selection: files[4].id))
        try both("changes-untracked", ChangesView(model: PanelSamples.changes, selection: files[5].id))
        try both("changes-empty", ChangesView(model: ChangesModel()))
    }

    func testHistory() throws {
        try both("history", HistoryView(model: PanelSamples.history, now: PanelSamples.now))
        try both("history-empty", HistoryView(model: HistoryModel()))
    }
}
