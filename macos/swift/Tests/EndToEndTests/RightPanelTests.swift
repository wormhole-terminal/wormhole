import Foundation
import Testing

extension EndToEnd {
    /// Changes is read through `changes` (the tab's store: the files, their counts, the open diff), History through
    /// `history-state` and `commit-details`.
    @Test("in a throwaway repository: Changes lists the edited file and opens its diff, History lists the commits the test made")
    @MainActor func rightPanel() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(files: ["README.md": "one\n"], message: "first commit")
            try app.scratch.write(repo, "notes.txt", "two\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "second commit")
            try app.scratch.write(repo, "README.md", "one\nedited\n")
            await app.send(["new \(repo.path)"])
            await app.until("rows") { Listing($0).live.count == 2 }

            // Git is read off the main actor: the list is there once the store says it is loaded with the one edited file.
            let listed = await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=1") }
            app.expect(listed, contains: "  M README.md +1 -0")
            app.expect(await app.reply("state"), contains: "tab=changes")
            await app.send(["diff 1"])
            let open = await app.until("changes") { $0.contains("diff=shown") }
            app.expect(open, contains: "open=README.md diff=shown hunks=1")
            // The branch the title row names is the repository's.
            app.expect(await app.reply("title"), contains: "main")

            await app.send(["tab history"])
            let history = await app.until("history-state") { $0.contains("commits=2") }
            app.expect(history, contains: "branch=main")
            await app.send(["history 1"])
            let commit = await app.until("commit-details") { $0.contains("subject:") }
            app.expect(commit, contains: "subject: second commit")
            app.expect(await app.reply("history-state"), contains: "notes.txt")
        }
    }

    @Test("a note and a step, ticked, are there after a restart on the same data folder")
    @MainActor func notes() async throws {
        try await Instance.with { app in
            await app.send(["tab notes", "note-add remember the milk", "task-add write the tests", "task-done 1"])
            let said = await app.until("notes") { $0.contains("remember the milk") && $0.contains("write the tests:true") }
            app.expect(said, contains: "steps=")
            try await app.restart()
            await app.send(["tab notes"])
            let back = await app.until("notes") { $0.contains("remember the milk") }
            app.expect(back, contains: "write the tests:true")
        }
    }
}
