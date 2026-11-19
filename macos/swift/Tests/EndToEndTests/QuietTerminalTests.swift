import Foundation
import Testing

/// Things a terminal must stay quiet about: ⌘↑ and ⌘↓ where no shell integration marks the prompts, and `Wormhole notify`
/// while notifications for it are off. Read from the screen (what reached the program) and the instance's log of banners.
/// (Focus reports, DECSET 1004, are not here: a view has the focus only in a key window, which a test instance never has.)
extension EndToEnd {
    @Test("without prompt marks ⌘↑ and ⌘↓ type nothing into the program; with them they move through the prompts")
    @MainActor func promptKeysTypeNothingWithoutMarks() async throws {
        try await Instance.with { app in
            // `cat` echoes whatever reaches it: nothing may.
            await app.send(["type before-the-keys\\n"])
            let before = await app.screen { $0.components(separatedBy: "before-the-keys").count == 3 }
            let said = await app.send(["key cmd-up", "key cmd-down", "key cmd-up", "wait-idle 1500", "screen"])
            app.expect(said[4] == before, "nothing was typed:\n\(said[4])\n--\n\(before)")
            app.expect(await app.reply("scrollback"), contains: "offset=0")
        }
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.shell("echo one")
            await app.shell("/usr/bin/seq 1 120")
            await app.until("blocks") { !Instance.blocks($0, running: "/usr/bin/seq 1 120").isEmpty }
            app.expect(await app.reply("scrollback"), contains: "offset=0")
            // ⌘↑: the view goes back to the prompt before, which is in the history now.
            await app.send(["key cmd-up", "key cmd-up", "frames 2"])
            await app.until("scrollback") { (Instance.value("offset", in: $0).flatMap { Int($0) } ?? 0) > 0 }
        }
    }

    @Test("Wormhole notify makes no banner while its notifications are off, and one once they are on")
    @MainActor func notifyRespectsItsSwitch() async throws {
        try await Instance.with(shell: .zsh, config: Self.access("open")) { app in
            guard let terminal = (await app.rows()).live.first else { return app.fail("a terminal") }
            await app.send(["set notify_needs_you=false"])
            await app.until("get notify_needs_you") { $0.contains("false") }
            let quiet = await app.reply("control --from \(terminal) notify --title Quiet-Title quiet-body")
            app.expect(!quiet.hasPrefix("error"), "the request is taken: \(quiet)")
            await app.send(["delay 1500"])
            app.expect(!(await app.reply("notices")).contains("Quiet-Title"), "no banner while the switch is off")

            await app.send(["set notify_needs_you=true"])
            await app.until("get notify_needs_you") { $0.contains("true") }
            // Asked again until the banner comes: the rules read the setting a moment after the file changed.
            var notices = ""
            let start = Date()
            repeat {
                _ = await app.reply("control --from \(terminal) notify --title Loud-Title loud-body")
                notices = (await app.send(["delay 400", "notices"]))[1]
            } while !notices.contains("Loud-Title: loud-body") && Date().timeIntervalSince(start) < 15
            app.expect(notices, contains: "Loud-Title: loud-body")
            app.expect(!notices.contains("Quiet-Title"), "the refused one never comes")
        }
    }
}
