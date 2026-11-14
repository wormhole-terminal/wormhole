import Foundation
import Testing

/// A terminal's text kept between launches (`restore_terminal_text`): after a quit, after a crash once a minute's save has run,
/// erased by ⌘K at once and never kept for a terminal that opted out; and what a profile gives the terminals it starts.
extension EndToEnd {
    @Test("a terminal's text comes back after a quit above a restored line, not for a terminal that opted out, nor with the setting off")
    @MainActor func savedText() async throws {
        try await Instance.with { app in
            await app.emit("kept-after-quit")
            await app.screen { $0.contains("kept-after-quit") }
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            let (first, second) = (two[0], two[1])
            await app.emit("private-words")
            await app.screen { $0.contains("private-words") }
            let menu = await app.reply("row-menu \(second)")
            app.expect(menu, contains: "Don't keep this terminal's text")
            app.expect(await app.reply("menu-pick Don't keep this terminal's text"), contains: "picked")

            try await app.restart()
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["select \(first)"])
            let restored = await app.screen { $0.contains("kept-after-quit") }
            app.expect(restored, contains: "restored")
            await app.send(["select \(second)", "wait-idle 3000"])
            app.expect(!(await app.reply("screen")).contains("private-words"), "the opted-out terminal's text was not kept")

            // With the setting off nothing is kept, and what was kept is erased at the next launch.
            await app.send(["set restore_terminal_text=false"])
            _ = await app.waitForConfig { $0.range(of: "\"restore_terminal_text\"\\s*:\\s*false", options: .regularExpression) != nil }
            try await app.restart()
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["select \(first)", "wait-idle 3000"])
            app.expect(!(await app.reply("screen")).contains("kept-after-quit"), "nothing comes back with restore_terminal_text off")
        }
    }

    @Test("after a minute's save a crash keeps the text, and ⌘K erases the saved copy at once")
    @MainActor func textAfterCrash() async throws {
        try await Instance.with { app in
            await app.emit("minute-marker")
            await app.screen { $0.contains("minute-marker") }
            await app.send(["save-windows"])
            // The text is saved once a minute (`RustWorkspaceBackend.startSaving`): past one, the app ends as a crash would.
            await app.send(["delay 65000"], timeout: 90)
            await app.end(by: SIGKILL)
            try await app.start()
            await app.screen { $0.contains("minute-marker") }

            await app.send(["key cmd-k"])
            await app.screen { !$0.contains("minute-marker") }
            await app.end(by: SIGKILL)
            try await app.start()
            await app.send(["wait-idle 3000"])
            app.expect(!(await app.reply("screen")).contains("minute-marker"), "the cleared lines did not come back after the crash")
        }
    }

    @Test("a profile's environment and start folder reach the terminals it starts, with WORMHOLE_PROFILE")
    @MainActor func profileEnvironment() async throws {
        try await Instance.with(shell: .zsh) { app in
            let start = app.scratch.home.appendingPathComponent("profile-start")
            try FileManager.default.createDirectory(at: start, withIntermediateDirectories: true)
            let said = await app.reply("profile env=WH_E2E_PROFILE=from-profile start=\(start.path)")
            app.expect(said, contains: "from-profile")
            app.expect(said, contains: "problem=none")
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("echo env=$WH_E2E_PROFILE id=$WORMHOLE_PROFILE; pwd")
            let shown = await app.screen { $0.contains("env=from-profile") }
            app.expect(shown, contains: "id=default")
            app.expect(shown, contains: "/profile-start")
        }
    }
}
