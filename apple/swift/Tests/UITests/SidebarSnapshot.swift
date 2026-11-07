import AppKit
import DesignSystem
@testable import SidebarUI
import SwiftUI
import WorkspaceModel
import XCTest

/// The instant the fixture's second lines read as, so "Working · 1m" stays that.
let sidebarNow = Date(timeIntervalSinceReferenceDate: 800_000_000)

/// The data of the GPUI reference screenshots (`gpui-ref/dark-sidebar-crop-opaque.png`): one pinned terminal, a plain one,
/// a group of three (working, needs input, finished; the first selected) and a scratch terminal, all Claude's. The pinned one
/// is not in the arrangement: pinned terminals lead the loose ones wherever it puts them.
enum SidebarFixture {
    static let home = "/Users/ref"
    static let state: WorkspaceState = {
        func ago(_ seconds: TimeInterval) -> Date { sidebarNow.addingTimeInterval(-seconds) }
        let terminals = [
            TerminalSummary(
                id: 1, title: "zsh", isPinned: true, agent: .claude,
                folder: "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/ref-repo"),
            TerminalSummary(id: 2, title: "build docs", agent: .claude, folder: home),
            TerminalSummary(id: 3, title: "sidebar polish", activity: .working, agent: .claude, activitySince: ago(70), folder: home),
            TerminalSummary(
                id: 4, title: "diff perf", activity: .needsInput, agent: .claude, message: "Approve cargo test?",
                activitySince: ago(30), folder: home),
            TerminalSummary(id: 5, title: "rate limits", activity: .finished, agent: .claude, activitySince: ago(70), folder: home),
            TerminalSummary(id: 6, title: "scratch", agent: .claude, folder: home),
        ]
        let group = TerminalGroup(id: "g1", name: "Agents", members: [3, 4, 5], color: .blue)
        return WorkspaceState(terminals: terminals, groups: [group], selected: 3, arrangement: [.terminal(2), .group("g1"), .terminal(6)])
    }()
}

extension SidebarFixture {
    /// One terminal per agent the registry knows, in a mix of states: every logo, monogram and brand colour.
    static let agents: WorkspaceState = {
        let states: [Activity] = [.working, .idle, .needsInput, .finished, .failed]
        let terminals = AgentID.allCases.enumerated().map { at, agent in
            TerminalSummary(
                id: at + 1, title: agent.name, activity: states[at % states.count], agent: agent,
                activitySince: sidebarNow.addingTimeInterval(-120), folder: home)
        }
        return WorkspaceState(terminals: terminals, selected: 2)
    }()
}

/// Renders through a hosting view in an off-screen window: `ImageRenderer` draws text fields and scroll views wrong.
@MainActor
enum SidebarSnapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["SIDEBAR_PNG_DIR"]
            ?? "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/sidebar2")

    @discardableResult
    static func write(
        _ state: WorkspaceState = SidebarFixture.state, theme: String, contrast: Bool = false, appearance: SidebarAppearance = .init(),
        name: String, size: CGSize = CGSize(width: 350, height: 560), settle: TimeInterval = 0.4,
        caught: SidebarView.Caught? = nil, groupEdits: GroupEdits? = nil
    ) throws -> URL {
        let library = ThemeLibrary.bundled
        ThemeStore.shared.increaseContrast = contrast
        let theme = try XCTUnwrap(library.theme(id: theme))
        var appearance = appearance
        appearance.now = sidebarNow
        appearance.home = SidebarFixture.home
        return try render(
            SidebarView(state: state, appearance: appearance, groupEdits: groupEdits, caught: caught, send: { _ in }),
            theme: theme, contrast: contrast, name: name, size: size, settle: settle)
    }

    /// Any view, themed and set on the window's own column surface.
    @discardableResult
    static func render(
        _ content: some View, theme: Theme, contrast: Bool = false, name: String, size: CGSize, settle: TimeInterval = 0.4
    ) throws -> URL {
        let view = content
            .environment(\.tokens, theme.tokens(increaseContrast: contrast))
            .environment(\.colorScheme, theme.appearance.colorScheme)
            .frame(width: size.width, height: size.height)
            // The window's own column surface, which the list leaves to it.
            .background(theme.tokens(increaseContrast: contrast).sidebarBackground)
        let host = NSHostingView(rootView: view)
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: theme.appearance == .dark ? .darkAqua : .aqua)
        // Tokens are sRGB: a window in the display's colour space would shift every pixel a little.
        window.colorSpace = .sRGB
        window.contentView = host
        host.layoutSubtreeIfNeeded()
        RunLoop.current.run(until: Date().addingTimeInterval(settle))
        host.layoutSubtreeIfNeeded()
        let rep = try XCTUnwrap(host.bitmapImageRepForCachingDisplay(in: host.bounds))
        host.cacheDisplay(in: host.bounds, to: rep)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let url = directory.appendingPathComponent("\(name).png")
        try XCTUnwrap(rep.representation(using: .png, properties: [:])).write(to: url)
        return url
    }
}
