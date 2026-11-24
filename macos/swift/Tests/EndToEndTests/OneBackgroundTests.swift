import Foundation
import Testing

/// The one fill (`one_background`), read from what the app says it paints (`backdrop`), not from the switch the command sets: on by
/// default, a row in this app's Settings (the GPUI app's has none: core hides it there, `platform::front_end`), listed on the Changed
/// page while it is off, and written to `config.json`.
extension EndToEnd {
    @Test("One background: on by default; the row is in Settings; off and on again, the window's backdrop and the file follow")
    @MainActor func oneBackground() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("backdrop"), contains: "one_fill=true")
            let appearance = await Self.page(app, "appearance") { $0.line("row", "one-background") != nil }
            app.expect(appearance.line("row", "one-background") != nil, "the row is on Appearance in this app:\n\(appearance.text)")

            await app.send(["set one_background=false"])
            await app.until("backdrop") { $0.contains("one_fill=false") }
            _ = await app.waitForConfig { $0.range(of: #""one_background"\s*:\s*false"#, options: .regularExpression) != nil }
            let changed = await Self.page(app, "changed") { $0.line("row", "one-background") != nil }
            app.expect(changed.line("row", "one-background") != nil, "off, it is a changed row:\n\(changed.text)")

            await app.send(["set one_background=true"])
            await app.until("backdrop") { $0.contains("one_fill=true") }
            _ = await app.waitForConfig { $0.range(of: #""one_background"\s*:\s*true"#, options: .regularExpression) != nil }
            await app.until("settings-page") { Readout($0).line("row", "one-background") == nil }
        }
    }
}
