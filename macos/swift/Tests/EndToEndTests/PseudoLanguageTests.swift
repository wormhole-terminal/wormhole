import Foundation
import Testing

/// The interface's words from the core's catalogs in the language in use, switched while the app runs (`language en-XA`, the
/// pseudo-language: every message inside `⟦ ⟧`, `wormhole-text/src/pseudo.rs:44`). GPUI redraws every window in the new
/// language (`app/language.rs:43`) and leaves the native menus as they were (`app/language.rs:10`), so the menus are not asked here.
extension EndToEnd {
    @Test("in en-XA the palette's catalog words and a Settings page's choices are the pseudo-language's; back in English after")
    @MainActor func pseudoLanguageInPaletteAndSettings() async throws {
        try await Instance.with { app in
            let commands: (String) -> [Substring] = { $0.split(separator: "\n").filter { $0.contains("<action-") } }
            let english = commands(await app.reply("palette-rows"))
            app.expect(!english.isEmpty && !english.contains { $0.contains("⟦") }, "English first:\n\(english.joined(separator: "\n"))")

            app.expect(await app.reply("language en-XA"), contains: "language en-XA")
            // A command's title is the core keymap's own label, English in the GPUI app's palette too (as the shortcut sheet's
            // rows are); what the catalog gives is the section's title and a command's second line.
            let listed = await app.reply("palette-rows")
            let pseudo = commands(listed)
            app.expect(listed.contains("# ⟦"), "the section's title is the catalog's, in en-XA:\n\(listed)")
            app.expect(pseudo.contains { $0.contains(" - ⟦") }, "a command's second line is the catalog's, in en-XA:\n\(pseudo.joined(separator: "\n"))")
            // The default shell's choices are the core's words (`Login shell` and the shells found).
            let page = await Self.page(app, "terminal") { $0.under("row", "default-shell").contains { $0.kind == "option" } }
            let options = page.under("row", "default-shell").filter { $0.kind == "option" }
            // An option's line is its key, then its words.
            let lines = page.text.split(separator: "\n").filter { $0.trimmingCharacters(in: .whitespaces).hasPrefix("option ") }
            app.expect(!options.isEmpty && lines.contains { $0.contains("⟦") }, "Login shell in en-XA:\n\(page.text)")

            await app.send(["language en"])
            app.expect(!commands(await app.reply("palette-rows")).contains { $0.contains("⟦") }, "English again")
        }
    }
}
