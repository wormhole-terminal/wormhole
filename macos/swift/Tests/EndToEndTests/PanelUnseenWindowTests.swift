import Foundation
import Testing

/// The panel's own git reads in a window nobody sees (X: lane b's "Sight for git reads"). GPUI reads each terminal's repository
/// once and holds a window nobody sees back (`window/repo/state.rs` `Sight`: a change is owed, and read the moment the window
/// is seen again). The Swift panel has a watch of its own: it must tell nothing to an unseen window and once to a seen one.
extension Instance {
    /// The window counts as seen. A test instance's window is behind the others and so is not, and the panel's git reads wait for
    /// a window nobody sees: a scenario that reads Changes or History after a file changed asks for it.
    func seeWindow() async { await send(["window-state off on"]) }
}

extension EndToEnd {
    @Test("a file changed while the window is not seen is not read into Changes until the window is seen again, then it is")
    @MainActor func changesWaitForAWindowNobodySees() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("sight-repo", files: ["README.md": "one\n"])
            await app.terminal(in: repo)
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=0 ") }
            await app.seeWindow()
            // Seen, a change is read at once: the watch is live and the panel follows it.
            try app.scratch.write(repo, "seen.txt", "new\n")
            await app.until("changes") { $0.contains("phase=loaded") && $0.contains("files=1 ") }

            await app.send(["window-state off off"])
            try app.scratch.write(repo, "unseen-a.txt", "new\n")
            try app.scratch.write(repo, "unseen-b.txt", "new\n")
            // Not told: for as long as the debounce and a read would take several times over (the app's own pause between
            // the asks), the list is what it was.
            for _ in 0..<6 {
                let answer = (await app.send(["changes", "delay 500"])).first ?? ""
                app.expect(answer.contains("files=1 "), "Changes is not read while the window is not seen:\n\(answer)")
            }

            await app.send(["window-state off on"])
            let rows = await Self.changeRows(app, files: 3)
            app.expect(rows.all("file").count == 3, "all three files once the window is seen again:\n\(rows.text)")
        }
    }
}
