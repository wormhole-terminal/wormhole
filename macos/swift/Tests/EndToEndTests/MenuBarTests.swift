import Foundation
import Testing

extension EndToEnd {
    @Test("Edit's command block items are greyed until the shell marks a prompt")
    @MainActor func blockItemsFollowShellIntegration() async throws {
        try await Instance.with { app in
            // `cat` is a shell with no integration: no prompt mark, so nothing to jump to (GPUI binds them only then, `has_blocks`).
            app.expect(await app.reply("available previousPrompt"), contains: "previousPrompt: false")
            // A prompt mark as a shell's integration writes it (OSC 133;A): `cat` writes the line back, and the terminal reads it.
            await app.send(["type \u{1B}]133;A\u{07}\\r"])
            await app.until("available previousPrompt") { $0.contains("previousPrompt: true") }
            app.expect(await app.reply("available copyOutput"), contains: "copyOutput: true")
        }
    }
}
