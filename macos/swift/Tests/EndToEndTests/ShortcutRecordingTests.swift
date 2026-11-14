import Foundation
import Testing

extension EndToEnd {
    @Test("while a shortcut is being recorded, a key of the app's own is the recorder's: ⇧⌘N opens no terminal")
    @MainActor func recorderHasTheKeysFirst() async throws {
        try await Instance.with { app in
            let before = await app.setting("hotkey")
            await app.send(["settings-reveal hotkey"])
            await app.until("settings-rows") { $0.contains("hotkey | ") }
            let terminals = Self.allTerminals(await app.reply("sessions"))
            // ⇧⌘N is New Terminal's second key, which the app's own key monitor runs anywhere else (with Settings in front too).
            // Here the recorder has it: it says the app uses it and goes on recording, so the next key is recorded.
            await app.send(["settings-record hotkey shift-cmd-n shift-cmd-space"])
            let taken = await app.until("get hotkey") { $0 != "hotkey = \(before)" }
            app.expect(taken, contains: "space")
            app.expect(Self.allTerminals(await app.reply("sessions")) == terminals, "⇧⌘N opened no terminal while it was recorded")
        }
    }
}
