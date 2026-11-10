// Where the Changes and History tabs' data comes from: `git` through core's hardened `GitCmd` (read-only), reached
// by the FFI. Swift holds what is shown and asks for the rest on demand: a file's diff only when it is opened.

public struct GitFailure: Error, Equatable, Sendable {
    public var message: String
    public init(_ message: String) { self.message = message }
}

public protocol GitBackend: Sendable {
    /// Whether `folder` is in a repository (the tabs say "not a repository" otherwise).
    func isRepository(_ folder: String) async -> Bool
    /// The status of the working tree, or the diff stat against `rev` when comparing.
    func changes(in folder: String, comparedWith rev: Rev?) async throws -> [ChangedFile]
    /// The same, with the name of the compared branch when it no longer exists (the list is then against HEAD).
    func changesResult(in folder: String, comparedWith rev: Rev?) async throws -> ChangesResult
    /// One file's diff as the core parsed it: the working tree against HEAD, or against `rev`.
    func diff(of path: String, in folder: String, comparedWith rev: Rev?) async throws -> DiffFile
    /// The log of `rev` (HEAD when nil, or the stashes), limited to commits that touched `paths` when given.
    func log(in folder: String, of rev: Rev?, paths: [String]) async throws -> HistoryModel
    /// An opened commit as its block draws it: the message, who and when, the refs and its files with their counts.
    func details(of sha: String, in folder: String) async throws -> CommitDetails
    /// The opened commit's message for lines of about `lineChars` characters, and whether it leaves anything out
    /// (core's estimate of how the view wraps; nil for a commit that was not opened).
    func compactMessage(of sha: String, in folder: String, lineChars: Int) -> CommitDetails.Compact?
    /// One file's diff in a commit.
    func diff(of path: String, inCommit sha: String, folder: String) async throws -> DiffFile
    /// What the branch picker lists: branches and tags, and in History the stashes too.
    func revs(in folder: String, forHistory: Bool) async throws -> [Rev]
    /// Fires when something that matters to `git status` changed (the debounced watch of `git/watch`).
    func changes(in folder: String) -> AsyncStream<Void>
}

/// What a status read gave: the files, and the compared branch's name when it was deleted.
public struct ChangesResult: Sendable {
    public var files: [ChangedFile]
    public var compareMissing: String?

    public init(files: [ChangedFile], compareMissing: String? = nil) { (self.files, self.compareMissing) = (files, compareMissing) }
}

extension GitBackend {
    public func changesResult(in folder: String, comparedWith rev: Rev?) async throws -> ChangesResult {
        ChangesResult(files: try await changes(in: folder, comparedWith: rev))
    }
}
