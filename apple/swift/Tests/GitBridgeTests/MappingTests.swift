// The records as the panel's values, over canned records: no repository, no FFI call.

import PanelModel
import Testing
import WormholeBindings

@testable import GitBridge

@Suite struct MappingTests {
    @Test func aChangeKeepsItsCountsItsRenameAndASubmodulesWords() {
        let file = ChangedFile(
            GitChange(
                path: "src/b.rs", originalPath: "src/a.rs", kind: .renamed, added: 3, removed: 1, lfs: false, submodule: nil))
        #expect((file.kind, file.added, file.removed, file.originalPath) == (.renamed, 3, 1, "src/a.rs"))
        #expect(file.lines == .counts(added: 3, removed: 1))

        let binary = ChangedFile(GitChange(path: "x.png", originalPath: nil, kind: .added, added: nil, removed: nil, lfs: false, submodule: nil))
        #expect(binary.lines == .binary)

        let module = ChangedFile(
            GitChange(path: "vendor", originalPath: nil, kind: .modified, added: nil, removed: nil, lfs: false, submodule: "2 new commits"))
        #expect(module.lines == .submodule("2 new commits"))
        #expect(module.about == "Modified · 2 new commits")
    }

    @Test func aRevGoesToRustAndBackUnchanged() {
        for rev in [Rev(name: "main"), Rev(name: "v1", kind: .tag), Rev(name: "abc123", kind: .commit, detail: "Fix it"), .stashes] {
            #expect(Rev(GitRev(rev)) == rev)
        }
    }

    @Test func aLogRowCarriesItsFirstParentAndTheMergeFlagAndRemotesAreBranches() {
        let row = GitLogCommit(
            sha: "a1b2c3d4e5f6", short: "a1b2c3d", parent: "9f8e7d6c", merge: true, author: "Ada", time: 1_700_000_000,
            subject: "Merge x",
            refs: [GitRef(kind: .head, name: "HEAD"), GitRef(kind: .remote, name: "origin/main"), GitRef(kind: .tag, name: "v1")])
        let commit = Commit(row)
        #expect(commit.parents == ["9f8e7d6c"] && commit.isMerge)
        #expect(commit.refs.map(\.kind) == [.head, .branch, .tag])
        #expect(commit.author.name == "Ada" && commit.author.time == 1_700_000_000)
    }

    @Test func openedCommitsCarryMessageSignatureAndFiles() {
        let signature = GitSignature(name: "Ada", email: "ada@example.com", time: 1_700_000_000, zone: 7200)
        let details = GitCommitDetails(
            sha: "abc", parents: ["p1", "p2"], author: signature, committer: signature, refs: [], subject: "Subject",
            body: "Body.", cut: false,
            files: [GitChange(path: "a", originalPath: nil, kind: .added, added: 1, removed: 0, lfs: true, submodule: nil)])
        let commit = Commit(details)
        #expect(commit.isMerge && commit.body == "Body." && commit.author.zone == 7200)
        #expect(commit.files.first?.lines == .lfs)
    }

    @Test func aTextDiffIsTheUnifiedPatchAndTheOtherKindsSayWhyInCoresWords() throws {
        let patch = "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -1 +1 @@\n-x\n+y\n"
        #expect(try GitDiff.text(patch: patch).text() == patch)
        #expect(UnifiedDiffParser.parse(patch).first?.hunks.first?.added == 1)
        #expect(try GitDiff.binary(patch: "diff --git a/b b/b\nBinary files a/b and b/b differ\n", sizes: nil).text().contains("Binary files"))

        func failure(_ diff: GitDiff) -> String {
            do { _ = try diff.text(); return "" } catch { return (error as? GitFailure)?.message ?? "" }
        }
        #expect(failure(.lfs(summary: "1 KB → 2 KB · oid aaaaaa…")) == "Stored with Git LFS: 1 KB → 2 KB · oid aaaaaa…")
        #expect(failure(.tooLarge(bytes: 9_000_000, title: "This diff is 9 MB", detail: "Loading it can take a few seconds.")) == "This diff is 9 MB. Loading it can take a few seconds.")
        let newer = GitLogCommit(sha: "s", short: "s1", parent: nil, merge: false, author: "A", time: 0, subject: "newer", refs: [])
        #expect(failure(.submodule(label: "1 new commit", note: nil, ahead: [newer], behind: [])) == "1 new commit\n+ s1 newer")
    }

    @Test func rustErrorsBecomeThePanelsFailure() {
        #expect(GitQueue.failure(GitBackendError.Failed(message: "fatal: bad revision 'x'")).message == "fatal: bad revision 'x'")
        #expect(GitQueue.failure(GitBackendError.NotARepository).message == "Not a git repository")
    }
}
