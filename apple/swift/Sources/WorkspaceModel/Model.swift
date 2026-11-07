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

/// One sidebar row: a terminal, as the list shows it. Only facts: the wording of the second line is
/// `SidebarWording`'s, from these and the clock.
public struct TerminalSummary: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var title: String
    public var subtitle: String?  // folder or running program
    public var branch: String?
    public var activity: Activity
    public var isPinned: Bool
    public var unread: Int
    /// The coding agent in the terminal; nil is a plain shell.
    public var agent: AgentID?
    /// What the agent says it is doing or asking ("Approve cargo test?"), when it says.
    public var message: String?
    /// When the terminal got into its `activity`.
    public var activitySince: Date?
    /// The command in front of a plain terminal and when it started: only one that ends by itself.
    public var runningCommand: String?
    public var runningSince: Date?
    /// The last command failed and the next hasn't started.
    public var lastFailure: CommandFailure?
    public var hasBell: Bool
    /// The terminal's folder as a path (`~` is fine): the quiet second line.
    public var folder: String?
    /// The split the terminal is in; only whether two terminals have the same one matters.
    public var splitID: Int?
    /// Notes and tasks still open (the row's count) and files changed (its "±N").
    public var noteCount: Int
    public var changeCount: Int

    public init(
        id: TerminalID, title: String, subtitle: String? = nil, branch: String? = nil,
        activity: Activity = .idle, isPinned: Bool = false, unread: Int = 0, agent: AgentID? = nil,
        message: String? = nil, activitySince: Date? = nil, runningCommand: String? = nil,
        runningSince: Date? = nil, lastFailure: CommandFailure? = nil, hasBell: Bool = false,
        folder: String? = nil, splitID: Int? = nil, noteCount: Int = 0, changeCount: Int = 0
    ) {
        self.id = id
        self.title = title
        self.subtitle = subtitle
        self.branch = branch
        self.activity = activity
        self.isPinned = isPinned
        self.unread = unread
        self.agent = agent
        self.message = message
        self.activitySince = activitySince
        self.runningCommand = runningCommand
        self.runningSince = runningSince
        self.lastFailure = lastFailure
        self.hasBell = hasBell
        self.folder = folder
        self.splitID = splitID
        self.noteCount = noteCount
        self.changeCount = changeCount
    }
}

/// A command that ended with a failing exit code.
public struct CommandFailure: Sendable, Equatable {
    public var command: String?
    public var exitCode: Int
    public var at: Date

    public init(command: String? = nil, exitCode: Int, at: Date) {
        self.command = command
        self.exitCode = exitCode
        self.at = at
    }
}

/// A named run of rows in the sidebar.
public struct TerminalGroup: Sendable, Equatable, Identifiable {
    public var id: String
    public var name: String
    public var members: [TerminalID]
    public var isCollapsed: Bool
    public var color: FolderColor?
    /// The folder the group stands for, as a path.
    public var folder: String?
    /// A pinned group stays as a head with no terminal in it.
    public var isPinned: Bool

    public init(
        id: String, name: String, members: [TerminalID], isCollapsed: Bool = false,
        color: FolderColor? = nil, folder: String? = nil, isPinned: Bool = false
    ) {
        self.id = id
        self.name = name
        self.members = members
        self.isCollapsed = isCollapsed
        self.color = color
        self.folder = folder
        self.isPinned = isPinned
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
}
