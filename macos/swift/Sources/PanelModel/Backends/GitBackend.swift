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
    /// One file's diff as the core parsed it: the working tree against HEAD, or against `rev`. `loadAnyway` asks again for a
    /// patch that was too large to load unasked.
    func diff(of path: String, in folder: String, comparedWith rev: Rev?, loadAnyway: Bool) async throws -> DiffFile
    /// The log of `rev` (HEAD when nil, or the stashes), limited to commits that touched `paths` when given.
    func log(in folder: String, of rev: Rev?, paths: [String]) async throws -> HistoryModel
    /// An opened commit as its block draws it: the message, who and when, the refs and its files with their counts.
    func details(of sha: String, in folder: String) async throws -> CommitDetails
    /// The opened commit's message for lines of about `lineChars` characters, and whether it leaves anything out
    /// (core's estimate of how the view wraps; nil for a commit that was not opened).
    func compactMessage(of sha: String, in folder: String, lineChars: Int) -> CommitDetails.Compact?
    /// One file's diff in a commit.
    func diff(of path: String, inCommit sha: String, folder: String, loadAnyway: Bool) async throws -> DiffFile
    /// What the branch picker lists: branches and tags, and in History the stashes too.
    func revs(in folder: String, forHistory: Bool) async throws -> [Rev]
    /// Fires when something that matters to `git status` changed (the debounced watch of `git/watch`).
    func changes(in folder: String) -> AsyncStream<Void>
    /// The repository root of `folder`, nil outside a repository. A requirement, so a call through `any GitBackend`
    /// reaches the real backend and not the default below.
    func root(of folder: String) async -> String?
    /// Whether commit `sha` is on `rev`: the palette's commit keeps the History tab's branch when it is.
    func isOn(_ sha: String, rev: Rev, in folder: String) async -> Bool
    /// The ones of `paths` that are regular files in the working tree now (the open icon is for those); nil from a
    /// backend that does not look, and the row then goes by the kind of change.
    func presentFiles(_ paths: [String], in folder: String) async -> Set<String>?
    /// How long until the earliest of `times` (Unix seconds) has an age that reads differently; nil when nothing
    /// will (no times, or a backend with a fixed clock).
    func nextAgeChange(of times: [Int]) -> Duration?
}

/// What a status read gave: the files, the compared branch's name when it was deleted, and the commit the list is
/// against (HEAD, or the merge-base with the compared branch), which an open diff is read again for when it moves.
public struct ChangesResult: Sendable {
    public var files: [ChangedFile]
    public var compareMissing: String?
    public var baseRev: String?

    public init(files: [ChangedFile], compareMissing: String? = nil, baseRev: String? = nil) {
        (self.files, self.compareMissing, self.baseRev) = (files, compareMissing, baseRev)
    }
}

extension GitBackend {
    public func changesResult(in folder: String, comparedWith rev: Rev?) async throws -> ChangesResult {
        ChangesResult(files: try await changes(in: folder, comparedWith: rev))
    }
}

extension GitBackend {
    /// The repository root of `folder`, nil outside a repository (and from a backend that knows none).
    public func root(of folder: String) async -> String? { nil }

    /// A backend that cannot ask says yes, so the branch stays.
    public func isOn(_ sha: String, rev: Rev, in folder: String) async -> Bool { true }
    public func presentFiles(_ paths: [String], in folder: String) async -> Set<String>? { nil }
    public func nextAgeChange(of times: [Int]) -> Duration? { nil }
}
