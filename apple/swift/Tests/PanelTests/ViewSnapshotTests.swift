import DesignSystem
import PanelModel
import SwiftUI
import XCTest

@testable import PanelUI

@MainActor
final class ViewSnapshotTests: XCTestCase {
    /// Themes whose light/dark and accent differ, and the two default ones under Increase Contrast.
    static let looks: [(id: String, contrast: Bool)] = [
        ("wormhole.dark", false), ("wormhole.light", false), ("catppuccin_mocha", false), ("github_light", false),
        ("nord", false), ("wormhole.dark", true), ("wormhole.light", true),
    ]

    private func each<V: View>(_ name: String, _ view: @autoclosure () -> V, size: CGSize? = nil) throws {
        for look in Self.looks {
            let url = try Snapshot.write(view(), name: name, theme: look.id, contrast: look.contrast, size: size ?? CGSize(width: 360, height: 620))
            XCTAssertGreaterThan((try? Data(contentsOf: url).count) ?? 0, 2_000, name)
        }
    }

    func testNotes() throws {
        try each("notes", NotesView(model: .constant(PanelSamples.notes)))
        try each("notes-empty", NotesView(model: .constant(NotesModel())))
    }

    func testChanges() throws {
        let files = PanelSamples.changes.files
        try each("changes", ChangesView(model: PanelSamples.changes))
        try each("changes-flat", ChangesView(model: PanelSamples.changes, tree: false))
        try each("changes-diff", ChangesView(model: PanelSamples.changes, selection: files[0].id), size: CGSize(width: 360, height: 760))
        try each("changes-diff-split", ChangesView(model: PanelSamples.changes, selection: files[2].id), size: CGSize(width: 900, height: 620))
        try each("changes-binary", ChangesView(model: PanelSamples.changes, selection: files[4].id))
        try each("changes-empty", ChangesView(model: ChangesModel()))
    }

    func testHistory() throws {
        try each("history", HistoryView(model: PanelSamples.history, now: PanelSamples.now))
        try each("history-open", HistoryView(model: PanelSamples.history, now: PanelSamples.now, expanded: PanelSamples.history.commits[0].id), size: CGSize(width: 360, height: 760))
        try each("history-empty", HistoryView(model: HistoryModel()))
    }
}
