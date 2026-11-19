import Foundation
import Testing

/// A project action's own key (`actions.json` `key`): held down it runs the action once, and it runs only the action of the
/// project of the terminal that has the keys, never the one the panel showed before a switch (`ProjectActionKeys.swift`).
extension EndToEnd {
    @Test("an action's key held down runs it once; in a terminal of another folder it runs nothing of the first project")
    @MainActor func actionKeyHeldAndSwitched() async throws {
        try await Instance.with(shell: .zsh, auth: "allow") { app in
            let keyed = try app.scratch.repository("keyed-held")
            try app.scratch.project(keyed, files: ["actions.json": Self.sayAction])
            let plain = try app.scratch.repository("plain-beside")
            await app.flags(["project_folders": true])
            await app.terminal(in: keyed)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }

            // Pressed, repeated twice by the keyboard, let go: one run, and the repeats reach neither the terminal nor a menu.
            await app.send(["key-held ctrl-cmd-t"])
            await app.screen(showing: "e2e-42")
            await app.send(["type echo mark-$((2+3))\\n"])
            await app.screen(showing: "mark-5")
            app.expect(await app.count("e2e-42") == 1, "the held key ran the action once")

            // Another terminal, in a folder with no project, selected and keyed at once: whatever the panel still shows of the
            // first project, the key is not that project's here.
            await app.terminal(in: plain)
            await app.send(["key ctrl-cmd-t", "type echo mark-$((3+4))\\n"])
            await app.screen(showing: "mark-7")
            app.expect(await app.count("e2e-42") == 0, "the first project's action did not run in the other terminal")
        }
    }
}
