import AppKit
import SwiftUI
import Testing
import WorkspaceModel

@testable import WindowUI

/// The window's layout drawn offscreen into PNGs (collapsed, expanded, each panel tab), plus the rules the
/// layout follows. The PNGs are for looking at; the assertions only check that something was drawn.
@MainActor
@Suite struct WindowTests {
    static let output = URL(
        fileURLWithPath: "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/window")

    private static let terminals = [
        TerminalSummary(id: 1, title: "zsh", subtitle: "~/Projects/wormhole", branch: "main"),
        TerminalSummary(id: 2, title: "agent", subtitle: "~/Projects/wormhole", branch: "exp/swift-ui", activity: .working),
    ]

    private func snapshot(_ state: WorkspaceState, name: String, size: CGSize = CGSize(width: 1200, height: 720)) throws -> Bool {
        let root = WorkspaceRootView(state: state, send: { _ in }, terminals: TerminalViews { _ in nil })
        let hosting = NSHostingView(rootView: root)
        hosting.sizingOptions = []
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: size), styleMask: [.titled, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.titlebarAppearsTransparent = true
        window.contentView = hosting
        hosting.frame = NSRect(origin: .zero, size: size)
        hosting.layoutSubtreeIfNeeded()
        guard let bitmap = hosting.bitmapImageRepForCachingDisplay(in: hosting.bounds) else { return false }
        hosting.cacheDisplay(in: hosting.bounds, to: bitmap)
        try FileManager.default.createDirectory(at: Self.output, withIntermediateDirectories: true)
        guard let png = bitmap.representation(using: .png, properties: [:]) else { return false }
        try png.write(to: Self.output.appendingPathComponent("\(name).png"))
        return png.count > 1000
    }

    @Test func expandedLayoutRenders() throws {
        let state = WorkspaceState(terminals: Self.terminals, selected: 1)
        #expect(try snapshot(state, name: "expanded"))
    }

    @Test func collapsedLayoutRenders() throws {
        let state = WorkspaceState(terminals: Self.terminals, selected: 2, sidebarVisible: false, panelVisible: false)
        #expect(try snapshot(state, name: "collapsed"))
    }

    @Test(arguments: PanelTab.allCases) func panelTabRenders(tab: PanelTab) throws {
        let state = WorkspaceState(terminals: Self.terminals, selected: 1, panelTab: tab)
        #expect(try snapshot(state, name: "panel-\(tab.rawValue)", size: CGSize(width: 900, height: 480)))
    }

    @Test func narrowWindowKeepsTheTerminalsMinimum() {
        let open = ColumnWidths(sidebar: 420, panel: 720, window: 640, sidebarVisible: true, panelVisible: true)
        #expect(640 - open.sidebar - open.panel >= Chrome.terminalMin)
        let wide = ColumnWidths(sidebar: 260, panel: 320, window: 1600, sidebarVisible: true, panelVisible: true)
        #expect(wide.sidebar == 260 && wide.panel == 320)
    }

    @Test func selectedTerminalIsFoundById() {
        let state = WorkspaceState(terminals: Self.terminals, selected: 2)
        #expect(state.selectedTerminal?.title == "agent")
        #expect(WorkspaceState().selectedTerminal == nil)
    }

    @Test func aFailedMakeIsRetriedNotCached() {
        var made = 0
        let views = TerminalViews { _ in
            made += 1
            return nil
        }
        _ = views.view(for: 1)
        _ = views.view(for: 1)
        #expect(made == 2)
        #expect(views.existing(1) == nil)
    }
}
