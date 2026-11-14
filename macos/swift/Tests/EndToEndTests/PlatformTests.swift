import Foundation
import Testing

/// What reaches the app from the system, driven through the seams a test instance has instead: a folder or a `.command` opened
/// with it (`open-url`), a second launch, `SIGTERM`, the menu bar extra's, the Dock's and the quick panel's rows (`ambient`,
/// `quick-*`, `ambient-run`), and the notice chip and banners (`notice`, `notify-sim`, `notice-log`: logged, never shown).
extension EndToEnd {
    @Test("a folder and a .command opened with the app become terminals there; a web address and a missing file are refused")
    @MainActor func openedFromTheSystem() async throws {
        try await Instance.with(shell: .zsh) { app in
            let folder = app.scratch.home.appendingPathComponent("opened-folder")
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            let script = folder.appendingPathComponent("run-me.command")
            try Data("#!/bin/sh\necho cmd-$((3*3))\n".utf8).write(to: script)
            try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: script.path)
            let start = (await app.rows()).live.count

            await app.send(["open-url \(folder.absoluteString)"])
            let opened = Listing(await app.until("rows") { Listing($0).live.count == start + 1 })
            app.expect(opened.rows.contains { $0.folder.hasSuffix("opened-folder") }, "a terminal in the folder:\n\(opened.text)")
            await app.send(["open-url \(script.absoluteString)"])
            await app.until("rows") { Listing($0).live.count == start + 2 }
            await app.screen(showing: "cmd-9")

            await app.send(["open-url https://example.com/x", "open-url file:///no/such/file.command"])
            app.expect((await app.rows()).live.count == start + 2, "nothing opened for them")
            app.expect(await app.reply("second-launch hide"), contains: "second-launch windows=1")
        }
    }

    @Test("SIGTERM quits as ⌘Q does, without a question, and the terminals are back at the next launch")
    @MainActor func sigtermQuits() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "type still-here\\n"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.screen(showing: "still-here")
            app.expect(await app.signal(SIGTERM), "the app ended on SIGTERM")
            try await app.start()
            await app.until("rows") { Listing($0).live.count == 2 }
        }
    }

    @Test("a waiting agent in a folded group is listed by the menu bar extra, the Dock and the quick panel, and their rows run")
    @MainActor func ambientRows() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let waiting = two.live.first, let other = two.live.last else { return }
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: waiting, becomes: "needsInput")
            await app.send(["group \(waiting)"])
            let grouped = Listing(await app.until("rows") { !Listing($0).groups.isEmpty })
            if let group = grouped.groups.first?.id { await app.send(["fold \(group)"]) }

            let listed = await app.until("ambient") { $0.contains("badge needs_you=1") }
            app.expect(listed, contains: "quick")
            let quick = await app.reply("quick-open")
            app.expect(quick, contains: "query=\"\"")
            app.expect(await app.reply("quick-key escape"), contains: "closed")

            await app.send(["select \(other)", "ambient-run jump-to-waiting"])
            await app.until("rows") { Listing($0).selected == waiting }
            await app.send(["ambient-run new-terminal"])
            await app.until("rows") { Listing($0).live.count == 3 }
            app.expect(await app.reply("ambient-run nonsense"), contains: "error")
        }
    }

    @Test("the notice chip: a sticky warning, its explanation and its close; banners decided by the core and logged")
    @MainActor func noticesAndBanners() async throws {
        try await Instance.with { app in
            await app.flags(["notices": true])
            app.expect(await app.reply("notice-click"), contains: "error: no chip to click")
            await app.send(["notice sticky"])
            app.expect(await app.until("notice chip") { $0.contains("sticky=true") }, contains: "info=false")
            await app.send(["notice-click info"])
            await app.until("notice chip") { $0.contains("info=true") }
            await app.send(["notice-click close"])
            await app.until("notice chip") { $0.contains("shown=none") }

            // A copy's chip says how much was copied.
            await app.send(["notice 12"])
            app.expect(await app.until("notice chip") { !$0.contains("shown=none") }, contains: "12")

            let decided = await app.reply("notify-sim needs-you 1 post")
            app.expect(decided, contains: "-> Banner")
            app.expect(await app.until("notice-log") { $0.contains("wormhole/sim/") }, contains: "level=")
            app.expect(await app.reply("notify-sim bogus"), contains: "needs-you, agent-done")
            app.expect(await app.reply("notify-permission denied"), contains: "permission=denied")
        }
    }
}
