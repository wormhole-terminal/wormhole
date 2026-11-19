// The facts of one terminal that the views draw from, as the core's snapshot has them (core `workspace::SidebarRow`).
// Its words (second line, VoiceOver label) are the core's too, on the row.

import Foundation

/// What a question about ending a terminal says of its agent: at work, waiting for an answer, or neither.
public enum RunState: Sendable, Equatable { case idle, busy, attention }

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
    /// The main working tree this terminal's folder is a linked worktree of (core `Place.linked_to`): the row's worktree mark.
    public var worktreeOf: String?
    /// The program running in front in the terminal (core `TerminalSummary.command`): what a close or a quit would end. Only the
    /// all-terminals list carries it; the rows' words say it too, but a close is decided from the core's facts.
    public var command: String?
    /// The terminal's folder as the core has it, the full path (core `TerminalSummary.cwd`, only in the all-terminals list):
    /// a folder whose name is not text arrives as a key of its bytes (`DiskName`), which the panel hands back to the core.
    public var path: String?
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
    /// The name the person (or a project's layout) gave it, which the program's name never changes; nil when it has none.
    public var customName: String?
    /// The program in front of the shell, as the core spells it, while one is (nil in the sample).
    public var runningCommand: String?
    /// Closing it asks first: the core's rule (`workspace::asks_before_closing`), never derived here.
    public var asksToClose = false
    /// Its status as told or detected, for a question's words; a finished, failed or ended one is `.idle` there.
    public var run: RunState = .idle
    /// The exit code of the command that failed, while its failure is the terminal's state (core `SidebarRow.failure_code`).
    public var failureCode: Int32?
    /// For a terminal in a submodule, whose it is ("in terminal"), and whether its repository has no branch (a detached
    /// HEAD): the title row's facts (core `TitleTerminal`).
    public var superproject: String?
    public var detached = false

    public init(
        id: TerminalID, title: String, folder: String? = nil, branch: String? = nil, activity: Activity = .idle,
        isPinned: Bool = false, unread: Bool = false, hasBell: Bool = false, agent: AgentID? = nil,
        message: String? = nil, splitID: Int? = nil, noteCount: Int = 0, changeCount: Int = 0,
        isAgentHidden: Bool = false, forgetsText: Bool = false, repo: String? = nil, isDirty: Bool = false,
        ahead: Int = 0, behind: Int = 0, repoFiles: Int? = nil, group: GroupID? = nil, groupName: String? = nil,
        isFolded: Bool = false, glyph: TileGlyph? = nil, worktreeOf: String? = nil,
        command: String? = nil
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
        self.worktreeOf = worktreeOf
        self.command = command
    }
}

extension TerminalSummary {
    /// The folder the panel's tabs read (the Files tab, the git tabs, the project): the core's full path where the summary
    /// has it, else the drawn folder with its `~` written out (the sample's and the rows').
    public var readPath: String? {
        path ?? folder.map { $0 == "~" || $0.hasPrefix("~/") ? HomeFolder.path + $0.dropFirst(1) : $0 }
    }
}
