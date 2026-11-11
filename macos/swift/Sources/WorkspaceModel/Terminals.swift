// Every terminal that exists, listed or folded away (core `TerminalsSnapshot`): what the palette, the status bar and the
// "move to" menus read, because the sidebar's own rows leave out a folded group's. The counts are the core's too.

import Foundation

/// How many terminals are in each state of the status bar (core `TerminalCounts`).
public struct TerminalCounts: Sendable, Equatable {
    public var needsYou = 0
    public var working = 0
    public var finished = 0
    public var failed = 0
    public var quiet = 0

    public init(needsYou: Int = 0, working: Int = 0, finished: Int = 0, failed: Int = 0, quiet: Int = 0) {
        (self.needsYou, self.working, self.finished, self.failed, self.quiet) = (needsYou, working, finished, failed, quiet)
    }
}

public struct TerminalsSnapshot: Sendable, Equatable {
    /// In the list's order, folded groups' terminals too (`TerminalSummary.isFolded`).
    public var terminals: [TerminalSummary]
    public var counts: TerminalCounts

    public init(terminals: [TerminalSummary] = [], counts: TerminalCounts = TerminalCounts()) {
        (self.terminals, self.counts) = (terminals, counts)
    }
}
