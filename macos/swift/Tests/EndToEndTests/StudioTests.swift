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
            app.expect(await app.late("studio-open own:Mine"), contains: "name=Mine")

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
            let saved = await app.late("studio-save")
            app.expect(saved, contains: "name=Dawn")
            app.expect(saved, contains: "dirty=false")
            let versions = await app.reply("studio-versions")
            app.expect(versions, contains: "versions=2")
            app.expect(versions, contains: "pinned")
        }
    }
}
