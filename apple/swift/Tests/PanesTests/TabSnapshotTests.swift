import DesignSystem
import PanesModel
import SwiftUI
import XCTest

@testable import PanesUI

@MainActor
final class TabSnapshotTests: XCTestCase {
    func testTabsAndPanesStates() throws {
        for look in PaneSnapshotTests.looks {
            func write<V: View>(_ name: String, _ size: CGSize, _ view: V) throws {
                _ = try Snapshot.write(view, name: name, theme: look.id, contrast: look.contrast, size: size)
            }
            try write("tabs", CGSize(width: 900, height: 44), TabBarView(model: TabSamples.bar(), profiles: TabSamples.profiles))
            try write("tabs-hints", CGSize(width: 900, height: 44), TabBarView(model: TabSamples.bar(), hints: true))
            try write("tabs-switcher", CGSize(width: 360, height: 360), GroupSwitcherView(model: TabSamples.bar(), actions: TabBarActions()) {})
            // A terminal dragged from the sidebar over the right half of the second pane.
            let panes = PaneSamples.split
            var state = PaneDropState()
            state.dragged = DraggedTerminal(id: 9, origin: .sidebarRow)
            (state.target, state.zone) = (2, .bottom)
            try write("panes-drop", CGSize(width: 900, height: 600), PaneDropZones(panes: panes, state: state, size: CGSize(width: 900, height: 600), names: { PaneSamples.info[$0]?.title ?? "" }))
        }
    }
}
