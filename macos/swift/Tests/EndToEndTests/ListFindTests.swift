import Foundation
import Testing

/// ⌘F while the terminal list has the keys opens the terminal's find bar (the list has no search of its own): TO-VERIFY
/// line 183. Read from where the keys are (`key`'s reply) and the find bar (`find-state`).
extension EndToEnd {
    @Test("⌘F with the keys in the terminal list opens the selected terminal's find bar")
    @MainActor func findFromTheList() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("find-state"), contains: "closed")
            // F6 walks the window's areas: until the list has the keys.
            var said = ""
            for _ in 0..<6 where !said.contains("focus=sidebar") { said = await app.reply("key f6") }
            app.expect(said, contains: "focus=sidebar")
            await app.send(["key cmd-f"])
            await app.until("find-state") { $0.contains(" open") }
        }
    }
}
