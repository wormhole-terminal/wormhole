// A file's diff as the core parsed it: what the diff view draws, with no git or text layout in it.

public struct DiffLine: Hashable, Sendable {
    public enum Kind: Sendable { case context, added, removed }

    public var kind: Kind
    public var oldNumber: Int?
    public var newNumber: Int?
    public var text: String
    /// Git says the file ends without a newline after this line.
    public var noNewline = false
    /// The syntax colours of `text`, filled in by `DiffHighlighting`; empty until then.
    public var spans: [SyntaxSpan] = []

    public init(kind: Kind, oldNumber: Int? = nil, newNumber: Int? = nil, text: String, noNewline: Bool = false) {
        (self.kind, self.oldNumber, self.newNumber, self.text, self.noNewline) = (kind, oldNumber, newNumber, text, noNewline)
    }
}

public struct DiffHunk: Hashable, Sendable {
    /// The line as it is drawn: `@@ -12,6 +12,8 @@ fn section`.
    public var header: String
    public var lines: [DiffLine]

    public init(header: String, lines: [DiffLine]) { (self.header, self.lines) = (header, lines) }
}

public struct DiffFile: Hashable, Sendable {
    public var oldPath: String?
    public var newPath: String?
    public var isBinary = false
    public var hunks: [DiffHunk] = []
    /// Core's words where a diff has no lines: an LFS pointer, a file too large to load unasked, a submodule's commits.
    public var note: String?

    public var isNew: Bool { oldPath == nil }
    public var isDeleted: Bool { newPath == nil }
    public var isRenamed: Bool { oldPath != nil && newPath != nil && oldPath != newPath }
    public var path: String { newPath ?? oldPath ?? "" }

    public init(oldPath: String? = nil, newPath: String? = nil, isBinary: Bool = false, hunks: [DiffHunk] = [], note: String? = nil) {
        (self.oldPath, self.newPath, self.isBinary, self.hunks, self.note) = (oldPath, newPath, isBinary, hunks, note)
    }
}
