import Foundation
import Testing

/// The folder picker (⌘J) over a terminal, driven by its own keys (`folder-picker …`) and read by `folder-picker state`; its
/// queries run off the main thread, so each answer is waited for until it says `busy false`.
extension EndToEnd {
    @Test("the folder picker narrows to what is typed, pastes the pick into its terminal, and closes when another is selected")
    @MainActor func folderPicker() async throws {
        try await Instance.with { app in
            let picks = app.scratch.home.appendingPathComponent("picks")
            for name in ["pick-me", "other-folder"] {
                try FileManager.default.createDirectory(at: picks.appendingPathComponent(name), withIntermediateDirectories: true)
            }
            await app.flags(["folder_picker": false])
            app.expect(await app.reply("folder-picker open"), contains: "error: the Folder picker flag is off")
            await app.flags(["folder_picker": true])
            let opened = await app.terminal(in: picks)
            guard let picking = opened.selected else { return }

            app.expect(await app.reply("folder-picker open"), contains: "folder-picker open")
            await app.send(["folder-picker type pick"])
            let narrowed = await app.until("folder-picker state") { $0.contains("busy false") && $0.contains("pick-me") }
            app.expect(narrowed, contains: "typed \"pick\"")
            app.expect(!narrowed.contains("other-folder"), "only what matches:\n\(narrowed)")
            await app.send(["folder-picker enter"])
            await app.screen(showing: "pick-me")

            // Open again, then another terminal comes in front: the picker goes.
            await app.send(["folder-picker open", "key cmd-t"])
            await app.until("rows") { Listing($0).selected != picking && Listing($0).live.count == 3 }
            await app.until("folder-picker state") { $0.contains("error: the folder picker is not open") }
        }
    }
}
