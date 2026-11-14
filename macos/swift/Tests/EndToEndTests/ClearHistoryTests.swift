import Foundation
import Testing

/// ⌘K erases the history and leaves the screen (`Pty::clear_history`: "Erase the history (the screen stays)"), then sends the
/// form feed a shell redraws its prompt for. The scenarios that press ⌘K elsewhere see only the echoed `^L`; these read the
/// history itself: its length from the frame the view was given (`scrollback`), and what the wheel finds above the screen.
extension EndToEnd {
    /// `history=` of a `scrollback` reply.
    private static func history(_ reply: String) -> Int? { Instance.value("history", in: reply).flatMap { Int($0) } }

    /// The rows drawn with the wheel turned to the top of the history, then the view back at the live screen.
    @MainActor private static func topOfHistory(_ app: Instance) async -> String {
        let said = await app.send(["scroll 700 400 -9000", "frames 2", "delay 150", "screen", "scroll 700 400 9000", "frames 2"])
        return said[3]
    }

    @Test("⌘K erases the history and leaves the screen: the frame's history is 0 lines and the wheel finds nothing above")
    @MainActor func clearBufferErasesTheHistory() async throws {
        try await Instance.with { app in
            // A hundred lines through `cat` (each is echoed and written back): more than two screens of history.
            await app.send(["type " + (1...100).map { "early-\($0)\\n" }.joined()])
            await app.screen { $0.contains("early-100") }
            let filled = await app.until("scrollback") { (Self.history($0) ?? 0) >= 100 }
            let rows = Instance.value("rows", in: filled).flatMap { Int($0) } ?? 0
            app.expect((await Self.topOfHistory(app)).split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == "early-1" }, "the wheel reaches the first line")
            let before = await app.screen { $0.contains("early-100") }

            await app.send(["key cmd-k"])
            let cleared = await app.until("scrollback") { Self.history($0) == 0 }
            app.expect(Self.history(cleared) == 0, "no line of history is left: \(cleared)")
            // The screen stays: every line that was on it is still there (the form feed's echo is added, nothing is taken).
            let after = await app.screen { $0.contains("^L") }
            let kept = before.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
            let now = Set(after.split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) })
            app.expect(kept.count >= rows - 2 && kept.allSatisfy { now.contains($0) || now.contains($0 + "^L") }, "the screen kept its lines:\n\(before)\n--\n\(after)")
            // And the wheel has nowhere to go: the top of the history is the screen itself.
            let top = await Self.topOfHistory(app)
            app.expect(!top.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == "early-1" }, "the early lines are gone:\n\(top)")
            app.expect(top.contains("early-100"), "the view cannot leave the live screen:\n\(top)")

            // New output is history again.
            await app.send(["type " + (1...60).map { "late-\($0)\\n" }.joined()])
            await app.until("scrollback") { (Self.history($0) ?? 0) >= 60 }
        }
    }

    @Test(
        "⌘K in a hosted terminal erases the host's history too: attached again, the lines do not come back",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func clearBufferReachesTheHost() async throws {
        try await Instance.with(hosts: true) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.send(["type " + (1...100).map { "early-\($0)\\n" }.joined()])
            await app.screen { $0.contains("early-100") }
            await app.until("scrollback") { (Self.history($0) ?? 0) >= 100 }

            await app.send(["key cmd-k"])
            await app.until("scrollback") { Self.history($0) == 0 }
            // A current host forgets its history when told: no warning that it kept it.
            app.expect(!(await app.reply("notice chip")).contains("history"), "no notice that the host kept the history")

            // Let go and attached again, the screen is repainted from the host: what it kept is what comes back.
            app.expect(await app.reply("detach"), contains: "detached 1")
            await app.until("sessions") { $0.contains("detached(hostID:") }
            await app.send(["reattach"])
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.send(["type after-attach\\n"])
            await app.screen { $0.contains("after-attach") }
            let back = await app.reply("scrollback")
            app.expect((Self.history(back) ?? 999) < 10, "the host kept no history to give back: \(back)")
            let top = await Self.topOfHistory(app)
            app.expect(!top.split(separator: "\n").contains { $0.trimmingCharacters(in: .whitespaces) == "early-1" }, "the early lines did not come back:\n\(top)")
        }
    }
}
