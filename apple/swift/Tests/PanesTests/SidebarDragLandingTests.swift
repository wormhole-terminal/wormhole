import DesignSystem
import PanesModel
import SwiftUI
import WorkspaceModel
import XCTest

@testable import PanesUI

/// A row dragged out of the sidebar's list lands on the pane under the pointer, on the edge it is nearest, by the same
/// rules as a drop of the system's drag session.
final class SidebarDragLandingTests: XCTestCase {
    private let size = CGSize(width: 900, height: 600)

    func testThePointerAimsAtThePaneAndEdgeUnderIt() {
        let panes = PaneSamples.split
        let first = panes.shown(width: size.width, height: size.height)[0]
        var state = PaneDropState()
        state.aim(at: CGPoint(x: first.rect.x + first.rect.w - 4, y: first.rect.y + first.rect.h / 2), in: panes, size: size)
        XCTAssertEqual(state.target, first.id)
        XCTAssertEqual(state.zone, .right, "near the right edge of the pane")
        state.aim(at: CGPoint(x: -20, y: 300), in: panes, size: size)
        XCTAssertNil(state.target, "left of the area is no pane")
    }

    func testLettingGoJoinsATerminalTheSplitDoesNotHaveAndMovesOneItHas() throws {
        var panes = PaneSamples.split
        var state = PaneDropState(dragged: DraggedTerminal(id: 99, origin: .sidebarRow))
        state.aim(at: CGPoint(x: 880, y: 300), in: panes, size: size)
        let landed = try XCTUnwrap(state.perform(on: &panes, size: size))
        XCTAssertEqual(landed.id, 99)
        XCTAssertTrue(panes.contains(99), "it joined the split")

        let mate = try XCTUnwrap(panes.panes.first { $0 != 99 })
        var again = PaneDropState(dragged: DraggedTerminal(id: mate, origin: .sidebarRow))
        again.aim(at: CGPoint(x: 880, y: 300), in: panes, size: size)
        let count = panes.panes.count
        _ = again.perform(on: &panes, size: size)
        XCTAssertEqual(panes.panes.count, count, "a terminal already in the split is moved, not added twice")
    }

    func testNothingLandsWithoutAnAim() {
        var panes = PaneSamples.split
        let state = PaneDropState(dragged: DraggedTerminal(id: 99, origin: .sidebarRow))
        XCTAssertNil(state.perform(on: &panes, size: size))
        XCTAssertFalse(panes.contains(99))
    }

    /// The area while a row is dragged over the second pane's right edge: the zone's tint and pill, and the row's chip.
    @MainActor func testThePanesShowTheRowBeingDragged() throws {
        let panes = PaneSamples.split
        let size = CGSize(width: 900, height: 600)
        let second = panes.shown(width: size.width, height: size.height)[1]
        let pointer = TerminalDragPointer()
        // The window's coordinates: the area stands 8 in from its corner.
        pointer.move(9, to: CGPoint(x: second.rect.x + second.rect.w - 12, y: second.rect.y + second.rect.h / 2 + 8))
        for look in PaneSnapshotTests.looks {
            let view = PaneArea(
                panes: .constant(panes), gaps: true, session: PaneDragSession(), sidebarDrag: pointer,
                info: { PaneSamples.info[$0] ?? PaneInfo(title: "diff perf", activity: .needsInput) },
                content: { SampleTerminal(id: $0) }
            ).padding(8)
            _ = try Snapshot.write(view, name: "panes-sidebar-drag", theme: look.id, contrast: look.contrast, size: CGSize(width: 916, height: 616))
        }
    }
}
