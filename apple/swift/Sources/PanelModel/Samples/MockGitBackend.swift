// A `GitBackend` over the sample data, for previews, screenshots and tests. `delay` makes a slow repository.

import Foundation

public struct MockGitBackend: GitBackend {
    public var files: [ChangedFile]
    public var history: HistoryModel

    public init(files: [ChangedFile] = PanelSamples.files, history: HistoryModel = PanelSamples.history) {
        self.files = files
        self.history = history
    }

    public func isRepository(_ folder: String) async -> Bool { true }

    public func changes(in folder: String, comparedWith rev: Rev?) async throws -> [ChangedFile] { files }

    public func diff(of path: String, in folder: String, comparedWith rev: Rev?, loadAnyway: Bool) async throws -> DiffFile {
        PanelSamples.diffs[path] ?? DiffFile(newPath: path)
    }

    public func log(in folder: String, of rev: Rev?, paths: [String]) async throws -> HistoryModel {
        var shown = history
        if !paths.isEmpty { shown.commits = PanelSamples.commits(touching: paths) }
        shown.branch = rev?.title ?? history.branch
        return shown
    }

    public func details(of sha: String, in folder: String) async throws -> CommitDetails {
        guard let details = PanelSamples.details(of: sha) else { throw GitFailure("No such commit") }
        return details
    }

    /// Every sample message is short enough to show whole.
    public func compactMessage(of sha: String, in folder: String, lineChars: Int) -> CommitDetails.Compact? {
        PanelSamples.details(of: sha).map { .init(text: $0.whole, more: false) }
    }

    public func diff(of path: String, inCommit sha: String, folder: String, loadAnyway: Bool) async throws -> DiffFile {
        PanelSamples.diffs[path] ?? DiffFile(newPath: path)
    }

    public func revs(in folder: String, forHistory: Bool) async throws -> [Rev] {
        var revs = [Rev(name: "main"), Rev(name: "feature/panel-files"), Rev(name: "fix/login"), Rev(name: "v1.6.3", kind: .tag)]
        revs += history.commits.prefix(3).map { Rev(name: $0.sha, kind: .commit, detail: $0.subject) }
        if forHistory { revs.append(.stashes) }
        return revs
    }

    public func changes(in folder: String) -> AsyncStream<Void> { AsyncStream { $0.finish() } }
}
