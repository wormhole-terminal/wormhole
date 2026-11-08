// `GitBackend` over the Rust core's git domain. Every call is a blocking FFI call run on `GitQueue`; what a
// repository says is hostile input and core has capped it (output size, timeouts, patch size) before it
// arrives here. Read-only: nothing in this file writes to a repository.

import PanelModel
import WormholeBindings

public struct RealGitBackend: GitBackend {
    let git: Git

    init(_ git: Git) { self.git = git }

    /// The setting `size_units` changed: sizes in a diff's words are KiB rather than KB.
    public func setBinarySizeUnits(_ binary: Bool) { git.setBinaryUnits(binary: binary) }

    public func isRepository(_ folder: String) async -> Bool {
        await GitQueue.run { [git] in git.isRepository(folder: folder) }
    }

    /// The status, or against `rev`: the changes plus what the title row says (branch, ahead and behind).
    public func state(in folder: String, comparedWith rev: Rev?) async throws -> GitRepoState {
        GitRepoState(try await GitQueue.run { [git] in try git.status(folder: folder, compare: rev?.name) })
    }

    public func changes(in folder: String, comparedWith rev: Rev?) async throws -> [ChangedFile] {
        try await state(in: folder, comparedWith: rev).files
    }

    public func diff(of path: String, in folder: String, comparedWith rev: Rev?) async throws -> DiffFile {
        try await GitQueue.run { [git] in
            try git.diff(folder: folder, path: path, compare: rev?.name, loadAnyway: false).file()
        }
    }

    public func log(in folder: String, of rev: Rev?, paths: [String]) async throws -> HistoryModel {
        let log = try await GitQueue.run { [git] in try git.log(folder: folder, rev: rev.map(GitRev.init), paths: paths) }
        return HistoryModel(commits: log.commits.map(Commit.init), branch: rev?.title ?? log.branch)
    }

    public func details(of sha: String, in folder: String) async throws -> CommitDetails {
        CommitDetails(try await GitQueue.run { [git] in try git.commitDetails(folder: folder, sha: sha) })
    }

    /// Asked while the block draws, so it does no git: core keeps the message of every commit it opened.
    public func compactMessage(of sha: String, in folder: String, lineChars: Int) -> CommitDetails.Compact? {
        git.compactMessage(folder: folder, sha: sha, lineChars: UInt32(clamping: lineChars)).map { .init(text: $0.text, more: $0.more) }
    }

    public func diff(of path: String, inCommit sha: String, folder: String) async throws -> DiffFile {
        try await GitQueue.run { [git] in
            try git.commitDiff(folder: folder, sha: sha, path: path, loadAnyway: false).file()
        }
    }

    public func revs(in folder: String, forHistory: Bool) async throws -> [Rev] {
        try await GitQueue.run { [git] in try git.revs(folder: folder, forHistory: forHistory) }.map(Rev.init)
    }

    public func changes(in folder: String) -> AsyncStream<Void> { RepoChanges.stream(git: git, folder: folder) }
}
