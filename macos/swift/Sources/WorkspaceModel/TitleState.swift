// The title row as data (core `TitleSnapshot`): which parts the row has before and after its flexible space and where its
// chevrons stand, the ways to draw the folder, and the path menu. The shell measures the texts and fits them.

import Foundation

/// A part of the row, by the key core and `config.json` spell it (`folder`, `branch_status`, `open_in`, ...).
public struct TitleSlot: Sendable, Equatable {
    public var part: String
    /// The model has something for it (a branch needs a repository, ports a listener, the pull request one `gh` found).
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

/// The branch's pull request and its checks (core `git::pr`), with how VoiceOver says it (core's `PullRequest::words`).
public struct PullRequestFact: Sendable, Equatable {
    public enum State: Sendable, Equatable { case open, merged, closed }
    public enum Checks: Sendable, Equatable { case passing, failing, pending }

    public var number: Int
    public var url: URL?
    public var state: State
    public var draft: Bool
    public var checks: Checks?
    public var words: String

    public init(
        number: Int, url: URL? = nil, state: State = .open, draft: Bool = false, checks: Checks? = nil, words: String = ""
    ) {
        (self.number, self.url, self.state, self.draft, self.checks, self.words) = (number, url, state, draft, checks, words)
    }

    public var label: String { "#\(number)" }
    /// Checks are only drawn (and said) of an open one.
    public var openChecks: Checks? { state == .open ? checks : nil }
}

public struct TitleState: Sendable, Equatable {
    public var terminal: TerminalSummary?
    public var left: [TitleSlot]
    public var right: [TitleSlot]
    public var hidden: [String]
    public var pathSteps: [PathStepText]
    public var pathMenu: [PathMenuFolder]
    /// The ports the terminal's programs listen on, lowest first, and its branch's pull request, as the core looked them up.
    public var ports: [UInt16]
    public var pullRequest: PullRequestFact?

    public init(
        terminal: TerminalSummary? = nil, left: [TitleSlot] = [], right: [TitleSlot] = [], hidden: [String] = [],
        pathSteps: [PathStepText] = [], pathMenu: [PathMenuFolder] = [], ports: [UInt16] = [], pullRequest: PullRequestFact? = nil
    ) {
        (self.terminal, self.left, self.right, self.hidden, self.pathSteps, self.pathMenu) = (terminal, left, right, hidden, pathSteps, pathMenu)
        (self.ports, self.pullRequest) = (ports, pullRequest)
    }
}
