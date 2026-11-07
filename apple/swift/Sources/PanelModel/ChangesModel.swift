// The Changes tab's data: the status and, per file, the diff text git printed for it.

public struct ChangesModel: Equatable, Sendable {
    public var files: [ChangedFile]
    public var diffs: [ChangedFile.ID: String]
    /// The branch the list is compared with ("vs main"); nil is the working tree against HEAD.
    public var compare: String?

    public init(files: [ChangedFile] = [], diffs: [ChangedFile.ID: String] = [:], compare: String? = nil) {
        self.files = files
        self.diffs = diffs
        self.compare = compare
    }
}
