// A parsed unified diff: what the diff view draws, with no git or text layout in it.

public struct DiffLine: Hashable, Sendable {
    public enum Kind: Sendable { case context, added, removed, noNewline }

    public var kind: Kind
    public var oldNumber: Int?
    public var newNumber: Int?
    public var text: String
    /// The syntax colours of `text`, filled in by `DiffHighlighting`; empty until then.
    public var spans: [SyntaxSpan] = []
}

public struct DiffHunk: Hashable, Sendable {
    /// Where in the file the hunk starts, as git wrote it: `-12,6 +12,8`.
    public var range: String
    /// The enclosing function or heading git found for the hunk; empty when it found none.
    public var section: String
    public var lines: [DiffLine]

    public var added: Int { lines.count { $0.kind == .added } }
    public var removed: Int { lines.count { $0.kind == .removed } }
}

public struct DiffFile: Hashable, Sendable {
    public var oldPath: String?
    public var newPath: String?
    public var isBinary = false
    public var hunks: [DiffHunk] = []

    public var isNew: Bool { oldPath == nil }
    public var isDeleted: Bool { newPath == nil }
    public var isRenamed: Bool { oldPath != nil && newPath != nil && oldPath != newPath }
    public var path: String { newPath ?? oldPath ?? "" }
}
