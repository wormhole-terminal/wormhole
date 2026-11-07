// The facts of one terminal that the views draw from, as the core's snapshot has them (core `workspace::SidebarRow`).
// Its words (second line, VoiceOver label) are the core's too, on the row.

import Foundation

public struct TerminalSummary: Sendable, Equatable, Identifiable {
    public var id: TerminalID
    public var title: String
    /// The terminal's folder as a path, `~`-shortened by the core: the quiet second line.
    public var folder: String?
    public var branch: String?
    public var activity: Activity
    public var isPinned: Bool
    /// Output nobody has seen while another terminal was selected.
    public var unread: Bool
    public var hasBell: Bool
    /// The coding agent in the terminal; nil is a plain shell.
    public var agent: AgentID?
    /// What the agent says it is doing or asking ("Approve cargo test?"), when it says.
    public var message: String?
    /// The split the terminal is in; only whether two terminals have the same one matters.
    public var splitID: Int?
    /// Notes and tasks still open (the row's count) and files changed (its "±N").
    public var noteCount: Int
    public var changeCount: Int

    public init(
        id: TerminalID, title: String, folder: String? = nil, branch: String? = nil, activity: Activity = .idle,
        isPinned: Bool = false, unread: Bool = false, hasBell: Bool = false, agent: AgentID? = nil,
        message: String? = nil, splitID: Int? = nil, noteCount: Int = 0, changeCount: Int = 0
    ) {
        self.id = id
        self.title = title
        self.folder = folder
        self.branch = branch
        self.activity = activity
        self.isPinned = isPinned
        self.unread = unread
        self.hasBell = hasBell
        self.agent = agent
        self.message = message
        self.splitID = splitID
        self.noteCount = noteCount
        self.changeCount = changeCount
    }
}
