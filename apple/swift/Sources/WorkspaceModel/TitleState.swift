// The title row as data (core `TitleSnapshot`): which parts the row has before and after its flexible space and where its
// chevrons stand, the ways to draw the folder, and the path menu. The shell measures the texts and fits them.

import Foundation

/// A part of the row, by the key core and `config.json` spell it (`folder`, `branch_status`, `open_in`, ...).
public struct TitleSlot: Sendable, Equatable {
    public var part: String
    /// The model has something for it (the gauge, ports and the pull request come with their feeds).
    public var present: Bool
    public var chevronBefore: Bool

    public init(part: String, present: Bool, chevronBefore: Bool) { (self.part, self.present, self.chevronBefore) = (part, present, chevronBefore) }
}

/// A way to draw the folder: the parents, then the last folder.
public struct PathStepText: Sendable, Equatable {
    public var parents: String
    public var last: String

    public init(parents: String, last: String) { (self.parents, self.last) = (parents, last) }
}

/// A folder of the path menu (the folder's parents, nearest last).
public struct PathMenuFolder: Sendable, Equatable {
    public var path: String
    public var name: String

    public init(path: String, name: String) { (self.path, self.name) = (path, name) }
}

public struct TitleState: Sendable, Equatable {
    public var terminal: TerminalSummary?
    public var left: [TitleSlot]
    public var right: [TitleSlot]
    public var hidden: [String]
    public var pathSteps: [PathStepText]
    public var pathMenu: [PathMenuFolder]

    public init(
        terminal: TerminalSummary? = nil, left: [TitleSlot] = [], right: [TitleSlot] = [], hidden: [String] = [],
        pathSteps: [PathStepText] = [], pathMenu: [PathMenuFolder] = []
    ) {
        (self.terminal, self.left, self.right, self.hidden, self.pathSteps, self.pathMenu) = (terminal, left, right, hidden, pathSteps, pathMenu)
    }
}
