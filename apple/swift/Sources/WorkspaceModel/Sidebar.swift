// The sidebar's list as the core's snapshot has it (core `workspace::SidebarSnapshot`): which rows, in which order, under
// which heading, each row with its finished words. The views draw it and decide nothing: pins, splits, ⌘ numbers, folds,
// search and wording are the core's (`view::sidebar`), so these values only carry them.

import Foundation

public struct SidebarRow: Sendable, Equatable, Identifiable {
    public var id: TerminalID { terminal.id }
    public var terminal: TerminalSummary
    /// ⌘1–⌘9 shortcut number, when the row has one, and its keycap text.
    public var number: Int?
    public var keycap: String?
    /// How the row belongs to its split; empty for a terminal in none.
    public var splitMarks: SplitMarks
    public var secondLine: SecondLine
    /// Its place among its section's rows, for VoiceOver.
    public var position: Int
    public var of: Int
    /// The whole VoiceOver label: state, detail, selection, place and shortcut.
    public var spoken: String

    public init(
        terminal: TerminalSummary, number: Int? = nil, keycap: String? = nil, splitMarks: SplitMarks = SplitMarks(),
        secondLine: SecondLine = SecondLine(text: ""), position: Int = 1, of: Int = 1, spoken: String = ""
    ) {
        self.terminal = terminal
        self.number = number
        self.keycap = keycap
        self.splitMarks = splitMarks
        self.secondLine = secondLine
        self.position = position
        self.of = of
        self.spoken = spoken
    }
}

/// A group's head (core `GroupHead`).
public struct GroupHead: Sendable, Equatable, Identifiable {
    public var id: GroupID
    public var name: String
    public var color: GroupColor?
    public var isPinned: Bool
    public var isFolded: Bool
    /// The folder the group stands for, `~`-shortened.
    public var folder: String?
    public var spoken: String
    /// A colour picked freely as `#rrggbb` (`color` is then nil: it is none of the nine names).
    public var customColor: String?
    /// The folder's full path, which `folder` shortens; set by the group's menu, it pins the group.
    public var folderPath: String?
    /// The group keeps only terminals that are in its folder.
    public var onlyInFolder: Bool

    public init(
        id: GroupID, name: String, color: GroupColor? = nil, isPinned: Bool = false, isFolded: Bool = false,
        folder: String? = nil, spoken: String = "", customColor: String? = nil, folderPath: String? = nil,
        onlyInFolder: Bool = false
    ) {
        self.id = id
        self.name = name
        self.color = color
        self.isPinned = isPinned
        self.isFolded = isFolded
        self.folder = folder
        self.spoken = spoken
        self.customColor = customColor
        self.folderPath = folderPath
        self.onlyInFolder = onlyInFolder
    }
}

public struct SidebarSection: Sendable, Equatable, Identifiable {
    /// The core's key: stable across frames, so the views animate a section as one thing.
    public var id: String
    /// nil: a run of terminals in no group.
    public var head: GroupHead?
    /// Empty while the group is folded; `terminalCount` still says what is folded away.
    public var rows: [SidebarRow]
    /// Terminals in the group that pass the search, folded away or not, for a head's count.
    public var terminalCount: Int
    /// Of those, the ones that need input or failed: a folded head shows them as its alert dot.
    public var alertCount: Int
    /// While the group is folded: one tile per member that passes the search, the loudest first.
    public var tiles: [MemberTile]
    /// The loudest alert among them, for the head's dot (`needsYou` or `failed`).
    public var alert: Urgency?
    /// Members working now: what to ask about before closing the group.
    public var workingCount: Int

    public var title: String? { head?.name }
    public var isCollapsed: Bool { head?.isFolded ?? false }
    public var isHeadless: Bool { head == nil }

    public init(
        id: String, head: GroupHead? = nil, rows: [SidebarRow], terminalCount: Int? = nil, alertCount: Int = 0,
        tiles: [MemberTile] = [], alert: Urgency? = nil, workingCount: Int = 0
    ) {
        self.id = id
        self.head = head
        self.rows = rows
        self.terminalCount = terminalCount ?? rows.count
        self.alertCount = alertCount
        self.tiles = tiles
        self.alert = alert
        self.workingCount = workingCount
    }
}

/// What the list is, with when to look again.
public struct SidebarSnapshot: Sendable, Equatable {
    public var sections: [SidebarSection]
    public var selected: TerminalID?
    /// How long until words that move with time ("Working · 3m") read differently: the one timer's due time.
    public var refreshIn: Duration?

    public init(sections: [SidebarSection], selected: TerminalID?, refreshIn: Duration? = nil) {
        self.sections = sections
        self.selected = selected
        self.refreshIn = refreshIn
    }
}

public enum Bracket: Sendable, Equatable {
    case start, mid, end
}

/// What a row shows of its split.
public struct SplitMarks: Sendable, Equatable {
    public var bracket: Bracket?
    /// A member of a split that isn't together in this list: the chain mark stands in for the bracket.
    public var chain = false
    /// Another member of the selected terminal's split: it is on screen beside it.
    public var onScreen = false
    /// How many terminals the split has, this one included; 0 outside a split.
    public var members = 0

    public init(bracket: Bracket? = nil, chain: Bool = false, onScreen: Bool = false, members: Int = 0) {
        self.bracket = bracket
        self.chain = chain
        self.onScreen = onScreen
        self.members = members
    }
}

/// What colour a second line takes, by meaning; the design system maps each to a token.
public enum SecondLineTone: Sendable, Equatable {
    case plain, attention, done, failed
}

/// A row's second line: its words, their tone, whether they are a command (code font; a path is in the interface font)
/// and what sits at the line's right.
public struct SecondLine: Sendable, Equatable {
    public var text: String
    public var tone: SecondLineTone
    public var isCommand: Bool
    public var trail: String?

    public init(text: String, tone: SecondLineTone = .plain, isCommand: Bool = false, trail: String? = nil) {
        self.text = text
        self.tone = tone
        self.isCommand = isCommand
        self.trail = trail
    }
}
