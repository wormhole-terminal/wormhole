import Foundation
import Testing

/// What the menus, the palette and the shortcut sheet offer (`menus`, `palette-rows`, `shortcut-rows`, `available`), as the
/// features and flags of the window's profile decide, and the panel's tab keys.
extension EndToEnd {
    /// The `shown` of `shortcut-rows`' head line, `<shown> of <total>`.
    static func shownShortcuts(_ said: String) -> Int { Int(said.split(separator: " ").first ?? "") ?? -1 }

    @Test("History switched off leaves the menus, the palette and the shortcut sheet; Persistent terminals adds its menu items")
    @MainActor func featureSwitchesAndMenus() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("menus"), contains: "Show History")
            app.expect(await app.reply("palette-rows show history"), contains: "<action-showHistory>")
            let sheet = Self.shownShortcuts(await app.reply("shortcut-rows history"))
            app.expect(sheet > 0, "the sheet lists History")

            await app.send(["set features={\"history\":false}"])
            await app.until("menus") { !$0.contains("Show History") }
            app.expect(!(await app.reply("palette-rows show history")).contains("<action-showHistory>"), "the palette has no Show History")
            app.expect(Self.shownShortcuts(await app.reply("shortcut-rows history")) < sheet, "the sheet lists fewer rows for history")

            await app.flags(["persistent_terminals": true])
            let menus = await app.until("menus") { $0.contains("Quit and End All Terminals") }
            app.expect(menus, contains: "Session Hosts")
            app.expect(await app.reply("available toggleSidebar"), contains: "toggleSidebar: true")
            app.expect(await app.reply("available bogusAction"), contains: "bogusAction: false")
        }
    }

    @Test("the panel's tab keys: Usage does nothing without an agent; under Zen a tab key changes the tab and it shows after")
    @MainActor func panelTabKeys() async throws {
        try await Instance.with { app in
            await app.send(["key ctrl-cmd-1"])
            await app.until("state") { $0.contains("tab=changes") }
            app.expect(!(await app.reply("menus")).contains("Show Usage"), "no agent on screen: no Show Usage")
            await app.send(["key ctrl-cmd-4"])
            app.expect(await app.reply("state"), contains: "tab=changes")

            app.expect(await app.reply("zen on"), contains: "zen on")
            await app.send(["key ctrl-cmd-3"])
            await app.until("state") { $0.contains("tab=notes") }
            app.expect(await app.reply("zen off"), contains: "zen off")
            app.expect(await app.reply("state"), contains: "tab=notes")
            app.expect(await app.reply("zen x"), contains: "error")
        }
    }

    @Test("the palette and the shortcut sheet close with Esc and the terminal has the keys again, with no click")
    @MainActor func overlaysGiveTheKeysBack() async throws {
        try await Instance.with { app in
            await app.send(["palette", "wait-idle 5000", "key esc", "wait-idle 5000"])
            app.expect((await app.reply("keys")).hasPrefix("focus=terminal"), "after the palette")
            await app.send(["shortcuts", "wait-idle 5000", "key esc", "wait-idle 5000"])
            app.expect((await app.reply("keys")).hasPrefix("focus=terminal"), "after the sheet")
            await app.send(["text back-in-the-terminal"])
            await app.screen(showing: "back-in-the-terminal")
        }
    }
}
