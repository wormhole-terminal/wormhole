// What the title row says about the terminal it is for (`window/title/mod.rs` `Facts`): plain values the
// window gathers once per frame from the session, git and the extras' lookups.

import Foundation
import WorkspaceModel

/// The agent's model and how full its context window is (core `usage::Gauge`).
public struct GaugeFact: Sendable, Equatable {
    public var model: String?
    /// The share used, 0 to 100.
    public var context: Double?

    public init(model: String? = nil, context: Double? = nil) { (self.model, self.context) = (model, context) }

    public var percent: Int? { context.map { Int($0.rounded()) } }
    /// From this share used on the meter turns to the warning colour: early enough to compact.
    public var warns: Bool { (context ?? 0) >= 80 }

    /// "Opus 4.5, 41% of the context window used".
    public var words: String {
        let used = percent.map { "\($0)% of the context window used" }
        return [model, used].compactMap { $0 }.joined(separator: ", ")
    }
}

/// The branch's pull request and its checks (core `git::pr`).
public struct PullRequestFact: Sendable, Equatable {
    public enum State: Sendable, Equatable { case open, merged, closed }
    public enum Checks: Sendable, Equatable { case passing, failing, pending }

    public var number: Int
    public var url: URL?
    public var state: State
    public var draft: Bool
    public var checks: Checks?

    public init(number: Int, url: URL? = nil, state: State = .open, draft: Bool = false, checks: Checks? = nil) {
        (self.number, self.url, self.state, self.draft, self.checks) = (number, url, state, draft, checks)
    }

    public var label: String { "#\(number)" }
    /// Checks are only drawn (and said) of an open one.
    public var openChecks: Checks? { state == .open ? checks : nil }

    /// "Pull request #142, open, draft, checks failing".
    public var words: String {
        var words = "Pull request #\(number), \(state == .open ? "open" : state == .merged ? "merged" : "closed")"
        if state == .open {
            let checkWords = checks.map { $0 == .passing ? "checks passing" : $0 == .failing ? "checks failing" : "checks running" }
            words += ([draft ? "draft" : nil, checkWords].compactMap { $0 }).map { ", " + $0 }.joined()
        }
        return words
    }
}

public struct TitleFacts: Sendable, Equatable {
    public var terminalID: Int
    /// Where the terminal is, and how that reads (`~/Projects/tube`).
    public var folder: String?
    public var shownFolder: String?
    public var activity: Activity
    public var agent: AgentID?
    /// The terminal's name: its title, else what runs in it.
    public var label: String
    /// How it is doing after the name ("working · 4m"); nothing while it is idle.
    public var words: String?
    /// The folder's branch: `.some(nil)` is a detached HEAD, nil is no repository.
    public var branch: String??
    /// For a terminal in a submodule, whose it is ("in terminal").
    public var superproject: String?
    public var branchStatus: BranchStatus?
    public var gauge: GaugeFact?
    public var ports: [UInt16]
    public var pullRequest: PullRequestFact?
    public var row: TitleRow
    /// Hidden from the programs that control Wormhole: the name wears a lock.
    public var hiddenFromAgents: Bool

    public init(
        terminalID: Int = 0, folder: String? = nil, shownFolder: String? = nil, activity: Activity = .idle,
        agent: AgentID? = nil, label: String, words: String? = nil, branch: String?? = nil, superproject: String? = nil,
        branchStatus: BranchStatus? = nil, gauge: GaugeFact? = nil, ports: [UInt16] = [],
        pullRequest: PullRequestFact? = nil, row: TitleRow = .default, hiddenFromAgents: Bool = false
    ) {
        (self.terminalID, self.folder, self.shownFolder, self.activity, self.agent) = (terminalID, folder, shownFolder, activity, agent)
        (self.label, self.words, self.branch, self.superproject) = (label, words, branch, superproject)
        (self.branchStatus, self.gauge, self.ports, self.pullRequest) = (branchStatus, gauge, ports, pullRequest)
        (self.row, self.hiddenFromAgents) = (row, hiddenFromAgents)
    }
}
