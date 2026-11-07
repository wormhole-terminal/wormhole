// One commit of the History tab: what the log lists, and what the opened commit's block shows of it
// (`wormhole_core::git::Commit` and `CommitDetails`). Times are Unix seconds, as git hands them over.

import Foundation

public struct Signature: Hashable, Sendable {
    public var name: String
    public var email: String
    public var time: Int
    /// Offset from UTC in seconds, where the person was when they wrote it.
    public var zone: Int

    public init(name: String, email: String = "", time: Int, zone: Int = 0) {
        self.name = name
        self.email = email
        self.time = time
        self.zone = zone
    }

    /// "2026-10-20 15:45 +02:00", the form the details give.
    public var stamp: String {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(secondsFromGMT: zone) ?? .gmt
        let parts = calendar.dateComponents([.year, .month, .day, .hour, .minute], from: Date(timeIntervalSince1970: TimeInterval(time)))
        let sign = zone < 0 ? "-" : "+"
        let offset = String(format: "%@%02d:%02d", sign, abs(zone) / 3600, abs(zone) % 3600 / 60)
        return String(format: "%04d-%02d-%02d %02d:%02d ", parts.year ?? 0, parts.month ?? 0, parts.day ?? 0, parts.hour ?? 0, parts.minute ?? 0) + offset
    }
}

public enum RefKind: Sendable { case head, branch, tag }

public struct Commit: Identifiable, Hashable, Sendable {
    public var sha: String
    public var subject: String
    public var body: String
    public var author: Signature
    public var parents: [String]
    public var refs: [(kind: RefKind, name: String)]
    public var files: [ChangedFile]

    public var id: String { sha }
    public var short: String { String(sha.prefix(7)) }
    public var isMerge: Bool { parents.count > 1 }

    public init(
        sha: String, subject: String, body: String = "", author: Signature, parents: [String] = [],
        refs: [(kind: RefKind, name: String)] = [], files: [ChangedFile] = []
    ) {
        self.sha = sha
        self.subject = subject
        self.body = body
        self.author = author
        self.parents = parents
        self.refs = refs
        self.files = files
    }

    public static func == (lhs: Commit, rhs: Commit) -> Bool { lhs.sha == rhs.sha && lhs.files == rhs.files }
    public func hash(into hasher: inout Hasher) { hasher.combine(sha) }

    /// "2 files +4 −1", the numbers in the file rows' colours.
    public var changeSummary: (files: String, added: String, removed: String) {
        let counts = files.reduce((0, 0)) { ($0.0 + ($1.added ?? 0), $0.1 + ($1.removed ?? 0)) }
        return ("\(files.count) file\(files.count == 1 ? "" : "s")", "+\(counts.0)", "\u{2212}\(counts.1)")
    }
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
