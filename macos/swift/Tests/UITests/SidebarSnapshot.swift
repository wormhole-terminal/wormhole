import AppKit
import DesignSystem
@testable import SidebarUI
import SwiftUI
import WorkspaceModel
import XCTest

/// The data of the GPUI reference screenshots (`gpui-ref/dark-sidebar-crop-opaque.png`): one pinned terminal, a plain one,
/// a group of three (working, needs input, finished; the first selected) and a scratch terminal, all Claude's, with the
/// words the core gives their second lines.
enum SidebarFixture {
    static let home = "/Users/ref"
    static let state: WorkspaceState = {
        func claude(_ id: Int, _ title: String, _ activity: Activity = .idle, pinned: Bool = false, message: String? = nil,
                    folder: String = "~", line: SecondLine? = nil) -> MockTerminal {
            MockTerminal(
                TerminalSummary(id: id, title: title, folder: folder, activity: activity, isPinned: pinned, agent: .claude, message: message), line)
        }
        let head = GroupHead(id: 1, name: "Agents", color: .blue)
        let items: [MockItem] = [
            .loose(claude(1, "zsh", pinned: true, folder: "…/scratchpad/ref-repo")),
            .loose(claude(2, "build docs")),
            .group(
                head,
                [
                    claude(3, "sidebar polish", .working, line: SecondLine(text: "Working · 1m")),
                    claude(4, "diff perf", .needsInput, message: "Approve cargo test?", line: SecondLine(text: "Needs input · Approve cargo test?", tone: .attention)),
                    claude(5, "rate limits", .finished, line: SecondLine(text: "Finished · 1m ago", tone: .done)),
                ]),
            .loose(claude(6, "scratch")),
        ]
        return WorkspaceState(sections: MockWorkspace.sections(items, selected: 3), selected: 3)
    }()

    /// The same list with the group folded.
    static var folded: WorkspaceState {
        var state = state
        state.sections = state.sections.map { section in
            guard var head = section.head else { return section }
            head.isFolded = true
            return SidebarSection(id: section.id, head: head, rows: [], terminalCount: section.terminalCount, alertCount: section.alertCount)
        }
        return state
    }
}

extension SidebarFixture {
    /// One terminal per agent the registry knows, in a mix of states: every logo, monogram and brand colour.
    static let agents: WorkspaceState = {
        let states: [Activity] = [.working, .idle, .needsInput, .finished, .failed]
        func line(_ activity: Activity) -> SecondLine {
            switch activity {
            case .working: SecondLine(text: "Working · 2m")
            case .needsInput: SecondLine(text: "Needs input", tone: .attention)
            case .finished: SecondLine(text: "Finished · 2m ago", tone: .done)
            case .failed: SecondLine(text: "Failed · 2m ago", tone: .failed)
            case .idle, .exited: SecondLine(text: "~")
            }
        }
        let terminals = AgentID.allCases.enumerated().map { at, agent in
            let activity = states[at % states.count]
            return MockTerminal(TerminalSummary(id: at + 1, title: agent.name, folder: "~", activity: activity, agent: agent), line(activity))
        }
        return WorkspaceState(sections: MockWorkspace.sections(terminals.map { .loose($0) }, selected: 2), selected: 2)
    }()
}

/// Renders through a hosting view in an off-screen window: `ImageRenderer` draws text fields and scroll views wrong.
@MainActor
enum SidebarSnapshot {
    static let directory = URL(
        fileURLWithPath: ProcessInfo.processInfo.environment["SIDEBAR_PNG_DIR"]
            ?? NSTemporaryDirectory() + "wormhole-swift-shots/sidebar2")

    @discardableResult
    static func write(
        _ state: WorkspaceState = SidebarFixture.state, theme: String, contrast: Bool = false, appearance: SidebarAppearance = .init(),
        name: String, size: CGSize = CGSize(width: 350, height: 560), settle: TimeInterval = 0.4,
        caught: SidebarView.Caught? = nil
    ) throws -> URL {
        let library = ThemeLibrary.bundled
        ThemeStore.shared.increaseContrast = contrast
        let theme = try XCTUnwrap(library.theme(id: theme))
        return try render(
            SidebarView(state: state, appearance: appearance, hooks: SidebarHooks(), caught: caught, send: { _ in }),
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
