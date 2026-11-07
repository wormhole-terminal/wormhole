import AppKit
import DesignSystem
import SidebarUI
import SwiftUI
import WorkspaceModel
import XCTest

/// The data of the GPUI reference screenshots (`gpui-ref/dark-cmd-hints.png`): one pinned shell, a plain one, a group of
/// three agents (the first selected) and a scratch terminal.
enum SidebarFixture {
    static let state: WorkspaceState = {
        let terminals = [
            TerminalSummary(id: 1, title: "zsh", subtitle: ".../scratchpad/ref-repo", isPinned: true),
            TerminalSummary(id: 2, title: "build docs", subtitle: "~"),
            TerminalSummary(id: 3, title: "sidebar polish", activity: .working),
            TerminalSummary(id: 4, title: "diff perf", subtitle: "Approve cargo test?", activity: .needsInput),
            TerminalSummary(id: 5, title: "rate limits", subtitle: "1m ago", activity: .finished),
            TerminalSummary(id: 6, title: "scratch", subtitle: "~"),
        ]
        return WorkspaceState(terminals: terminals, groups: [TerminalGroup(id: "g1", name: "Agents", members: [3, 4, 5])], selected: 3)
    }()
}

/// Renders through a hosting view in an off-screen window: `ImageRenderer` draws text fields and scroll views wrong.
@MainActor
enum SidebarSnapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["SIDEBAR_PNG_DIR"]
            ?? "/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/swift-sidebar")

    @discardableResult
    static func write(
        _ state: WorkspaceState = SidebarFixture.state, theme: String, contrast: Bool = false, appearance: SidebarAppearance = .init(),
        name: String, size: CGSize = CGSize(width: 350, height: 560), settle: TimeInterval = 0.4
    ) throws -> URL {
        let library = ThemeLibrary.bundled
        ThemeStore.shared.increaseContrast = contrast
        let theme = try XCTUnwrap(library.theme(id: theme))
        var appearance = appearance
        // The fixture's group wears the theme's primary, as a blue group does in the reference.
        if appearance.groupWash.isEmpty { appearance.groupWash = ["g1": theme.tokens(increaseContrast: contrast).primary] }
        let view = SidebarView(state: state, appearance: appearance, send: { _ in })
            .environment(\.tokens, theme.tokens(increaseContrast: contrast))
            .environment(\.colorScheme, theme.appearance.colorScheme)
            .frame(width: size.width, height: size.height)
        let host = NSHostingView(rootView: view)
        host.frame = CGRect(origin: .zero, size: size)
        let window = NSWindow(contentRect: host.frame, styleMask: [.borderless], backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: theme.appearance == .dark ? .darkAqua : .aqua)
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
