import DesignSystem
import PanesModel
import SwiftUI
import XCTest

@testable import PanesUI

@MainActor
final class TitleSnapshotTests: XCTestCase {
    private func row(_ facts: TitleFacts, width: CGFloat) -> some View {
        TitleRowView(facts: facts, backend: SampleOpenIn()).padding(.horizontal, 8).frame(width: width, height: 38)
    }

    func testTitleRow() throws {
        for look in PaneSnapshotTests.looks {
            for (name, facts, width) in [("title-agent", TitleSamples.agent, 880.0), ("title-full", TitleSamples.full, 960.0),
                                         ("title-narrow", TitleSamples.full, 520.0), ("title-tiny", TitleSamples.full, 300.0),
                                         ("title-plain", TitleSamples.plain, 600.0)] {
                _ = try Snapshot.write(row(facts, width: width), name: name, theme: look.id, contrast: look.contrast, size: CGSize(width: width, height: 38))
            }
        }
    }
}
