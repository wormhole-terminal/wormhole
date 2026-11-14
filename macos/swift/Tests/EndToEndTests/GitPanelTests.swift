import Foundation
import Testing

/// Changes, a diff and History in throwaway repositories, read through `changes` (the tab's store, a line a file),
/// `history-state`, `commit-details` and `picker`. Git is read off the main actor: each fact is waited for.
extension EndToEnd {
    /// The file lines of `changes`, without its head line.
    static func changedFiles(_ said: String) -> [String] { said.split(separator: "\n").dropFirst().map(String.init) }

    @Test("from a subfolder, Changes lists the repository's files by their whole path, a nested repository as a folder, and opens a diff")
    @MainActor func changesFromASubfolder() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("sub-repo", files: ["README.md": "one\n", "crates/x/lib.rs": "fn x() {}\n"])
            try app.scratch.write(repo, "crates/x/lib.rs", "fn x() {}\nfn y() {}\n")
            // An untracked folder that is a repository of its own: git lists it as one entry.
            try app.scratch.repository("sub-repo/vendor/lib", files: ["lib.c": "int x;\n"])
            await app.terminal(in: repo.appendingPathComponent("crates/x"))
            let listed = await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=2") }
            let files = Self.changedFiles(listed)
            app.expect(listed, contains: "  M crates/x/lib.rs +1 -0")
            let nested = files.first { $0.contains("vendor/lib") }
            app.expect(nested != nil && !(nested ?? "").contains("+0"), "the nested repository is one folder, with no line counts:\n\(listed)")

            guard let n = files.firstIndex(where: { $0.contains("crates/x/lib.rs") }) else { return }
            await app.send(["diff \(n + 1)"])
            app.expect(await app.until("changes") { $0.contains("diff=shown") }, contains: "open=crates/x/lib.rs diff=shown hunks=1")
            await app.until("keys") { $0.contains("+cover") }
            // Esc closes the diff, and the terminal has the keys again.
            await app.send(["key esc"])
            let keys = await app.until("keys") { !$0.contains("+cover") }
            app.expect(keys.hasPrefix("focus=terminal"), "the terminal has the keys:\n\(keys)")

            // Away on another tab while a file changes: back on Changes, the list is read again.
            await app.send(["tab notes"])
            try app.scratch.write(repo, "crates/x/new.txt", "new\n")
            await app.until("changes") { $0.contains("files=3") && $0.contains("crates/x/new.txt") }
        }
    }

    @Test("a compare branch is the terminal's own: another terminal shows its working tree, and the branch comes back with the first")
    @MainActor func compareFollowsEachTerminal() async throws {
        try await Instance.with(shell: .zsh) { app in
            let repo = try app.scratch.repository("cmp-repo", files: ["README.md": "one\n", "sub/keep.txt": "k\n"])
            try app.scratch.git(repo, "checkout", "-q", "-b", "feature")
            try app.scratch.write(repo, "f.txt", "f\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "feature work")
            let first = await app.terminal(in: repo)
            guard let a = first.selected else { return }
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=0") }
            app.expect(await app.until("picker compare ma") { $0.contains("main") }, contains: "branch main")
            await app.send(["compare main"])
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("f.txt") }

            let second = await app.terminal(in: repo)
            app.expect(second.selected != a, "the second terminal is selected")
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=0") }
            await app.send(["select \(a)"])
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("f.txt") }

            // A `cd` in the same terminal keeps its compare branch.
            await app.send(["type cd sub\\n"])
            await app.until("rows") { Listing($0).row(a)?.folder.hasSuffix("/sub") == true }
            app.expect(await app.until("changes") { $0.contains("phase=loaded") }, contains: "f.txt")
            await app.send(["compare HEAD"])
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=0") }
        }
    }

    @Test("History opens a commit's details, shows more of a long message, opens its file, and keeps each terminal's branch")
    @MainActor func historyDetails() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("hist-repo", files: ["README.md": "one\n"], message: "first commit")
            try app.scratch.git(repo, "branch", "older")
            try app.scratch.write(repo, "two.txt", "two\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "second commit")
            let body = String(repeating: "A long line of the body that goes on. ", count: 12)
            try app.scratch.write(repo, "three.txt", "three\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "third commit", "-m", body)
            let first = await app.terminal(in: repo)
            guard let a = first.selected else { return }
            await app.send(["tab history"])
            await app.until("history-state") { $0.contains("commits=3") }

            await app.send(["history 1"])
            let details = await app.until("commit-details") { $0.contains("subject:") }
            app.expect(details, contains: "subject: third commit")
            app.expect(details, contains: "message: compact (toggle offered")
            app.expect(await app.reply("commit-more"), contains: "commit-more whole")
            app.expect(await app.reply("commit-details"), contains: "message: whole")
            app.expect(await app.later("history-copy 1 subject", answer: "copy="), contains: "copy=third commit")
            // A tag put on the open commit from outside: its refs are read again.
            try app.scratch.git(repo, "tag", "v-e2e")
            await app.until("commit-details") { $0.contains("v-e2e") }
            await app.send(["history-file 1"])
            app.expect(await app.until("history-state") { $0.contains("diff=shown") }, contains: "three.txt")
            await app.send(["history 0"])
            app.expect(await app.reply("commit-details"), contains: "none")

            // The branch picker: this terminal's log is another branch's; a second terminal has HEAD's, and the first gets its own back.
            app.expect(await app.until("picker history old") { $0.contains("older") }, contains: "branch older")
            await app.send(["history-rev older"])
            await app.until("history-state") { $0.contains("commits=1") }
            await app.terminal(in: repo)
            await app.until("history-state") { $0.contains("commits=3") }
            await app.send(["select \(a)"])
            await app.until("history-state") { $0.contains("commits=1") }
        }
    }
}
