import Foundation
import Testing

/// The title row's right-click menu read with `title-menu` and chosen with `menu-pick <n|title>` (its lines are `menu`'s:
/// `n. title [x] [off]`, `---`). TO-VERIFY.md lines 330, 331.
extension EndToEnd {
    /// The item lines of a `title-menu` reply, without its head line.
    static func menuItems(_ said: String) -> [String] { said.split(separator: "\n").dropFirst().map(String.init) }

    @Test("a part unticked in the title row's menu leaves it; under Zen the parts, Reset and Customize are off (330)")
    @MainActor func titleMenuParts() async throws {
        try await Instance.with { app in
            let menu = Self.menuItems(await app.reply("title-menu"))
            app.expect(menu.contains("---") && menu.last?.contains("Customize title row") == true, "the menu's items:\n\(menu)")
            guard let shown = menu.first(where: { $0.hasSuffix("[x]") }), let n = shown.split(separator: ".").first else { return app.fail("a shown part:\n\(menu)") }
            app.expect(await app.reply("menu-pick \(n)"), contains: "picked")
            let after = Self.menuItems(await app.until("title-menu") { Self.menuItems($0).first { $0.hasPrefix("\(n). ") }?.hasSuffix("[x]") == false })
            app.expect(after.contains { $0.contains(". Reset title row") && !$0.contains("[off]") }, "Reset is on once the row differs:\n\(after)")

            app.expect(await app.reply("zen on"), contains: "zen on")
            let zen = Self.menuItems(await app.reply("title-menu")).filter { $0 != "---" }.dropFirst()
            app.expect(zen.allSatisfy { $0.hasSuffix("[off]") }, "every item is off under Zen:\n\(zen)")
            app.expect(await app.reply("menu-pick \(n)"), contains: "is off")
        }
    }

    @Test("Customize title row… opens Settings on the Title row setting, marked (331)")
    @MainActor func titleMenuCustomize() async throws {
        try await Instance.with { app in
            _ = await app.reply("title-menu")
            app.expect(await app.reply("menu-pick Customize title row…"), contains: "picked")
            let page = Readout(await app.until("settings-page") { Readout($0).line("row", "title-row")?.has("revealed") == true })
            app.expect(page.first("settings-page")?.words.first == "appearance", "on Appearance:\n\(page.text)")
        }
    }
}
