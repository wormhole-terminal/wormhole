import Foundation
import Testing

/// Closing what still runs (data loss): ⌘W, Close group, Close Window, ⌘Q and `Wormhole close` ask first, name what runs and count
/// what ends, a folded group's terminals included; a terminal whose command ended closes at once. A test instance holds each
/// question for `question` (`PendingQuestions`); `quit-ask` says what ⌘Q would ask without quitting.
extension EndToEnd {
    /// Waits until the core has seen `program` run (what ⌘Q would ask names it), so a close is sure to ask.
    @MainActor private static func running(_ program: String, in app: Instance) async {
        await app.until("quit-ask", timeout: 25) { $0.contains(program) }
    }

    @Test("⌘W on a running command asks once, names it, Cancel keeps it and Close ends it; an ended command closes without asking")
    @MainActor func closeWhatRuns() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["key cmd-t"])
            guard let two = await app.live(2) else { return }
            let sleeper = two[1]
            await app.shell("/bin/sleep 300")
            await Self.running("sleep", in: app)

            await app.send(["key cmd-w"])
            let asked = await app.asked()
            app.expect(asked, contains: "sleep")
            // A second ⌘W while the question is up is dropped, not queued behind it.
            await app.send(["key cmd-w"])
            app.expect(await app.question(), contains: "waiting=1")
            await app.send(["question cancel"])
            app.expect(Listing(await app.reply("rows")).row(sleeper) != nil, "Cancel kept the terminal")
            await app.send(["key cmd-w"])
            await app.asked()
            await app.send(["question confirm"])
            await app.until("rows") { Listing($0).row(sleeper) == nil }

            // A command that ran and ended (a failure among them) leaves nothing to ask about.
            await app.send(["key cmd-t"])
            guard let again = await app.live(2) else { return }
            await app.shell("false")
            await app.send(["delay 500"])
            app.expect(await app.reply("quit-ask"), contains: "no question")
            await app.send(["key cmd-w"])
            await app.until("rows") { Listing($0).live.count == 1 }
            app.expect(await app.question(), contains: "no question is up")
            app.expect(Listing(await app.reply("rows")).row(again[1]) == nil, "the terminal closed at once")
        }
    }

    @Test("a running command in a folded group: ⌘W, Close group, Close Window, ⌘Q and `Wormhole close` all ask and name it")
    @MainActor func closeInFoldedGroup() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.shell("/bin/sleep 300")
            await app.send(["key cmd-t"])
            guard let three = await app.live(3) else { return }
            let (sleeper, other) = (three[1], three[2])
            await Self.running("sleep", in: app)
            await app.send(["group \(sleeper) \(other)"])
            let grouped = Listing(await app.until("rows") { Listing($0).groups.first?.count == 2 })
            guard let group = grouped.groups.first else { return }
            await app.send(["fold \(group.id)", "select \(sleeper)"])
            await app.until("rows") { Listing($0).groups.first?.folded == true }

            await app.send(["key cmd-w"])
            app.expect(await app.asked(), contains: "sleep")
            await app.send(["question cancel"])

            // Close group from the head's menu (⌃↩'s, kept unshown): the group's two terminals are counted.
            app.expect(await app.reply("group-menu \(group.id)"), contains: "Close group")
            await app.send(["menu-pick Close group"])
            let closing = Instance.plain(await app.asked())
            app.expect(closing, contains: "sleep")
            app.expect(closing, contains: "2 terminals")
            await app.send(["question cancel"])

            await app.send(["close-window"])
            app.expect(await app.asked(), contains: "sleep")
            await app.send(["question cancel"])

            // A search that lists no row hides nothing from the questions.
            await app.send(["search zz-no-such-terminal"])
            app.expect(await app.reply("quit-ask"), contains: "sleep")
            await app.send(["search"])

            // From the command line (here from inside the first terminal) closing always asks (`fromCommandLine`).
            let said = await app.reply("control --from \(three[0]) close --terminal \(sleeper)")
            app.expect(!said.hasPrefix("error"), "the request was taken: \(said)")
            app.expect(await app.asked(), contains: "sleep")
            await app.send(["question cancel"])
            app.expect(Listing(await app.reply("rows")).live.count == 3, "every Cancel kept the terminals")
        }
    }

    @Test("an agent idle at its prompt makes ⌘W, Close Window and ⌘Q ask; a working one is said to be working")
    @MainActor func closeAgent() async throws {
        try await Instance.with(shell: .agent("claude")) { app in
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await Self.running("claude", in: app)
            await app.send(["key cmd-w"])
            let asked = Instance.plain(await app.asked())
            app.expect(asked, contains: "Still running: claude")
            await app.send(["question cancel"])
            await app.send(["close-window"])
            app.expect(await app.asked(), contains: "claude")
            await app.send(["question cancel"])

            try await app.report(agent: "claude", event: "UserPromptSubmit", terminal: 1, becomes: "working")
            await app.send(["close-window"])
            app.expect(await app.asked(), contains: "An agent is still working.")
            await app.send(["question cancel"])
            app.expect(Listing(await app.reply("rows")).live.count == 2, "Cancel kept both terminals")
        }
    }
}
