import Foundation
import Testing

/// Appearance's theme cards read with `theme-cards [filter]` (`Readout`'s shape): the shelves the mode asks for, the filter and
/// its placeholder, a newer file's mark, the hidden themes with their kind. TO-VERIFY.md lines 449, 868, 869, 871, 872, 873.
extension EndToEnd {
    @Test("one shelf under Dark or Light, two under Match system; the filter finds a card by its name, says when none match (449, 868, 872)")
    @MainActor func themeShelvesAndFilter() async throws {
        try await Instance.with { app in
            await app.send(["set theme_mode=\"dark\""])
            let dark = Readout(await app.until("theme-cards") { Readout($0).first("theme-cards")?["mode"] == "dark" })
            app.expect(dark.all("shelf").map { $0.words.first ?? "" } == ["dark"], "one shelf under Dark:\n\(dark.text)")
            await app.send(["set theme_mode=\"system\""])
            let both = Readout(await app.until("theme-cards") { Readout($0).first("theme-cards")?["mode"] == "system" })
            app.expect(both.all("shelf").map { $0.words.first ?? "" } == ["dark", "light"], "two shelves under Match system:\n\(both.text)")

            // "Filter 60 themes": the count is every card of the shelves on show.
            let cards = both.all("card")
            let placeholder = both.first("theme-cards")?["placeholder"] ?? ""
            app.expect(placeholder.hasPrefix("Filter") && placeholder.contains("\(cards.count)"), "the placeholder counts the cards:\n\(both.text)")
            app.expect(cards.filter { $0.has("picked") }.count == 2, "one pick a shelf, listed first:\n\(both.text)")
            guard let some = cards.first(where: { !$0.has("picked") }) else { return }
            let filtered = Readout(await app.reply("theme-cards \(some.label)"))
            app.expect(filtered.all("card").contains { $0.label == some.label }, "the filter finds \(some.label):\n\(filtered.text)")
            let none = Readout(await app.reply("theme-cards zzqqxx"))
            app.expect(none.first("none")?.label == "No themes match the filter.", "nothing matches:\n\(none.text)")
        }
    }

    @Test("a theme file made by a newer Wormhole wears its mark (869)")
    @MainActor func themeCardNewerMark() async throws {
        try await Instance.with { app in
            // A file a newer Wormhole made has this version's shape and more (a hand-written `extends` under a version
            // number is no theme at all, so it gets no card).
            try await Self.newerThemeFile(app, named: "Newer")
            // The card is found by its file: what a card is called is the theme's own business.
            let isNewer: (Readout.Line) -> Bool = { ($0["file"] ?? "").hasSuffix("/Newer.json") }
            let listed = Readout(await app.until("theme-cards") { Readout($0).all("card").contains(where: isNewer) })
            let card = listed.all("card").first(where: isNewer)
            app.expect(card?["newer"].flatMap { Int($0) }.map { $0 > 0 } == true, "the card says values were skipped:\n\(listed.text)")
        }
    }

    @Test("a hidden theme leaves its shelf and is listed under Hidden themes with Dark or Light (871, 873)")
    @MainActor func themeCardsHidden() async throws {
        try await Instance.with { app in
            await app.send(["set theme_mode=\"system\""])
            let before = Readout(await app.until("theme-cards") { Readout($0).first("theme-cards")?["mode"] == "system" })
            guard let card = before.all("card").first(where: { !$0.has("picked") }), let value = card["value"] else { return }
            await app.send(["set hidden_themes=[\"\(value)\"]"])
            let after = Readout(await app.until("theme-cards") { !Readout($0).all("hidden").isEmpty })
            app.expect(!after.all("card").contains { $0["value"] == value }, "\(value) left its shelf:\n\(after.text)")
            let hidden = after.all("hidden").first { $0["value"] == value }
            app.expect(hidden?["kind"] == "Dark" || hidden?["kind"] == "Light", "listed with its kind:\n\(after.text)")
        }
    }
}
