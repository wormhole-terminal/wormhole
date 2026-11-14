import Foundation
import Testing

/// The interface's words from the core's catalogs in the language in use, switched while the app runs (`language en-XA`, the
/// pseudo-language: every message inside `⟦ ⟧`, `wormhole-text/src/pseudo.rs:44`). GPUI redraws every window in the new
/// language (`app/language.rs:43`) and leaves the native menus as they were (`app/language.rs:10`), so the menus are not asked here.
extension EndToEnd {
    @Test("in en-XA the palette's commands and a Settings page's choices are the pseudo-language's; back in English after")
    @MainActor func pseudoLanguageInPaletteAndSettings() async throws {
        try await Instance.with { app in
            let commands: (String) -> [Substring] = { $0.split(separator: "\n").filter { $0.contains("<action-") } }
            let english = commands(await app.reply("palette-rows"))
            app.expect(!english.isEmpty && !english.contains { $0.contains("⟦") }, "English first:\n\(english.joined(separator: "\n"))")

            app.expect(await app.reply("language en-XA"), contains: "language en-XA")
            let pseudo = commands(await app.reply("palette-rows"))
            app.expect(!pseudo.isEmpty && pseudo.allSatisfy { $0.contains("⟦") }, "every command's title is the catalog's, in en-XA:\n\(pseudo.joined(separator: "\n"))")
            // The default shell's choices are the core's words (`Login shell` and the shells found).
            let page = await Self.page(app, "terminal") { $0.under("row", "default-shell").contains { $0.kind == "option" } }
            let options = page.under("row", "default-shell").filter { $0.kind == "option" }
            app.expect(options.contains { $0.label.contains("⟦") }, "Login shell in en-XA:\n\(page.text)")

            await app.send(["language en"])
            app.expect(!commands(await app.reply("palette-rows")).contains { $0.contains("⟦") }, "English again")
        }
    }
}
