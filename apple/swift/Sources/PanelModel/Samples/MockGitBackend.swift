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

    public func diff(of path: String, in folder: String, comparedWith rev: Rev?) async throws -> String {
        PanelSamples.diffs[path] ?? ""
    }

    public func log(in folder: String, of rev: Rev?, paths: [String]) async throws -> HistoryModel {
        var shown = history
        if !paths.isEmpty { shown.commits = shown.commits.filter { $0.files.contains { paths.contains($0.path) } } }
        shown.branch = rev?.title ?? history.branch
        return shown
    }

    public func details(of sha: String, in folder: String) async throws -> Commit {
        guard let commit = history.commits.first(where: { $0.sha == sha }) else { throw GitFailure("No such commit") }
        return commit
    }

    public func diff(of path: String, inCommit sha: String, folder: String) async throws -> String {
        PanelSamples.diffs[path] ?? ""
    }

    public func revs(in folder: String, forHistory: Bool) async throws -> [Rev] {
        var revs = [Rev(name: "main"), Rev(name: "feature/panel-files"), Rev(name: "fix/login"), Rev(name: "v1.6.3", kind: .tag)]
        revs += history.commits.prefix(3).map { Rev(name: $0.sha, kind: .commit, detail: $0.subject) }
        if forHistory { revs.append(.stashes) }
        return revs
    }

    public func changes(in folder: String) -> AsyncStream<Void> { AsyncStream { $0.finish() } }
}
