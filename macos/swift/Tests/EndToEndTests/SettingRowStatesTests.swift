import Foundation
import Testing

/// What a setting row says about its value (`style::setting`'s states) and what VoiceOver hears of the theme cards, read
/// through `settings-page`, `settings-rows` and `a11y`. TO-VERIFY.md, lane parts, wave 11.
extension EndToEnd {
    @Test("a row a trusted project sets carries the project's name, and neither the changed mark nor a reset arrow, though the person's value differs")
    @MainActor func settingsProjectRowNotChanged() async throws {
        try await Instance.with(auth: "allow", config: #"{"font_size": 23}"#) { app in
            let repo = try app.scratch.repository("sets-size")
            try app.scratch.project(repo, files: ["config.json": #"{"font_size": 19}"#])
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            let page = await Self.page(app, "fonts") { $0.line("row", "font-terminal")?["project"] != nil }
            let row = page.line("row", "font-terminal")
            app.expect(row?["project"] == "sets-size", "the project's name on the row:\n\(page.text)")
            app.expect(row?.has("changed") == false, "no changed mark on a row the project sets:\n\(page.text)")
            // The reset arrow is recorded while it is there (hidden until the pointer comes); a project-set row has none.
            let rows = await app.until("settings-rows") { $0.contains("font-terminal | ") }
            app.expect(!rows.contains("font-terminal.reset | "), "no reset arrow on a row the project sets:\n\(rows)")
            // The person's own value is kept: the arrow would have put it back.
            app.expect(await app.setting("font_size"), contains: "23")
        }
    }

    @Test("the theme cards of a shelf are radio buttons in a group named by the shelf, the pick chosen (choice_card)")
    @MainActor func themeCardsAreRadioButtons() async throws {
        try await Instance.with { app in
            await app.send(["set theme_mode=\"dark\"", "settings-section appearance"])
            let cards = Readout(await app.until("theme-cards") { Readout($0).all("card").contains { $0.has("picked") } })
            guard let picked = cards.all("card").first(where: { $0.has("picked") })?.label else { return app.fail("a picked card:\n\(cards.text)") }
            let tree = await app.reply("a11y Settings")
            let lines = tree.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
            // SwiftUI builds its part of the tree only for an assistive app (DRIVING.md): where it did, the cards are radios.
            guard lines.contains(where: { $0.contains("\"\(picked)\"") }) else { return }
            app.expect(lines.contains { $0.hasPrefix("RadioGroup") && $0.contains("\"In Dark Mode\"") }, "the shelf is a radio group named by its heading:\n\(tree)")
            let radio = lines.first { $0.hasPrefix("RadioButton") && $0.contains("\"\(picked)\"") }
            app.expect(radio?.contains(" = 1") == true, "the pick is a chosen radio button:\n\(tree)")
            app.expect(!lines.contains { $0.hasPrefix("Button \"\(picked)\"") }, "no card is a plain button:\n\(tree)")
        }
    }
}
