import Foundation
import Testing

extension EndToEnd {
    /// What the commands reach of Changes: there is no command that lists the changed files or their count (GPUI's devctl has
    /// `changes`), so the scenario opens the tab and the first file's diff and asserts the window followed; the count and the
    /// diff's lines need such a command. History is read in full (`history-state`, `commit-details`).
    @Test("in a throwaway repository: Changes opens a diff, History lists the commit the test made")
    @MainActor func rightPanel() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository(files: ["README.md": "one\n"], message: "first commit")
            try app.scratch.write(repo, "notes.txt", "two\n")
            try app.scratch.git(repo, "add", "-A")
            try app.scratch.git(repo, "commit", "-q", "-m", "second commit")
            try app.scratch.write(repo, "README.md", "one\nedited\n")
            await app.send(["new \(repo.path)"])
            await app.until("rows") { Listing($0).live.count == 2 }

            await app.send(["tab changes", "diff 1"])
            let state = await app.until("state") { $0.contains("tab=changes") }
            app.expect(state, contains: "tab=changes")
            // The branch the title row names is the repository's.
            app.expect(await app.reply("title"), contains: "main")

            await app.send(["tab history"])
            let history = await app.until("history-state") { $0.contains("commits=2") }
            app.expect(history, contains: "branch=main")
            await app.send(["history 1"])
            let open = await app.until("commit-details") { $0.contains("subject:") }
            app.expect(open, contains: "subject: second commit")
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
