import AppKit
import DesignSystem
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
        TerminalSummary(id: 1, title: "zsh", folder: "~/Projects/wormhole", branch: "main"),
        TerminalSummary(id: 2, title: "agent", folder: "~/Projects/wormhole", branch: "exp/swift-ui", activity: .working),
    ]

    private func snapshot(
        _ state: WorkspaceState, name: String, size: CGSize = CGSize(width: 1200, height: 720), statusBar: Bool = false,
        mode: ThemeMode = .dark, theme: String? = nil
    ) throws -> Bool {
        ThemeStore.shared.mode = mode
        ThemeStore.shared.reduceTransparency = true  // opaque, like the reference's own
        if let theme { ThemeStore.shared.setTheme(theme) }
        let look = WindowLook()
        look.statusBar = statusBar
        let root = WorkspaceRootView(hub: WindowHub(store: WorkspaceStore(backend: MockWorkspaceBackend(items: state.terminals.map { .loose(MockTerminal($0)) }, selected: state.selected, chrome: ChromeState(sidebarVisible: state.sidebarVisible, panelVisible: state.panelVisible, panelTab: state.panelTab))), terminals: TerminalViews { _ in nil }), look: look)
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

    /// The same window as the GPUI reference screenshots (1320 x 820, status bar on), per appearance.
    @Test(arguments: [ThemeMode.dark, .light]) func referenceSizedWindow(mode: ThemeMode) throws {
        let state = WorkspaceState(terminals: Self.terminals, selected: 2, panelTab: .notes)
        #expect(try snapshot(state, name: "ref-\(mode.rawValue)", size: CGSize(width: 1320, height: 820), statusBar: true, mode: mode))
    }

    @Test func narrowWindowKeepsTheTerminalsMinimum() {
        var layout = PaneLayout()
        layout.sidebarWidth = 420
        layout.panelWidth = 720
        let narrow = layout.columns(window: 640)
        #expect(narrow.terminal >= PaneLayout.terminalMin)
        #expect(layout.panelVisible(window: 640) == false)
        let wide = PaneLayout().columns(window: 1600)
        #expect(wide.sidebar == 350 && wide.panel == 360)
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

    /// The tab bar has no folds (`tab_sets`): a group folded in the sidebar, which lists none of its rows, keeps its tabs.
    @Test func aFoldedGroupKeepsItsTabsWhenTerminalsAreShownAsTabs() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend(items: MockWorkspace.items, selected: 8))
        let hub = WindowHub(store: store, terminals: TerminalViews { _ in nil })
        #expect(hub.state.sections.first { $0.head?.id == 3 }?.rows.isEmpty == true, "Infra is folded: no rows")
        let bar = hub.tabBar
        #expect(bar.setLabel == "Infra" && bar.tabs.map(\.id) == [8, 9] && bar.tabs.map(\.name) == ["ssh build-mac", "terraform plan"])
        #expect(bar.sets.first { $0.label == "Infra" }?.members == [8, 9])
    }

    /// Space in the sidebar shows a terminal and keeps the keyboard in the list: the next terminal shown does not take the keys,
    /// and only that one (↩ and a click give the terminal the keyboard).
    @Test func spaceInTheListLeavesTheKeysOnlyForTheNextTerminalShown() {
        let views = TerminalViews { _ in nil }
        #expect(views.takesKeys())
        views.keepKeys()
        #expect(!views.takesKeys())
        #expect(views.takesKeys())
    }

    /// The tab numbers are the core's ⌘ numbers, not positions: a terminal folded away in its group has none.
    @Test func aTabNumberIsTheCoresNumber() {
        let store = WorkspaceStore(backend: MockWorkspaceBackend(items: MockWorkspace.items, selected: 8))
        let hub = WindowHub(store: store, terminals: TerminalViews { _ in nil })
        let numbers = hub.tabBar.tabs.map(\.number)
        #expect(numbers == [nil, nil], "Infra is folded: its terminals have no ⌘ number")
    }
}
