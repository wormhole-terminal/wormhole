import Foundation
import Testing

/// The profile's approvals changed by another program: `trusted-projects.json` in the scratch data folder is written from
/// outside the app, as a script or a second Wormhole on the same data folder writes it. The window follows the file as an
/// event (core's `platform::watch`, in the bridge's `files_panel/project/stores.rs`), well inside the slow looks that stay as
/// the fallback: the Project tab's half minute and the title row's minute. Every wait below ends long before either, so a
/// trust that is kept after the file says otherwise fails here.
extension EndToEnd {
    /// How long the window has to follow the file: an event's latency, never a slow look's.
    private static let followsWithin: TimeInterval = 10

    /// Another program takes the approval back: the profile's file is replaced, with nothing said to the app.
    @MainActor private static func revokeOutside(_ app: Instance, project: String) throws {
        let approvals = app.scratch.data.appendingPathComponent("trusted-projects.json")
        let stored = (try? String(contentsOf: approvals, encoding: .utf8)) ?? ""
        app.expect(stored.contains(project), "the approval is in the profile's file:\n\(stored)")
        try "{}".write(to: approvals, atomically: true, encoding: .utf8)
    }

    // Notes stays off here: its sync asks the core every two seconds, and a refusal there tells every tab too, which would
    // hide a window that does not follow the file.
    @Test("a project revoked by another program: the tab says so, the pull request chip goes and its action runs nothing, at once")
    @MainActor func trustRevokedOutside() async throws {
        try await Instance.with(shell: .zsh, auth: "allow", agents: ["gh"]) { app in
            let log = app.scratch.root.appendingPathComponent("gh-calls.txt")
            try Self.standInGH(app, log: log)
            let repo = try app.scratch.repository("outside-project")
            try app.scratch.project(repo, files: ["actions.json": Self.sayAction])
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }

            // Trusted: the chip is asked of gh and shown, and the action's key runs it.
            await app.until("title") { $0.contains("number: 142") }
            await app.send(["key ctrl-cmd-t"])
            await app.screen(showing: "e2e-42")
            let asked = ((try? String(contentsOf: log, encoding: .utf8)) ?? "").split(separator: "\n").count

            try Self.revokeOutside(app, project: "outside-project")

            // The window follows as an event: the tab reads untrusted and the chip goes, long before a slow look.
            let state = await app.until("project-state", timeout: Self.followsWithin) { $0.contains("status=untrusted") }
            app.expect(state.contains("status=untrusted"), "the Project tab says the project is not trusted:\n\(state)")
            let title = await app.until("title", timeout: Self.followsWithin) { !$0.contains("number: 142") }
            app.expect(!title.contains("number: 142"), "no pull request chip once the trust is gone:\n\(title)")
            app.expect(await app.reply("pull-request"), contains: "error: no pull request")

            // Nothing of the project runs: its key is the terminal's again, and the action by its id runs nothing either.
            await app.send(["key ctrl-cmd-t", "project-run say", "type echo mark-$((2+3))\\n"])
            await app.screen(showing: "mark-5")
            app.expect(await app.count("e2e-42") == 1, "after the trust went, neither the key nor the action ran")

            // And gh, which reads the repository's own git configuration, was not started again.
            let after = ((try? String(contentsOf: log, encoding: .utf8)) ?? "").split(separator: "\n").count
            app.expect(after == asked, "gh ran \(after - asked) more times after the revoke")
        }
    }

    @Test("a project revoked by another program no longer syncs its notes.md, either way")
    @MainActor func trustRevokedOutsideStopsNotesSync() async throws {
        try await Instance.with(auth: "allow") { app in
            let repo = try app.scratch.repository("outside-notes")
            try app.scratch.project(repo, files: ["notes.md": "# Notes\n\n## Next steps\n\n- [ ] from the file\n"])
            let notesFile = repo.appendingPathComponent(".wormhole/notes.md")
            await app.flags(["project_folders": true, "notes_and_tasks": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            // Trusted: notes.md reaches the terminal's notes.
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("from the file:false") }

            try Self.revokeOutside(app, project: "outside-notes")
            await app.send(["tab project"])
            await app.until("project-state", timeout: Self.followsWithin) { $0.contains("status=untrusted") }

            // A step made here stays out of the file, a line added to the file stays out of the notes. Two of the sync's own
            // periods (2 s each) pass before the look, on the app's clock: there is nothing to wait for but that it did not happen.
            await app.send(["tab notes", "task-seed 1 0"])
            await app.until("notes") { $0.contains("Step 1:false") }
            let text = (try? String(contentsOf: notesFile, encoding: .utf8)) ?? ""
            try app.scratch.write(repo, ".wormhole/notes.md", text + "- [ ] added after the revoke\n")
            await app.send(["delay 2500", "delay 2500"])
            let notes = await app.reply("notes")
            app.expect(!notes.contains("added after the revoke"), "the file's new line did not reach the notes:\n\(notes)")
            let file = (try? String(contentsOf: notesFile, encoding: .utf8)) ?? ""
            app.expect(!file.contains("Step 1"), "the step made after the revoke is not in notes.md:\n\(file)")
        }
    }
}
