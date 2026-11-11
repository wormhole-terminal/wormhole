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
    /// "Hide from Agents" and "Forget text" (the row menu's checks): the model keeps them, the shell enforces them.
    public var isAgentHidden: Bool
    public var forgetsText: Bool
    /// What git says of the terminal's folder (core `git_feed`): the repository's root, an unclean tree, how far the
    /// branch is from its upstream, the files the repository has changed.
    public var repo: String?
    public var isDirty: Bool
    public var ahead: Int
    public var behind: Int
    public var repoFiles: Int?
    /// Where the terminal stands in the list: its group (a folded one's members are listed only by `TerminalsSnapshot`).
    public var group: GroupID?
    public var groupName: String?
    public var isFolded: Bool
    /// The mark on its tile, when the core said (the sample leaves it out and the tile follows `activity`).
    public var glyph: TileGlyph?
    /// When it got into its state, on the core's clock (`WorkspaceBackend.waited`); nil in the sample.
    public var since: UInt64?
    /// The serial of its latest notification in its own words (0: none): a new one is news for the notifications even
    /// when nothing else of it moved.
    public var noticeSerial: UInt64 = 0
    /// The program in front of the shell, as the core spells it, while one is (nil in the sample).
    public var runningCommand: String?

    public init(
        id: TerminalID, title: String, folder: String? = nil, branch: String? = nil, activity: Activity = .idle,
        isPinned: Bool = false, unread: Bool = false, hasBell: Bool = false, agent: AgentID? = nil,
        message: String? = nil, splitID: Int? = nil, noteCount: Int = 0, changeCount: Int = 0,
        isAgentHidden: Bool = false, forgetsText: Bool = false, repo: String? = nil, isDirty: Bool = false,
        ahead: Int = 0, behind: Int = 0, repoFiles: Int? = nil, group: GroupID? = nil, groupName: String? = nil,
        isFolded: Bool = false, glyph: TileGlyph? = nil
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
        self.isAgentHidden = isAgentHidden
        self.forgetsText = forgetsText
        self.repo = repo
        self.isDirty = isDirty
        self.ahead = ahead
        self.behind = behind
        self.repoFiles = repoFiles
        self.group = group
        self.groupName = groupName
        self.isFolded = isFolded
        self.glyph = glyph
    }
}
