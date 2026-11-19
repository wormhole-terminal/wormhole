import Foundation
import Testing

/// The menu bar extra's and the quick panel's rows that act on terminals, run as choosing them would (`ambient-run`, `quick-*`:
/// a test instance makes no status item and shows no panel; the item drawn and the hotkey stay a person's): a reply typed in
/// the quick panel reaches the waiting terminal, a project row opens or shows its terminal, a project's action runs only
/// once the project is trusted.
extension EndToEnd {
    @Test("Reply in the quick panel types the answer into the waiting terminal and presses Return")
    @MainActor func quickPanelReply() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let waiting = two.live.first, let other = two.live.last else { return }
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: waiting, becomes: "needsInput")
            await app.send(["select \(other)"])
            // ⌘⏎ on a waiting row opens Reply in place; ⏎ sends it (status_item/quick_panel/mod.rs:3).
            await app.send(["quick-open"])
            // The waiting row is the one ⌘⏎ answers: down the list until it does.
            var said = await app.reply("quick-key cmd-enter")
            for _ in 0..<6 where !said.contains("answering tab") {
                await app.send(["quick-key down"])
                said = await app.reply("quick-key cmd-enter")
            }
            app.expect(said, contains: "answering tab \(waiting)")
            await app.send(["quick-text yes-from-the-panel"])
            app.expect(await app.reply("quick-key enter"), contains: "not answering")
            await app.send(["select \(waiting)"])
            await app.screen(showing: "yes-from-the-panel")
        }
    }

    @Test("a project's row opens a terminal in it, and shows that one when it is open; its action runs only once trusted")
    @MainActor func ambientProjectRows() async throws {
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            let repo = try app.scratch.repository("ambient-project")
            try app.scratch.project(repo, files: ["actions.json": Self.sayAction])
            await app.flags(["project_folders": true])
            // Open project: a terminal there, or the one already there (status_item/mod.rs:301).
            let before = (await app.rows()).live.count
            await app.send(["ambient-run open-project \(repo.path)"])
            let opened = Listing(await app.until("rows") { Listing($0).live.count == before + 1 })
            guard let there = opened.rows.first(where: { $0.folder.hasSuffix("ambient-project") })?.id else { return app.fail("no terminal in the project:\n\(opened.text)") }
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == before + 2 }
            await app.send(["ambient-run open-project \(repo.path)"])
            let shown = Listing(await app.until("rows") { Listing($0).selected == there })
            app.expect(shown.live.count == before + 2, "no second terminal for the same project:\n\(shown.text)")

            // Run action: refused by the core while the project is untrusted (status_item/mod.rs:283), run once it is.
            await app.send(["ambient-run run-action \(repo.path) say"])
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.send(["ambient-run run-action \(repo.path) say"])
            await app.screen(showing: "e2e-42")
            // Once: the run asked before the trust did nothing.
            app.expect(await app.count("e2e-42") == 1, "the action ran once, after the trust")
        }
    }
}
