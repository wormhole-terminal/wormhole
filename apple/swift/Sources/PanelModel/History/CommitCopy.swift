// What the History tab puts on the clipboard for a commit (`features/git/history/copy.rs`): its hash, short hash,
// message, subject or author. The first three are in the log row; the message and the author's address need the
// commit's details, which are read first for a commit that was never opened.

public enum CommitCopy: CaseIterable, Sendable {
    case hash, shortHash, message, subject, author

    /// The text, or nil while what it needs (the details, for the message and the author) is not read yet.
    public func text(of commit: Commit, details: CommitDetails?) -> String? {
        switch self {
        case .hash: commit.sha
        case .shortHash: commit.short
        // What the heading shows: the message's first line once it is read, the log's subject until then.
        case .subject: details?.subject ?? commit.subject
        case .message: details?.message
        case .author: details?.authorLine
        }
    }

    public var needsDetails: Bool { self == .message || self == .author }
}
