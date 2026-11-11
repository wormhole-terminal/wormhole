// The Changes tab's data: the status. A file's diff text is asked of the backend only when the file is opened.

public struct ChangesModel: Equatable, Sendable {
    public var files: [ChangedFile]
    /// What the list is compared with ("vs main"); nil is the working tree against HEAD.
    public var compare: Rev?

    public init(files: [ChangedFile] = [], compare: Rev? = nil) {
        self.files = files
        self.compare = compare
    }
}
