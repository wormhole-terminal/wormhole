import Foundation
import Testing

extension EndToEnd {
    @Test("the command palette opens, takes a query and runs an action")
    @MainActor func palette() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("palette-rows split right"), contains: "Split Right")
            // The window settles after each step (`wait-idle`) so the field has the keys before the text comes.
            await app.send(["palette", "wait-idle 5000", "text split right", "wait-idle 5000", "key return"])
            let split = await app.until("panes") { Panes($0).isSplit }
            app.expect(Panes(split).ids.count == 2, "Split Right ran from the palette:\n\(split)")
        }
    }

    @Test("Settings finds a word, and a toggle changes the window and config.json")
    @MainActor func settings() async throws {
        try await Instance.with { app in
            let found = await app.reply("settings-search status bar")
            let hits = Instance.value("hits", in: found).flatMap { Int($0) } ?? 0
            app.expect(hits > 0, "a search for the status bar has hits:\n\(found)")
            app.expect(found, contains: "status-bar")
            await app.send(["settings-search", "settings-reveal status-bar"])
            let page = await app.reply("settings-rows")
            let line = page.split(separator: "\n").first { $0.hasPrefix("status-bar | ") }
            let frame = line?.components(separatedBy: " | ").last?.split(separator: " ").compactMap { Double($0) } ?? []
            app.expect(frame.count == 4, "the status bar row on show with its frame:\n\(page)")
            guard frame.count == 4 else { return }
            let before = Instance.value("status_bar", in: await app.reply("state")) ?? "?"
            let wanted = before == "true" ? "false" : "true"
            // The click goes to the Settings window, on the middle of the row's control.
            let (x, y) = (Int(frame[0] + frame[2] / 2), Int(frame[1] + frame[3] / 2))
            await app.send(["pointer-window Settings", "click \(x) \(y)", "pointer-window"])
            await app.until("state") { Instance.value("status_bar", in: $0) == wanted }
            _ = await app.waitForConfig { $0.range(of: "\"status_bar\"\\s*:\\s*\(wanted)", options: .regularExpression) != nil }
        }
    }
}
