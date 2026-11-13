// A file's diff as the core parsed it: what the diff view draws, with no git or text layout in it.

public struct DiffLine: Hashable, Sendable {
    public enum Kind: Sendable { case context, added, removed }

    public var kind: Kind
    public var oldNumber: Int?
    public var newNumber: Int?
    public var text: String
    /// The line of the new file the open icon opens the editor at: its own, or the nearest one for a removed line (core's rule).
    public var openLine: Int?
    /// Git says the file ends without a newline after this line.
    public var noNewline = false
    /// The syntax colours of `text`, filled in by `DiffHighlighting`; empty until then.
    public var spans: [SyntaxSpan] = []

    public init(kind: Kind, oldNumber: Int? = nil, newNumber: Int? = nil, text: String, noNewline: Bool = false, openLine: Int? = nil) {
        (self.kind, self.oldNumber, self.newNumber, self.text, self.noNewline) = (kind, oldNumber, newNumber, text, noNewline)
        self.openLine = openLine ?? newNumber
    }
}

public struct DiffHunk: Hashable, Sendable {
    /// The line as it is drawn: `@@ -12,6 +12,8 @@ fn section`.
    public var header: String
    public var lines: [DiffLine]

    public init(header: String, lines: [DiffLine]) { (self.header, self.lines) = (header, lines) }
}

/// What a column with nothing to show says (`features/git/diff/notes.rs`): why, a size or what a pointer stands for, and
/// the LFS object whole for copying.
public struct SideNote: Hashable, Sendable {
    public var text: String
    public var detail: String?
    public var oid: String?

    public init(text: String, detail: String? = nil, oid: String? = nil) { (self.text, self.detail, self.oid) = (text, detail, oid) }
}

/// The notes of a diff with no lines: one per column when split (old, new), one when unified.
public struct DiffNotes: Hashable, Sendable {
    public var split: [SideNote?] = []
    public var unified: SideNote?

    public init(split: [SideNote?] = [], unified: SideNote? = nil) { (self.split, self.unified) = (split, unified) }
}

/// A patch bigger than core loads unasked: its two lines of words, until the person says "Load diff".
public struct TooLarge: Hashable, Sendable {
    public var title: String
    public var detail: String

    public init(title: String, detail: String) { (self.title, self.detail) = (title, detail) }
}

public struct DiffFile: Hashable, Sendable {
    public var oldPath: String?
    public var newPath: String?
    public var isBinary = false
    public var hunks: [DiffHunk] = []
    /// Core's words where a diff has no lines: an LFS pointer, a file too large to load unasked, a submodule's commits.
    public var note: String?
    public var notes = DiffNotes()
    public var tooLarge: TooLarge?
    /// A submodule's card in place of lines: the commits between the two the parent records.
    public var submodule: SubmoduleCommits?
    /// "Mode 100644 → 100755" when only the mode changed or besides the lines.
    public var modeChange: String?
    /// More lines than core colours: shown plain, with the badge "No highlighting".
    public var noHighlight = false
    public var isLFS = false
    /// The longest line as drawn, in characters (tabs and marks spelled out, capped): measured once when the diff is read, so
    /// an unwrapped layout never measures lines again.
    public var widest = 0

    public var isNew: Bool { oldPath == nil }
    public var isDeleted: Bool { newPath == nil }
    public var isRenamed: Bool { oldPath != nil && newPath != nil && oldPath != newPath }
    public var path: String { newPath ?? oldPath ?? "" }

    public init(oldPath: String? = nil, newPath: String? = nil, isBinary: Bool = false, hunks: [DiffHunk] = [], note: String? = nil) {
        (self.oldPath, self.newPath, self.isBinary, self.hunks, self.note) = (oldPath, newPath, isBinary, hunks, note)
    }
}
