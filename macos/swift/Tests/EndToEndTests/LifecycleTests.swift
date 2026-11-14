import Foundation
import Testing

extension EndToEnd {
    @Test("quit and come back: two terminals, the split and a terminal's text are restored")
    @MainActor func restore() async throws {
        try await Instance.with { app in
            await app.send(["type remember-me\\n"])
            await app.until("screen") { $0.contains("remember-me") }
            await app.send(["key cmd-d"])
            await app.until("panes") { Panes($0).ids.count == 2 }
            let before = await app.rows()
            await app.send(["save-windows"])
            try await app.restart()
            let after = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            app.expect(after.live == before.live, "the same terminals:\n\(before.text)\n\(after.text)")
            let panes = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            app.expect(panes.isSplit, "the split is back")
            // The terminal that had the text is the first one.
            await app.send(["select \(before.live[0])"])
            await app.until("screen") { $0.contains("remember-me") }
        }
    }

    @Test("a terminal opens in a new worktree, and closing it offers to remove the worktree")
    @MainActor func worktree() async throws {
        try await Instance.with(shell: .zsh) { app in
            let repo = try app.scratch.repository()
            let worktree = app.scratch.home.appendingPathComponent("repo-worktrees/feature-x")
            await app.send(["new \(repo.path)"])
            await app.until("rows") { Listing($0).live.count == 2 }
            // Asked as the palette asks; the sheet over the window takes the branch's name and Return.
            await app.send(["action newTerminalInWorktree"])
            await app.until("windows") { $0.contains("sheet=true") }
            await app.send(["text feature-x", "key return"])
            let opened = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            app.expect(opened.rows.contains { $0.folder.contains("feature-x") }, "a terminal in the worktree:\n\(opened.text)")
            app.expect(await app.waitFor(worktree), "the worktree folder exists: \(worktree.path)")

            await app.send(["key cmd-w"])
            let asked = await app.until("question") { $0.contains("title=") }
            app.expect(asked, contains: "Remove")
            await app.send(["question also 1"])
            app.expect(await app.waitFor(worktree, present: false), "the worktree folder was removed")
            app.expect(try app.scratch.git(repo, "branch", "--list", "feature-x"), contains: "feature-x")
        }
    }

    @Test("a hook from an agent makes its row work, need you, then finish")
    @MainActor func agentStatus() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            // Another terminal is in front, so the first one's finish is news.
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            try await app.report(agent: "codex", event: "UserPromptSubmit", terminal: 1, becomes: "working")
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: 1, becomes: "needsInput")
            try await app.report(agent: "codex", event: "Stop", terminal: 1, becomes: "finished")
        }
    }

    @Test("a second window opens, a terminal moves to a window of its own, each control file answers for its window")
    @MainActor func twoWindows() async throws {
        try await Instance.with { app in
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            // `cmd` follows the window in front, which is the new one now: each window is asked through its own file.
            let first = await app.reply("state", window: 1)
            let second = await app.reply("state", window: 2)
            let numbers = [Instance.value("window", in: first), Instance.value("window", in: second)]
            app.expect(numbers[0] != nil && numbers[0] != numbers[1], "each file answers for its own window: \(numbers)")
            let one = await app.rows(window: 1), two = await app.rows(window: 2)
            app.expect(!one.live.isEmpty && !two.live.isEmpty && Set(one.live).isDisjoint(with: two.live), "own terminals:\n\(one.text)\n\(two.text)")

            await app.send(["key cmd-t"], window: 1)
            let grown = await app.until("rows", window: 1) { Listing($0).live.count == 2 }
            guard let moving = Listing(grown).live.last else { return }
            await app.send(["move-to-window \(moving)"], window: 1)
            await app.until("sessions") { $0.split(separator: "\n").count == 3 }
            let third = await app.rows(window: 3)
            app.expect(third.live.count == 1, "the third window holds the moved terminal:\n\(third.text)")
            app.expect(!(await app.rows(window: 1)).live.contains(moving), "the first window let it go")
        }
    }
}
