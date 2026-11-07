// The values a sidebar row and a group head are made of.

import Foundation

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
    /// The repository the changes are counted in: only the topmost row of one repository shows the count.
    public var repoID: String?
    /// The main repository this terminal's folder is a worktree of (the row's worktree mark).
    public var worktreeOf: String?
    /// Hidden from the programs that control Wormhole: the row wears a lock.
    public var isHiddenFromAgents: Bool
    /// Its text isn't kept for a restart ("Don't keep this terminal's text").
    public var forgetsText: Bool

    public init(
        id: TerminalID, title: String, subtitle: String? = nil, branch: String? = nil,
        activity: Activity = .idle, isPinned: Bool = false, unread: Int = 0, agent: AgentID? = nil,
        message: String? = nil, activitySince: Date? = nil, runningCommand: String? = nil,
        runningSince: Date? = nil, lastFailure: CommandFailure? = nil, hasBell: Bool = false,
        folder: String? = nil, splitID: Int? = nil, noteCount: Int = 0, changeCount: Int = 0,
        repoID: String? = nil, worktreeOf: String? = nil, isHiddenFromAgents: Bool = false, forgetsText: Bool = false
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
        self.repoID = repoID
        self.worktreeOf = worktreeOf
        self.isHiddenFromAgents = isHiddenFromAgents
        self.forgetsText = forgetsText
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
    public var color: GroupColor?
    /// The folder the group stands for, as a path.
    public var folder: String?
    /// A pinned group stays as a head with no terminal in it.
    public var isPinned: Bool

    public init(
        id: String, name: String, members: [TerminalID], isCollapsed: Bool = false,
        color: GroupColor? = nil, folder: String? = nil, isPinned: Bool = false
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
