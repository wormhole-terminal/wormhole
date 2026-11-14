import Foundation
import Testing

extension EndToEnd {
    @Test("the Studio marks, filters and lists one contrast answer, and a renamed theme keeps its saved versions")
    @MainActor func studio() async throws {
        try await Instance.with { app in
            // A theme of the person's own in the older format and with no id: its first save converts it.
            // The themes folder is the active profile's, beside its `config.json`.
            guard let profile = app.configFile()?.deletingLastPathComponent() else { return app.fail("the app has written no config.json yet") }
            try app.scratch.write(profile, "themes/Mine.json", "{ \"name\": \"Mine\", \"extends\": \"Nord\" }\n")
            await app.send(["studio"])
            await app.until("state") { $0.contains("\nstudio dirty=") }
            app.expect(await app.later("studio-open own:Mine", answer: "studio-open own:Mine"), contains: "name=Mine")

            // Muted text too dark on Nord: core's one report marks the token (the Low count), the Low filter keeps it and
            // the card lists its pair.
            await app.send(["studio-select text.muted"])
            let set = await app.reply("studio-set #333847")
            app.expect(set, contains: "ok=true")
            let low = set.range(of: "low: ").map { Int(set[$0.upperBound...].prefix { $0.isNumber }) ?? 0 } ?? 0
            app.expect(low > 0, "the token is counted as falling short:\n\(set)")
            app.expect(await app.reply("studio-filter low muted"), contains: "shown=1")
            let report = await app.reply("studio-do state")
            app.expect((Int(Instance.value("issues", in: report) ?? "") ?? 0) > 0, "the card lists the pair:\n\(report)")
            app.expect(await app.reply("state"), contains: "token=text.muted origin=yours")

            // Renamed and saved: the theme keeps the id it answered to, so the original kept by the save is still listed.
            await app.send(["studio-filter all", "studio-rename Dawn"])
            let saved = await app.later("studio-save", answer: "studio-save")
            app.expect(saved, contains: "name=Dawn")
            app.expect(saved, contains: "dirty=false")
            let versions = await app.reply("studio-versions")
            app.expect(versions, contains: "versions=2")
            app.expect(versions, contains: "pinned")
            // The Versions sheet is up over the Studio, and Done (`close`) takes it down.
            app.expect(await app.reply("state"), contains: "modal=versions")
            await app.send(["studio-versions close"])
            app.expect(await app.reply("state"), contains: "modal=-")
        }
    }

    @Test("a theme of one appearance becomes a pair, as one undo step; the Low filter and a pair's chip answer")
    @MainActor func studioPair() async throws {
        try await Instance.with { app in
            await app.send(["studio"])
            await app.until("studio-select") { !$0.hasPrefix("error") }
            await app.later("studio-open new", answer: "studio-open new")
            let single = await app.reply("studio-theme-menu")
            app.expect(!single.contains("addable=-"), "a new theme has one appearance to add:\n\(single)")
            app.expect(await app.reply("studio-add-appearance"), contains: "studio-add-appearance")
            app.expect(await app.reply("studio-theme-menu"), contains: "addable=-")
            app.expect(await app.reply("studio-add-appearance"), contains: "error: the theme has both appearances")
            await app.send(["studio-undo"])
            app.expect(!(await app.reply("studio-theme-menu")).contains("addable=-"), "undo takes the second appearance away")
            await app.send(["studio-redo"])
            app.expect(await app.reply("studio-theme-menu"), contains: "addable=-")

            app.expect(await app.reply("studio-appearance light"), contains: "studio-appearance light")
            app.expect(await app.reply("studio-chip primary"), contains: "place=")
            let low = await app.reply("studio-filter low")
            app.expect(Instance.value("shown", in: low) != nil, "the Low filter says how many it shows:\n\(low)")
            app.expect(await app.reply("studio-filter all"), contains: "shown=")
        }
    }

    @Test("a hand-written theme is saved in the new format with its old text kept; one made by a newer Wormhole is never written")
    @MainActor func studioOlderAndNewerFiles() async throws {
        try await Instance.with { app in
            let themes = app.scratch.data.appendingPathComponent("themes")
            try app.scratch.write(themes, "Old.json", ##"{"name": "Old", "extends": "Nord", "ui": {"primary": "#ff0000"}}"##)
            let newer = ##"{"version": 9, "name": "Newer", "extends": "Nord", "from_the_future": true, "ui": {"primary": "#00ff00"}}"##
            try app.scratch.write(themes, "Newer.json", newer)
            await app.send(["studio"])
            await app.until("studio-select") { !$0.hasPrefix("error") }

            app.expect(await app.later("studio-open own:Old", answer: "studio-open own:Old"), contains: "name=Old")
            await app.send(["studio-select primary", "studio-set #ff0011"])
            let converted = await app.later("studio-save", answer: "studio-save")
            app.expect(converted, contains: "new theme format")
            let old = (try? String(contentsOf: themes.appendingPathComponent("Old.json"), encoding: .utf8)) ?? ""
            app.expect(old.contains("\"version\"") && old.lowercased().contains("ff0011"), "saved as version 1 with the edit:\n\(old)")
            let versions = await app.reply("studio-versions")
            app.expect((Instance.value("versions", in: versions).flatMap { Int($0) } ?? 0) >= 2, "the hand-written text is kept:\n\(versions)")
            await app.send(["studio-versions close"])

            await app.later("studio-open own:Newer", answer: "studio-open own:Newer")
            app.expect(await app.reply("studio-theme-menu"), contains: "newer=true")
            await app.send(["studio-select primary", "studio-set #112233"])
            await app.later("studio-save", answer: "studio-save")
            let after = try? String(contentsOf: themes.appendingPathComponent("Newer.json"), encoding: .utf8)
            app.expect(after == newer, "the newer file is untouched")
        }
    }
}
