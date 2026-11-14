import Foundation
import Testing

/// The control folder's own tools (X12) and the interface's words in another language (X07): a batch's timing, a window's own
/// control file, the read-and-drive commands by row, the inspector, and the pseudo-language `en-XA`.
extension EndToEnd {
    @Test("delay holds the batch; each window's own control file acts on that window; close-window closes the one it names")
    @MainActor func controlFilesPerWindow() async throws {
        try await Instance.with { app in
            let start = Date()
            app.expect(await app.reply("delay 300"), contains: "delay 300")
            app.expect(Date().timeIntervalSince(start) >= 0.3, "the batch waited")
            app.expect(await app.reply("delay soon"), contains: "error")

            // `cmd2` with one window: the line acts on the window there is.
            app.expect(await app.reply("state", window: 2), contains: "terminals=1")
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            // F6 in the second window moves its focus only.
            let moved = await app.send(["key f6", "keys"], window: 2)
            app.expect(!(moved.last ?? "").hasPrefix("focus=terminal"), "window 2's focus moved:\n\(moved.joined(separator: "\n"))")
            app.expect((await app.reply("keys", window: 1)).hasPrefix("focus=terminal"), "window 1's focus stayed on its terminal")

            await app.send(["close-window"], window: 2)
            let asked = await app.reply("question")
            if asked.contains("title=") { await app.send(["question confirm"]) }
            await app.until("sessions") { $0.split(separator: "\n").count == 1 }
        }
    }

    @Test("rows closed and hidden by number, layouts, shells, Zen, said back as GPUI's devctl says them")
    @MainActor func readAndDriveByRow() async throws {
        try await Instance.with { app in
            app.expect(Panes(await app.reply("panes")).isSplit == false, "one terminal is no split")
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            app.expect(await app.reply("layout columns"), contains: "layout columns")
            await app.until("panes") { Panes($0).isSplit }
            await app.send(["layout single"])
            await app.until("panes") { !Panes($0).isSplit }
            app.expect(await app.reply("layout x"), contains: "error: expected columns|rows|grid|single")

            let shells = await app.reply("shells")
            app.expect(shells.split(separator: "\n").contains { $0.hasPrefix("-\t") }, "the login shell is listed as -:\n\(shells)")
            let folder = app.scratch.home.appendingPathComponent("shell-here")
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            await app.send(["new-shell - \(folder.path)"])
            let three = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            app.expect(three.rows.contains { $0.folder.hasSuffix("shell-here") }, "the new shell is in the folder:\n\(three.text)")

            app.expect(await app.reply("close 9"), contains: "error: no row 9")
            app.expect(await app.reply("close 2"), contains: "close 2")
            await app.until("rows") { Listing($0).live.count == 2 }
            app.expect((await app.reply("question")).contains("title=") == false, "closed by number without a question")
            app.expect(await app.reply("agent-hide 1"), contains: "agent-hide 1")
            app.expect(await app.reply("agent-hide x"), contains: "error")

            app.expect(await app.reply("zen"), contains: "zen off")
            app.expect(await app.reply("zen on"), contains: "zen on")
            app.expect(await app.reply("zen toggle"), contains: "zen off")
        }
    }

    @Test("the inspector pins a marked element, says where it is built, and copies its note to the instance's own clipboard")
    @MainActor func inspector() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("inspect copy"), contains: "error: nothing is picked")
            app.expect(await app.reply("inspect on"), contains: "inspect on")
            var pinned = ""
            for point in ["150 66", "60 20", "400 20", "150 200", "700 20"] where !pinned.contains(".swift:") {
                pinned = await app.reply("inspect \(point) 2")
            }
            app.expect(pinned, contains: ".swift:")
            app.expect(await app.reply("inspect copy"), contains: "inspect copied")
            app.expect(await app.reply("pasteboard"), contains: ".swift:")
            app.expect(await app.reply("inspect off"), contains: "inspect off")
            app.expect(await app.reply("inspect pick"), contains: "error: inspect mode is off")
        }
    }

    @Test("en-XA puts the interface's words, the core's and the app's, in the pseudo-language; English comes back")
    @MainActor func pseudoLanguage() async throws {
        try await Instance.with { app in
            // The palette's section titles are catalog words. The shortcut sheet's rows are not: they are the core keymap's own
            // labels (`keys::keymap::sheet`), English in the GPUI app's sheet too, so they are no sign of the language.
            app.expect(!(await app.reply("palette-rows show history")).contains("⟦"), "English first")
            app.expect(await app.reply("language en-XA"), contains: "language en-XA")
            app.expect(await app.reply("palette-rows show history"), contains: "⟦")
            // A question the app puts: its words come from the catalog too.
            let size = Self.number(await app.setting("font_size")) ?? 13
            await app.send(["set font_size=\(size + 1)", "settings-reset-confirm"])
            app.expect(await app.until("question") { $0.contains("title=") }, contains: "⟦")
            await app.send(["question cancel"])
            await app.send(["language en"])
            app.expect(!(await app.reply("palette-rows show history")).contains("⟦"), "English again")
        }
    }
}
