import AppKit
import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// A group's head as VoiceOver reaches it, read from the hosted list's accessibility tree: a container named by the core's
/// words, with the controls a pointer can press as buttons inside it (GPUI's `groups.rs`: the label "Edit group …", the
/// chevron "Fold group …", the + "New terminal in …"). Before, the head hid its children and the + could not be reached.
@MainActor
final class GroupHeadSpeechTests: XCTestCase {
    func testAGroupHeadsControlsAreButtonsInsideIt() throws {
        let open = GroupHead(id: 1, name: "Agents", color: .blue)
        let empty = GroupHead(id: 2, name: "Release", isPinned: true)
        let items: [MockItem] = [
            .group(open, [MockTerminal(TerminalSummary(id: 3, title: "sidebar polish")), MockTerminal(TerminalSummary(id: 4, title: "diff perf"))]),
            .group(empty, []),
        ]
        let state = WorkspaceState(sections: MockWorkspace.sections(items, selected: 3), selected: 3)
        let tree = try Self.tree(of: SidebarView(state: state, send: { _ in }))
        // SwiftUI may build no nodes for a process that is not an assistive app (DRIVING.md, "What still needs a person").
        guard !tree.labels.isEmpty else {
            throw XCTSkip("the hosted list vends no accessibility nodes in this process: check the heads with VoiceOver")
        }
        let heads = state.sections.compactMap(\.head)
        let agents = try XCTUnwrap(tree.find(heads[0].spoken), "the open group's head is an element with the core's words:\n\(tree.labels)")
        XCTAssertTrue(agents.labels.contains("Edit group Agents"), "its name opens the popover:\n\(agents.labels)")
        XCTAssertTrue(agents.labels.contains("Fold group Agents"), "its chevron folds it:\n\(agents.labels)")
        let release = try XCTUnwrap(tree.find(heads[1].spoken), "the pinned, empty group's head:\n\(tree.labels)")
        XCTAssertTrue(release.labels.contains("New terminal in Release"), "its + is reachable inside it:\n\(release.labels)")
    }

    /// The list in an off-screen window, as `SidebarSnapshot` hosts it, and its accessibility tree once it has drawn.
    private static func tree(of view: some View) throws -> SpokenNode {
        let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: "wormhole.dark"))
        let size = CGSize(width: 350, height: 400)
        let host = NSHostingView(rootView: view.environment(\.tokens, theme.tokens()).frame(width: size.width, height: size.height))
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(0.3))
        host.layoutSubtreeIfNeeded()
        return SpokenNode(host)
    }
}

/// One element of an accessibility tree: what VoiceOver calls it, and the elements under it.
private struct SpokenNode {
    let label: String
    let children: [SpokenNode]

    @MainActor init(_ element: Any, depth: Int = 0) {
        let node = element as? NSAccessibilityProtocol
        label = node?.accessibilityLabel() ?? ""
        children = depth < 60 ? (node?.accessibilityChildren() ?? []).map { SpokenNode($0, depth: depth + 1) } : []
    }

    /// Every non-empty label in this subtree, this one first.
    var labels: [String] { (label.isEmpty ? [] : [label]) + children.flatMap(\.labels) }

    /// The first element of this subtree named `wanted`.
    func find(_ wanted: String) -> SpokenNode? { label == wanted ? self : children.lazy.compactMap { $0.find(wanted) }.first }
}
