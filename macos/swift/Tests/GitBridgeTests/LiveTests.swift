// The panel's git against the real Rust core and real throwaway repositories (the FFI must be linked).

import Foundation
import PanelModel
import Testing

@testable import GitBridge

@Suite struct LiveTests {
    @Test func changesDiffAndTheRevPickerOverARealRepository() async throws {
        let root = try Repo.make(["a.txt": "1\n2\n3\n", "old.txt": "keep\nthis\nfile\nplease\n"])
        defer { Repo.cleanup(root) }
        try Repo.write(root, "a.txt", "1\nTWO\n3\n4\n")
        try Repo.write(root, "fresh.txt", "hello\n")
        try Repo.git(root, "mv", "old.txt", "new.txt")
        try Repo.git(root, "tag", "v1")
        let git = try Repo.backends().git
        let folder = root.path

        let (inside, outside) = await (git.isRepository(folder), git.isRepository(NSTemporaryDirectory() + "no-such-folder-\(UUID().uuidString)"))
        #expect(inside && !outside)
        let state = try await git.state(in: folder, comparedWith: nil)
        #expect(state.branch == "main" && state.dirty)
        let kinds = Dictionary(uniqueKeysWithValues: state.files.map { ($0.path, $0.kind) })
        #expect(kinds == ["a.txt": .modified, "fresh.txt": .untracked, "new.txt": .renamed])
        let modified = try #require(state.files.first { $0.path == "a.txt" })
        #expect(modified.lines == .counts(added: 2, removed: 1))

        let parsed = try await git.diff(of: "a.txt", in: folder, comparedWith: nil, loadAnyway: false)
        let counts = parsed.hunks[0].lines.reduce(into: (added: 0, removed: 0)) { $0.added += $1.kind == .added ? 1 : 0; $0.removed += $1.kind == .removed ? 1 : 0 }
        #expect(parsed.path == "a.txt" && parsed.hunks.count == 1 && counts == (2, 1))
        let renamed = try await git.diff(of: "new.txt", in: folder, comparedWith: nil, loadAnyway: false)
        #expect(renamed.isRenamed && renamed.oldPath == "old.txt")

        let revs = try await git.revs(in: folder, forHistory: true)
        #expect(revs.contains(Rev(name: "main")) && revs.contains(Rev(name: "v1", kind: .tag)))
        #expect(revs.contains { $0.kind == .commit && $0.detail == "initial" })
        let compared = try await git.changes(in: folder, comparedWith: Rev(name: "v1", kind: .tag))
        #expect(compared.contains { $0.path == "fresh.txt" })
    }

    @Test func historyLogDetailsAndACommitsDiff() async throws {
        let root = try Repo.make()
        defer { Repo.cleanup(root) }
        try Repo.write(root, "b.txt", "new\n")
        try Repo.git(root, "add", "-A")
        try Repo.git(root, "commit", "-q", "-m", "Add b", "-m", "Because it is needed.")
        let git = try Repo.backends().git

        let log = try await git.log(in: root.path, of: nil, paths: [])
        #expect(log.branch == "main" && log.commits.map(\.subject) == ["Add b", "initial"])
        #expect(log.commits[0].refs.contains { $0.kind == .head })
        let only = try await git.log(in: root.path, of: nil, paths: ["a.txt"])
        #expect(only.commits.map(\.subject) == ["initial"])

        let opened = try await git.details(of: log.commits[0].sha, in: root.path)
        #expect(opened.whole == "Because it is needed." && opened.files.map(\.path) == ["b.txt"] && opened.authorLine.hasPrefix("Test <"))
        let compact = try #require(git.compactMessage(of: opened.sha, in: root.path, lineChars: 80))
        #expect(compact == CommitDetails.Compact(text: "Because it is needed.", more: false))
        let diff = try await git.diff(of: "b.txt", inCommit: opened.sha, folder: root.path, loadAnyway: false)
        #expect(diff.isNew)

        await #expect(throws: GitFailure.self) { try await git.log(in: root.path, of: Rev(name: "no-such-branch"), paths: []) }
    }

    /// Whether `stream` gives an element within `seconds`.
    private func fires(_ stream: AsyncStream<Void>, within seconds: Double) async -> Bool {
        await withTaskGroup(of: Bool.self) { group in
            group.addTask {
                var iterator = stream.makeAsyncIterator()
                return await iterator.next() != nil
            }
            group.addTask {
                try? await Task.sleep(for: .seconds(seconds))
                return false
            }
            let first = await group.next() ?? false
            group.cancelAll()
            return first
        }
    }

    @Test func theChangeStreamFiresAfterAnEdit() async throws {
        let root = try Repo.make()
        defer { Repo.cleanup(root) }
        let stream = try Repo.backends().git.changes(in: root.path)
        let edit = Task {
            try await Task.sleep(for: .seconds(1))  // the watch starts with the stream
            try Repo.write(root, "a.txt", "edited\n")
        }
        let fired = await fires(stream, within: 10)
        _ = try await edit.value
        #expect(fired)
    }

    @Test func aFolderThatBecomesARepositoryIsReadWhenItDoes() async throws {
        // A folder inside a repository is in it, so this one is made outside any.
        let outside = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("wormhole-git-bridge-plain-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
        defer { Repo.cleanup(outside) }
        let stream = try Repo.backends().git.changes(in: outside.path)
        let initialising = Task {
            try await Task.sleep(for: .seconds(1))
            try Repo.git(outside, "init", "-q")
        }
        let fired = await fires(stream, within: 15)
        _ = try await initialising.value
        #expect(fired)
    }

    @Test func aWorktreeIsPlannedMadeFoundAndRemovedAndAnUntrustedProjectBlocksIt() async throws {
        let root = try Repo.make()
        defer { Repo.cleanup(root) }
        let opened = Opened()
        let backends = try Repo.backends(created: { made, environment, _ in await opened.add(made, environment) })
        let worktrees = backends.worktrees
        let folder = root.path

        let problem = await worktrees.problem(making: folder)
        #expect(problem == nil)
        let name = root.lastPathComponent
        let expected = root.deletingLastPathComponent().appendingPathComponent("\(name)-worktrees/fix-login").path
        let plan = await worktrees.plan(branch: "fix/login", repo: folder)
        #expect(plan == .ready(branch: "fix/login", folder: expected))
        let refused = await worktrees.plan(branch: "no spaces", repo: folder)
        #expect(refused == .refused("A branch name can't contain spaces"))

        let creation = await worktrees.create(branch: "fix/login", repo: folder, command: nil)
        #expect(creation == nil)
        defer { try? FileManager.default.removeItem(atPath: expected) }
        let made = try #require(await worktrees.made(at: expected))
        #expect(made == MadeWorktree(repo: folder, branch: "fix/login", folder: expected))
        let (openedMade, environment) = await (opened.made, opened.environment)
        #expect(openedMade == [made] && environment["WORMHOLE_PORT"] != nil)
        #expect(FileManager.default.fileExists(atPath: expected + "/a.txt"))

        // Changes keep it; nothing is forced.
        try Repo.write(URL(fileURLWithPath: expected), "dirty.txt", "x")
        let kept = await worktrees.remove(made)
        #expect(kept?.contains("uncommitted") == true)
        try FileManager.default.removeItem(atPath: expected + "/dirty.txt")
        let removal = await worktrees.remove(made)
        #expect(removal == nil)
        #expect(!FileManager.default.fileExists(atPath: expected))

        // A project that is not trusted: nothing is written.
        let blocked = try Repo.backends(allowed: { _ in "Trust the project first" }).worktrees
        let (why, refusal) = await (blocked.problem(making: folder), blocked.create(branch: "other", repo: folder, command: nil))
        #expect(why == "Trust the project first" && refusal == "Trust the project first")
        #expect(!FileManager.default.fileExists(atPath: root.deletingLastPathComponent().appendingPathComponent("\(name)-worktrees/other").path))
    }
}

private actor Opened {
    var made: [MadeWorktree] = []
    var environment: [String: String] = [:]
    func add(_ worktree: MadeWorktree, _ env: [String: String]) { made.append(worktree); environment = env }
}
