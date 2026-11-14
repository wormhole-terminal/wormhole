import Foundation
import Testing

/// Two windows of one profile on the same project: what one does to the project's trust, its switches and its notes.md the
/// other neither undoes nor misses (one project store per profile, `wormhole-ffi` `files_panel/project/stores.rs`). Read
/// through each window's `project-state` and the file itself; the owner's question is answered by `WORMHOLE_AUTH=allow`.
extension EndToEnd {
    @Test("two windows of one profile on one project: both windows' steps reach notes.md; a revoke in one holds in the other, also after it approves another project; a switch made in one stays")
    @MainActor func projectInTwoWindows() async throws {
        try await Instance.with(auth: "allow") { app in
            let repo = try app.scratch.repository("shared-project")
            try app.scratch.project(repo, files: [
                "actions.json": Self.sayAction, "prompts/review.md": "# Review\nLook at the diff.\n",
                "notes.md": "# Notes\n\n## Next steps\n\n- [ ] from the file\n",
            ])
            let other = try app.scratch.repository("other-project")
            try app.scratch.project(other, files: ["actions.json": Self.sayAction])
            let file = repo.appendingPathComponent(".wormhole/notes.md")
            await app.flags(["project_folders": true])
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            guard await openTerminal(repo, in: 1, of: app) != nil, let shared = await openTerminal(repo, in: 2, of: app) else { return }

            // Trusted in the first window: the second reads the same approval.
            await app.send(["tab project"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("status=untrusted") }
            await app.send(["project-approve"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("status=trusted") }
            await app.send(["tab project"], window: 2)
            await app.until("project-state", window: 2) { $0.contains("status=trusted") && $0.contains("actions=[\"say\"]") }

            // A step added in each window, both merged now: notes.md holds both, once.
            await app.send(["tab notes", "task-add from window one", "notes-sync"], window: 1)
            await app.send(["tab notes", "task-add from window two", "notes-sync"], window: 2)
            let both = await projectNotes(app, file) { $0.contains("from window one") && $0.contains("from window two") }
            for step in ["from window one", "from window two", "from the file"] {
                app.expect(both.components(separatedBy: step).count == 2, "notes.md holds \"\(step)\" once:\n\(both)")
            }

            // Revoked in the first window: the second reads it at once, offers no action and runs none.
            await app.send(["tab project", "project-revoke"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("status=untrusted") }
            await app.send(["tab project"], window: 2)
            let revoked = await app.until("project-state", window: 2) { $0.contains("status=untrusted") }
            app.expect(revoked, contains: "actions=[]")
            app.expect(await app.reply("project-run say", window: 2), contains: "no such id say")

            // A step added in the second window now stays out of notes.md, and its Project tab says why.
            await app.send(["tab notes", "task-add after the revoke", "notes-sync", "tab project"], window: 2)
            await app.until("project-state", window: 2) { $0.contains("waits for the project to be trusted") }
            let after = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
            app.expect(!after.contains("after the revoke"), "a revoked project's notes.md is not written:\n\(after)")

            // The second window approves another project: the revoked one stays revoked in both windows.
            guard await openTerminal(other, in: 2, of: app) != nil else { return }
            await app.send(["tab project"], window: 2)
            await app.until("project-state", window: 2) { $0.contains("other-project") && $0.contains("status=untrusted") }
            await app.send(["project-approve"], window: 2)
            await app.until("project-state", window: 2) { $0.contains("other-project") && $0.contains("status=trusted") }
            let first = await app.until("project-state", window: 1) { $0.contains("shared-project") }
            app.expect(first, contains: "status=untrusted")
            app.expect(first, contains: "actions=[]")
            await app.send(["select \(shared)", "tab project"], window: 2)
            let second = await app.until("project-state", window: 2) { $0.contains("shared-project") }
            app.expect(second, contains: "status=untrusted")
            app.expect(second, contains: "actions=[]")

            // A switch turned off in each window: both stay off, in both windows, after the other one saved.
            await app.send(["project-feature prompts off"], window: 1)
            await app.until("project-state", window: 1) { $0.contains("prompts=false") }
            await app.send(["project-feature notes off"], window: 2)
            for window in [1, 2] {
                let state = await app.until("project-state", window: window) { $0.contains("notes=false") }
                app.expect(state, contains: "prompts=false")
            }
        }
    }

    /// A new terminal in `folder` in the n-th window, selected there; its id.
    @MainActor private func openTerminal(_ folder: URL, in window: Int, of app: Instance) async -> Int? {
        let before = (await app.rows(window: window)).live
        await app.send(["new \(folder.path)"], window: window)
        let listed = Listing(await app.until("rows", window: window) { Listing($0).live.count == before.count + 1 })
        let added = listed.live.first { !before.contains($0) }
        app.expect(added != nil, "a terminal in \(folder.lastPathComponent) in window \(window):\n\(listed.text)")
        return added
    }

    /// Waits, pausing by the app's own `delay`, until the project's notes.md satisfies `done`.
    @discardableResult
    @MainActor private func projectNotes(_ app: Instance, _ url: URL, timeout: TimeInterval = 20, _ done: (String) -> Bool) async -> String {
        let start = Date()
        var text = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        while !done(text), Date().timeIntervalSince(start) < timeout {
            await app.send(["delay 200"])
            text = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
        }
        app.expect(done(text), "notes.md never said what was expected; it is:\n\(text)")
        return text
    }
}
