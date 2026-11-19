import Foundation
import Testing

extension EndToEnd {
    /// Two terminals of one trusted project merge their notes with its `notes.md` every two seconds, the one on show from
    /// the panel and the other from the database (`NotesCoordinator`). Read through `notes` (the shown terminal's tab) and
    /// the file itself.
    @Test("two terminals of a trusted project: a step ticked in one and a line added to notes.md reach both and the file, once")
    @MainActor func notesSync() async throws {
        try await Instance.with(auth: "allow") { app in
            // The panel's tabs are features behind their flags, which a window follows: on for what this scenario shows.
            await app.flags(["project_folders": true, "notes_and_tasks": true])
            let repo = try app.scratch.repository(
                "project", files: [".wormhole/notes.md": "# Notes\n\n## Next steps\n\n- [ ] from the file\n", "README.md": "one\n"])
            let file = repo.appendingPathComponent(".wormhole/notes.md")
            let before = (await app.rows()).live
            await app.send(["new \(repo.path)", "new \(repo.path)"])
            let listed = Listing(await app.until("rows") { Listing($0).live.count == before.count + 2 })
            let added = listed.live.filter { !before.contains($0) }
            app.expect(added.count == 2, "two terminals in the project:\n\(listed.text)")
            guard added.count == 2 else { return }
            let (one, two) = (added[0], added[1])

            // The project is trusted (the approval is answered by WORMHOLE_AUTH=allow, never a prompt), so its notes.md syncs.
            await app.send(["select \(one)", "tab project"])
            await app.until("project-state") { $0.contains("status=untrusted") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("from the file:false") }

            // Ticked in the one on show while the other merges from the database: the tick reaches the file, then the other.
            await app.send(["task-done 1"])
            await notesFile(app, file) { $0.contains("- [x] from the file") }
            await app.send(["select \(two)"])
            await app.until("notes") { $0.contains("from the file:true") }

            // A line added to the file from outside reaches both.
            let text = try String(contentsOf: file, encoding: .utf8)
            var lines = text.components(separatedBy: "\n")
            let at = lines.firstIndex { $0.contains("from the file") } ?? lines.count - 1
            lines.insert("- [ ] added outside", at: at + 1)
            try app.scratch.write(repo, ".wormhole/notes.md", lines.joined(separator: "\n"))
            let second = await app.until("notes") { $0.contains("added outside:false") }
            await app.send(["select \(one)"])
            let first = await app.until("notes") { $0.contains("added outside:false") }

            // Nothing there twice, in either terminal or in the file.
            for (said, name) in [(first, "the first terminal"), (second, "the second terminal")] {
                app.expect(count("from the file", in: said) == 1 && count("added outside", in: said) == 1, "\(name) holds each step once:\n\(said)")
                app.expect(said, contains: "from the file:true")
            }
            let settled = await notesFile(app, file) { $0.contains("added outside") }
            app.expect(count("from the file", in: settled) == 1 && count("added outside", in: settled) == 1, "notes.md holds each once:\n\(settled)")
        }
    }

    private func count(_ needle: String, in text: String) -> Int { text.components(separatedBy: needle).count - 1 }

    /// Waits, pausing by the app's own `delay`, until the project's notes.md satisfies `done`.
    @discardableResult
    @MainActor private func notesFile(_ app: Instance, _ url: URL, timeout: TimeInterval = 20, _ done: (String) -> Bool) async -> String {
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
