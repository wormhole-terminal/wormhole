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
    /// The top level of the list as the person last arranged it (`arranged()` completes it).
    public var arrangement: [TopItem]

    public init(
        terminals: [TerminalSummary] = [], groups: [TerminalGroup] = [], selected: TerminalID? = nil,
        sidebarVisible: Bool = true, panelVisible: Bool = true, panelTab: PanelTab = .changes, search: String = "",
        overlay: Overlay? = nil, arrangement: [TopItem] = []
    ) {
        self.terminals = terminals
        self.groups = groups
        self.selected = selected
        self.sidebarVisible = sidebarVisible
        self.panelVisible = panelVisible
        self.panelTab = panelTab
        self.search = search
        self.overlay = overlay
        self.arrangement = arrangement
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
    case rename(TerminalID, String)
    case newGroup(String)
    case addToGroup(TerminalID, String)
    case moveTerminal(TerminalID, to: Landing)
    case moveGroup(String, before: TopItem?)
    /// A new terminal in the same split as this one, in its folder; it takes the selection.
    case splitRight(TerminalID)

    case renameGroup(String, String)
    case setGroupColor(String, GroupColor?)
    case setGroupFolder(String, String?)
    /// Pin the group (it stays as a head when its last terminal goes) or let it go again.
    case pinGroup(String)
    /// Take the group apart; its terminals stand where it stood.
    case ungroup(String)
    /// End every terminal in the group.
    case closeGroup(String)
    /// Take a terminal (and its split) out of its group: it stands right after it.
    case removeFromGroup(TerminalID)
    case newTerminalInGroup(String)
    /// A group holding the terminal and its split, named `name` or "Group N".
    case newGroupWithTerminal(TerminalID, name: String?)

    /// The first terminal shows beside the second ("Show beside…", a drop on the middle of a row).
    case joinSplit(TerminalID, with: TerminalID)
    case leaveSplit(TerminalID)
    case toggleForgetText(TerminalID)
    case toggleAgentHidden(TerminalID)
}
