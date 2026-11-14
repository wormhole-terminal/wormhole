import Foundation
import Testing

/// The terminals as tabs read with `tab-bar` and `switcher [query]`, clicked with `tab-click <n|+>` (`Readout`'s shape).
/// TO-VERIFY.md lines 172, 176, 822, 824, 825.
extension EndToEnd {
    @MainActor static func tabBar(_ app: Instance, until done: (Readout) -> Bool) async -> Readout {
        Readout(await app.until("tab-bar") { done(Readout($0)) })
    }

    @Test("as tabs: each terminal is a tab with its ⌘ number; a click on one selects it and gives it the keys (172, 825)")
    @MainActor func tabsClickAndNumbers() async throws {
        try await Instance.with { app in
            await app.send(["set terminals_in=\"tabs\"", "key cmd-t", "key cmd-t"])
            let bar = await Self.tabBar(app) { $0.all("tab").count == 3 && $0.first("tabs")?.has("on-top") == true }
            app.expect(bar.first("tab-bar")?.has("shown") == true, "the bar shows:\n\(bar.text)")
            app.expect(bar.all("tab").allSatisfy { $0["number"] != nil }, "every tab has its ⌘ number:\n\(bar.text)")
            app.expect(bar.all("tab").filter { $0.has("selected") }.count == 1, "one tab is selected:\n\(bar.text)")
            guard let first = bar.all("tab").first?.words.first, let id = Int(first) else { return }
            app.expect(await app.reply("tab-click 1"), contains: "tab-click 1")
            app.expect(Listing(await app.until("rows") { Listing($0).selected == id }).selected == id, "the first tab's terminal is selected")
            await app.send(["text typed-into-the-tab"])
            await app.screen(showing: "typed-into-the-tab")
            app.expect(await app.reply("tab-click 9"), contains: "error: no tab 9")
        }
    }

    @Test("the switcher lists the groups as cards with their tiles and its key hints; its search marks a terminal by its tile and a group by a dot (176, 824)")
    @MainActor func tabSwitcher() async throws {
        try await Instance.with { app in
            await app.send(["set terminals_in=\"tabs\"", "key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let grouped = two.live.first else { return }
            await app.send(["group \(grouped)"])
            let listing = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            guard let group = listing.groups.first else { return }
            let switcher = Readout(await app.until("switcher") { Readout($0).all("card").count >= 2 })
            let card = switcher.all("card").first { $0.label == group.name }
            app.expect(card?.has("group") == true && card?["tiles"] == String(grouped), "the group's card with its terminal's tile:\n\(switcher.text)")
            app.expect(switcher.all("card").filter { $0.has("current") }.count == 1, "the open set is ringed:\n\(switcher.text)")
            let footer = switcher.first("footer")
            app.expect(footer?["keys"]?.contains("previous, next group") == true && footer?["range"]?.contains("9") == true, "the key hints:\n\(switcher.text)")

            let name = listing.row(grouped)?.title ?? ""
            let byTerminal = Readout(await app.reply("switcher \(name)"))
            app.expect(byTerminal.all("hit").contains { $0.words == ["terminal", String(grouped)] && $0["mark"] == "tile" }, "the terminal's hit wears its tile:\n\(byTerminal.text)")
            let byGroup = Readout(await app.reply("switcher \(group.name)"))
            app.expect(byGroup.all("hit").contains { $0.words.first == "set" && $0["mark"] == "dot" }, "the group's hit a dot:\n\(byGroup.text)")
            app.expect(Readout(await app.reply("switcher zzqqxx")).first("none") != nil, "nothing matches")

            // The switcher on screen: open, its own field's search (empty), its cards.
            await app.send(["switcher open"])
            let shown = Readout(await app.until("switcher") { Readout($0).first("switcher")?.has("open") == true })
            app.expect(shown.first("switcher")?["query"] == "" && shown.all("card").count >= 2, "the open switcher, its field empty:\n\(shown.text)")
        }
    }

    @Test("the tab bar's + in a group without a folder asks the folder question before the terminal opens (822)")
    @MainActor func tabPlusAsksGroupFolder() async throws {
        try await Instance.with { app in
            let folders = ["tab-a", "tab-b"].map { app.scratch.home.appendingPathComponent($0) }
            for folder in folders { try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true) }
            await app.send(["set terminals_in=\"tabs\""])
            for folder in folders { await app.terminal(in: folder) }
            let ids = (await app.rows()).live
            await app.send(["group \(ids.suffix(2).map(String.init).joined(separator: " "))", "select \(ids.last ?? 0)"])
            await app.until("rows") { Listing($0).groups.first?.count == 2 }
            let before = (await app.rows()).live.count
            app.expect(await app.reply("tab-click +"), contains: "tab-click +")
            app.expect(await app.asked(), contains: "title=")
            app.expect((await app.rows()).live.count == before, "nothing opens while the question is up")
            await app.send(["group-ask later"])
            await app.until("rows") { Listing($0).live.count == before + 1 }
        }
    }
}
