import Foundation
import Testing

extension EndToEnd {
    @Test("the command palette opens, takes a query and runs an action")
    @MainActor func palette() async throws {
        try await Instance.with { app in
            // An action that does not depend on the window's size: Split Right is not offered where a pane has no room for a
            // second one (the core's rule, the GPUI palette's too), as on a small display.
            let found = await app.reply("palette-rows new terminal")
            app.expect(found, contains: "New terminal")
            // The list is the core's: settings come before actions, so the action is some rows down (↓), as a person would go.
            let results = found.split(separator: "\n").filter { !$0.hasPrefix("#") }
            guard let above = results.firstIndex(where: { $0.contains("<action-newTerminal>") }) else {
                return app.fail("New Terminal among the rows:\n\(found)")
            }
            let before = (await app.rows()).live.count
            // The window settles after each step (`wait-idle`) so the field has the keys before the text comes.
            await app.send(["palette", "wait-idle 5000", "text new terminal", "wait-idle 5000"] + Array(repeating: "key down", count: above) + ["key return"])
            let rows = await app.until("rows") { Listing($0).live.count == before + 1 }
            if Listing(rows).live.count != before + 1 { app.fail("New Terminal did not run from the palette:\n\(rows)\n\(await app.keysAndFrame())") }
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
            // The Settings window opens and scrolls to the row a moment after it is asked to: the row is there, and where it will
            // stay, once two asks in a row give the same line.
            var last: Substring?
            let page = await app.until("settings-rows") { rows in
                let line = rows.split(separator: "\n").first { $0.hasPrefix("status-bar | ") }
                defer { last = line }
                return line != nil && line == last
            }
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
