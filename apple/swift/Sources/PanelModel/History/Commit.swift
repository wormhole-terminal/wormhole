// One row of the History log (`wormhole_core::git::Commit`): what the list shows of a commit. Times are Unix seconds,
// as git hands them over. The opened commit's block is `CommitDetails`.

public enum RefKind: Sendable { case head, branch, tag }

public struct Commit: Identifiable, Hashable, Sendable {
    public var sha: String
    public var subject: String
    public var author: String
    public var time: Int
    /// The first parent only: a log row carries no more (`CommitDetails.parents` has them all).
    public var parents: [String]
    public var refs: [(kind: RefKind, name: String)]
    public var merge: Bool

    public var id: String { sha }
    public var short: String { String(sha.prefix(7)) }

    public init(
        sha: String, subject: String, author: String, time: Int, parents: [String] = [],
        refs: [(kind: RefKind, name: String)] = [], merge: Bool = false
    ) {
        (self.sha, self.subject, self.author, self.time, self.parents, self.refs, self.merge) = (sha, subject, author, time, parents, refs, merge)
    }

    public static func == (lhs: Commit, rhs: Commit) -> Bool { lhs.sha == rhs.sha }
    public func hash(into hasher: inout Hasher) { hasher.combine(sha) }
}

public struct HistoryModel: Equatable, Sendable {
    public var commits: [Commit]
    /// The branch the log is of; its chip sits in the card's head.
    public var branch: String?

    public init(commits: [Commit] = [], branch: String? = nil) {
        self.commits = commits
        self.branch = branch
    }
}
