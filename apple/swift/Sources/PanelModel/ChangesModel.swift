// The Changes tab's data: the files and, per file, the diff text git printed for it.

public struct ChangesModel: Equatable, Sendable {
    public var files: [ChangedFile]
    public var diffs: [ChangedFile.ID: String]

    public init(files: [ChangedFile] = [], diffs: [ChangedFile.ID: String] = [:]) {
        self.files = files
        self.diffs = diffs
    }

    public var staged: [ChangedFile] { files.filter(\.staged) }
    public var unstaged: [ChangedFile] { files.filter { !$0.staged } }

    /// The order the list is drawn in, which is the order the arrow keys walk.
    public var listed: [ChangedFile] { staged + unstaged }
}
