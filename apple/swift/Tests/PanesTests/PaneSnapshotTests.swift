import DesignSystem
import PanesModel
import SwiftUI
import XCTest

@testable import PanesUI

@MainActor
final class PaneSnapshotTests: XCTestCase {
    static let looks: [(id: String, contrast: Bool)] = [
        ("wormhole.dark", false), ("wormhole.light", false), ("catppuccin_mocha", false), ("github_light", false),
        ("wormhole.dark", true),
    ]

    private func each<V: View>(_ name: String, size: CGSize = CGSize(width: 900, height: 600), _ view: @autoclosure () -> V) throws {
        for look in Self.looks {
            let url = try Snapshot.write(view(), name: name, theme: look.id, contrast: look.contrast, size: size)
            XCTAssertGreaterThan((try? Data(contentsOf: url).count) ?? 0, 2_000, name)
        }
    }

    private func area(_ panes: TabPanes, gaps: Bool) -> some View {
        PaneArea(panes: .constant(panes), gaps: gaps, session: PaneDragSession(), info: { PaneSamples.info[$0] ?? PaneInfo(title: "?") }) {
            SampleTerminal(id: $0)
        }
    }

    func testPanes() throws {
        try each("panes-gaps", area(PaneSamples.split, gaps: true).padding(8))
        try each("panes-plain", area(PaneSamples.split, gaps: false))
        try each("panes-grid", area(PaneSamples.grid, gaps: true).padding(8))
        var zoomed = PaneSamples.split
        _ = zoomed.toggleZoom()
        try each("panes-zoomed", area(zoomed, gaps: true).padding(8))
        try each("panes-single", area(.single(1), gaps: true).padding(8))
    }
}
