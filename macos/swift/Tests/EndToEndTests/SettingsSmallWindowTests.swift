import Foundation
import Testing

/// Settings on a small display: the window is shorter than a page (a hosted runner's display gave it 660 points), so a
/// control low on the page is out of sight until the page is scrolled. `settings-press` and `settings-record` scroll to the
/// control first, as a person does; before, the press landed outside the window and the scenario failed only there.
extension EndToEnd {
    @Test("in a Settings window shorter than its page, a button below the fold is scrolled to and pressed")
    @MainActor func settingsPressBelowTheFold() async throws {
        try await Instance.with { app in
            await app.send(["settings-section appearance", "settings-size 960x560"])
            let rows = await app.until("settings-rows") { $0.contains("open-studio | ") }
            let line = rows.split(separator: "\n").first { $0.hasPrefix("open-studio | ") }.map(String.init) ?? ""
            let top = line.split(separator: "|").last?.split(separator: " ").compactMap { Double($0) }.dropFirst().first ?? 0
            app.expect(top > 560, "the button starts below what the window shows (y \(top)):\n\(line)")

            let pressed = await app.reply("settings-press open-studio")
            app.expect(!pressed.contains("nothing at"), "the press found the button: \(pressed)")
            await app.until("studio-select") { !$0.hasPrefix("error") }
        }
    }
}
