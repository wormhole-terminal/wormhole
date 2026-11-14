import Foundation
import Testing

extension EndToEnd {
    @Test("a part hidden from the title row's menu is the setting: config.json has it, a second window follows, a restart keeps it")
    @MainActor func titleRowMenuWritesTheSetting() async throws {
        try await Instance.with { app in
            // The row's own menu, as a right-click on the row opens it: every part ticked by default.
            let menu = await app.reply("title-menu")
            app.expect(menu, contains: "Open ports [x]")
            app.expect(Instance.value("row", in: menu)?.contains("ports") == true, "the row shows the ports by default:\n\(menu)")
            app.expect(await app.reply("menu-pick Open ports"), contains: "picked Open ports")

            // The row on screen changes when the setting comes back to it, and the file has it.
            func withoutPorts(_ said: String) -> Bool { Instance.value("row", in: said).map { !$0.contains("ports") } ?? false }
            let changed = await app.until("title-menu") { withoutPorts($0) }
            app.expect(!changed.contains("Open ports [x]"), "the menu no longer ticks it:\n\(changed)")
            _ = await app.waitForConfig { $0.contains("\"title_row\"") && !$0.contains("\"ports\"") }
            app.expect(!(await app.setting("title_row")).contains("ports"), "Settings reads the same row")

            // A window opened after it, and the first one still, show the row without the ports.
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            app.expect(withoutPorts(await app.reply("title-menu", window: 2)), "the second window follows the setting")
            app.expect(withoutPorts(await app.reply("title-menu", window: 1)), "the first window keeps it")

            // Reset from the menu puts the row back, in the setting too.
            _ = await app.reply("title-menu", window: 1)
            app.expect(await app.reply("menu-pick Reset title row", window: 1), contains: "picked Reset title row")
            await app.until("title-menu", window: 2) { Instance.value("row", in: $0)?.contains("ports") == true }

            // Hidden again, and the app quit and started: the row is as it was left.
            _ = await app.reply("title-menu")
            await app.send(["menu-pick Open ports"])
            _ = await app.waitForConfig { $0.contains("\"title_row\"") && !$0.contains("\"ports\"") }
            try await app.restart()
            app.expect(withoutPorts(await app.until("title-menu") { $0.hasPrefix("title-menu row=") }), "the row is kept across a restart")
        }
    }
}
