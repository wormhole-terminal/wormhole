// What the window shows, as plain values. No UI framework, no I/O: the Rust core fills it later
// (`swift-headless-workspace.md`); until then `MockWorkspace` does. Every UI module reads only this.

import Foundation

public typealias TerminalID = Int

/// What a terminal is doing, in the words of the sidebar's status dot.
public enum Activity: Sendable, Equatable {
    case idle
    case working
    case needsInput
    case finished
    case failed
}

/// One sidebar row: a terminal, as the list shows it.
public struct TerminalSummary: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var title: String
    public var subtitle: String?  // folder or running program
    public var branch: String?
    public var activity: Activity
    public var isPinned: Bool
    public var unread: Int

    public init(
        id: TerminalID, title: String, subtitle: String? = nil, branch: String? = nil,
        activity: Activity = .idle, isPinned: Bool = false, unread: Int = 0
    ) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.branch = branch
        self.activity = activity
        self.isPinned = isPinned
        self.unread = unread
    }
}

/// A named run of rows in the sidebar.
public struct TerminalGroup: Sendable, Equatable, Identifiable {
    public var id: String
    public var name: String
    public var members: [TerminalID]
    public var isCollapsed: Bool

    public init(id: String, name: String, members: [TerminalID], isCollapsed: Bool = false) {
        self.id = id
        self.name = name
        self.members = members
        self.isCollapsed = isCollapsed
    }
}

/// The right panel's tabs.
public enum PanelTab: String, Sendable, CaseIterable {
    case notes, changes, history
}

/// What is laid over the window, at most one at a time.
public enum Overlay: Sendable, Equatable {
    case palette, shortcuts
}

/// The whole window's state.
public struct WorkspaceState: Sendable, Equatable {
    public var terminals: [TerminalSummary]
    public var groups: [TerminalGroup]
    public var selected: TerminalID?
    public var sidebarVisible: Bool
    public var panelVisible: Bool
    public var panelTab: PanelTab
    public var search: String
    public var overlay: Overlay?

    public init(
        terminals: [TerminalSummary] = [], groups: [TerminalGroup] = [], selected: TerminalID? = nil,
        sidebarVisible: Bool = true, panelVisible: Bool = true, panelTab: PanelTab = .changes, search: String = "",
        overlay: Overlay? = nil
    ) {
        self.terminals = terminals
        self.groups = groups
        self.selected = selected
        self.sidebarVisible = sidebarVisible
        self.panelVisible = panelVisible
        self.panelTab = panelTab
        self.search = search
        self.overlay = overlay
    }
}

/// Everything a person can do to the window. The UI sends these; it never edits state itself.
public enum WorkspaceAction: Sendable, Equatable {
    case select(TerminalID)
    case newTerminal
    case close(TerminalID)
    case togglePin(TerminalID)
    case toggleGroup(String)
    case toggleSidebar
    case togglePanel
    case showPanelTab(PanelTab)
    case search(String)
    case showOverlay(Overlay?)
}
