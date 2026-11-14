import Foundation
import Testing

/// Worktrees (a terminal's port, the folder setting, a refusal for an untrusted project) and the Notes tab's tools (seeded notes and
/// steps, a note sent to the terminal, a prompt stashed).
extension EndToEnd {
    @Test("a worktree is made under the Worktrees folder setting, and its terminal has a port the repository's terminal has not")
    @MainActor func worktreePort() async throws {
        try await Instance.with(shell: .zsh) { app in
            let repo = try app.scratch.repository("port-repo")
            await app.send(["set worktree_base=\"~/wt\""])
            _ = await app.waitForConfig { $0.contains("\"worktree_base\"") }
            await app.terminal(in: repo)
            await app.send(["type echo \"port=[$WORMHOLE_PORT]\"\\n"])
            await app.screen(showing: "port=[]")

            await app.send(["action newTerminalInWorktree"])
            await app.until("windows") { $0.contains("sheet=true") }
            await app.send(["text wt-a", "key return"])
            let opened = Listing(await app.until("rows") { Listing($0).rows.contains { $0.folder.contains("wt-a") } })
            guard let row = opened.rows.first(where: { $0.folder.contains("wt-a") }) else { return }
            app.expect(row.folder.contains("wt/"), "the worktree is under the setting's folder: \(row.folder)")
            await app.send(["select \(row.id)", "type echo \"port=[$WORMHOLE_PORT]\"\\n"])
            await app.until("screen") { $0.range(of: #"port=\[[0-9]+\]"#, options: .regularExpression) != nil }
        }
    }

    @Test("a worktree asked for from the command line in an untrusted project is refused with the trust words, and nothing is made")
    @MainActor func worktreeRefusedUntrusted() async throws {
        let config = #"{"feature_flags": {"unlocked": true, "automation": true, "project_folders": true}, "agent_access": "open"}"#
        try await Instance.with(config: config) { app in
            let repo = try app.scratch.repository("guarded-repo")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 19}"#])
            guard let terminal = (await app.terminal(in: repo)).selected else { return }
            let said = await app.reply("control --from \(terminal) new-terminal --worktree wt-b")
            app.expect(said, contains: "Trust the project first")
            app.expect((await app.reply("windows")).contains("sheet=true") == false, "no sheet was put up")
            let made = app.scratch.home.appendingPathComponent("guarded-repo-worktrees/wt-b")
            app.expect(!FileManager.default.fileExists(atPath: made.path), "no worktree was made")
        }
    }

    @Test("notes and steps seeded, folded and ticked; a note goes to the terminal; nothing typed at a prompt is no stash")
    @MainActor func notesTools() async throws {
        try await Instance.with { app in
            await app.send(["tab notes", "note-seed 3 6", "task-seed 3 1"])
            let seeded = await app.until("notes") { $0.contains("Step 4:") }
            app.expect(seeded.components(separatedBy: "Note ").count - 1 >= 3, "three notes:\n\(seeded)")
            app.expect(seeded.components(separatedBy: ":true").count - 1 == 1, "one step of four is done:\n\(seeded)")
            app.expect(await app.reply("note-toggle 2"), contains: "note-toggle 2")
            app.expect(await app.reply("note-toggle 9"), contains: "error: no note 9")
            await app.send(["task-done 1"])
            await app.until("notes") { $0.contains("Step 1:") && $0.components(separatedBy: ":true").count - 1 != 1 }

            // A note's first paragraph goes to the terminal, which `cat` echoes.
            app.expect(await app.reply("send"), contains: "send")
            await app.screen(showing: "line 0: some text")
            app.expect(await app.reply("stash"), contains: "stash: nothing is typed at a prompt")
        }
    }
}
