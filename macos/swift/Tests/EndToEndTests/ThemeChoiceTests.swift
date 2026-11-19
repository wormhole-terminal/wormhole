import Foundation
import Testing

/// A theme chosen as a person chooses it: a click on its card in Settings → Appearance. The pick is written to `config.json`,
/// the window takes the theme, and both are still so after a relaunch.
extension EndToEnd {
    @Test("a click on a theme card picks it for its shelf, the window takes it, and the pick survives a relaunch")
    @MainActor func themeCardClickChoosesAndPersists() async throws {
        try await Instance.with { app in
            // One shelf, the dark one, and the window in dark: the card's pick is the theme on show.
            await app.send(["set theme_mode=\"dark\"", "settings-section appearance"])
            let cards = Readout(await app.until("theme-cards") { Readout($0).first("theme-cards")?["mode"] == "dark" })
            let before = Instance.value("theme", in: await app.reply("state")) ?? ""
            // A shelf shows four cards until Show all, the pick first; a value with a space would not survive `settings-press`'s
            // words, so the first card on show that is neither.
            guard let card = cards.under("shelf", "dark").prefix(4).first(where: { !$0.has("picked") && !($0["value"] ?? " ").contains(" ") }),
                let value = card["value"]
            else { return app.fail("no card to pick on the dark shelf:\n\(cards.text)") }

            // The card's click writes the shelf's key, `theme` for the dark one (settings_window/sections/theme_picker/card.rs:144).
            app.expect(await Self.press(app, "theme-card.\(value)"), contains: "settings-press theme-card.\(value)")
            let pattern = "\"theme\"\\s*:\\s*\"\(NSRegularExpression.escapedPattern(for: value))\""
            _ = await app.waitForConfig { $0.range(of: pattern, options: .regularExpression) != nil }
            let shown = await app.until("state") { (Instance.value("theme", in: $0) ?? before) != before }
            let theme = Instance.value("theme", in: shown) ?? ""
            app.expect(theme != before, "the window took the picked theme (was \(before)):\n\(shown)")
            let picked = Readout(await app.until("theme-cards") { Readout($0).under("shelf", "dark").contains { $0["value"] == value && $0.has("picked") } })
            app.expect(picked.under("shelf", "dark").filter { $0.has("picked") }.count == 1, "one pick on the shelf:\n\(picked.text)")

            // Over a relaunch: the setting, the card's mark and the window's theme are the same.
            try await app.restart()
            app.expect(await app.setting("theme"), contains: "\"\(value)\"")
            app.expect(Instance.value("theme", in: await app.reply("state")) == theme, "the window comes back in \(theme)")
            let again = Readout(await app.until("theme-cards") { Readout($0).first("theme-cards") != nil })
            app.expect(again.under("shelf", "dark").contains { $0["value"] == value && $0.has("picked") }, "the card is still the pick:\n\(again.text)")
        }
    }
}
