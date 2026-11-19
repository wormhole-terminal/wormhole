import Foundation
import Testing

/// A session host of an older build, made from the repository's own history (`Instance.oldHostProgram`: the commit before hosts
/// learned to forget a terminal's history, protocol minor 1; it is the same major, so the app attaches to it). ⌘K erases the
/// history here, and the person is told the host is too old to forget it: it gives the lines back when the terminal is attached
/// again (`pointer.rs` `clear_history`, GPUI's `history_kept_by_host`).
extension EndToEnd {
    private static func history(_ reply: String) -> Int? { Instance.value("history", in: reply).flatMap { Int($0) } }

    @Test(
        "⌘K in a terminal of a host too old to forget says the history is kept there, and it comes back when attached again",
        .enabled(if: Instance.oldHostProgram() != nil, "no older session host is built: see Instance.oldHostProgram"))
    @MainActor func clearBufferKeptByAnOldHost() async throws {
        guard let old = Instance.oldHostProgram() else { return }
        try await Instance.with(hosts: true, hostFrom: old) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.send(["type " + (1...100).map { "early-\($0)\\n" }.joined()])
            await app.screen { $0.contains("early-100") }
            await app.until("scrollback") { (Self.history($0) ?? 0) >= 100 }

            await app.send(["key cmd-k"])
            await app.until("scrollback") { Self.history($0) == 0 }
            // Erased here, and the person is told it is not erased there.
            let said = await app.until("notice chip") { $0.contains("too old to forget") }
            app.expect(said, contains: "Cleared here only")

            // Let go and attached again: the host's screen is repainted with what it kept.
            app.expect(await app.reply("detach"), contains: "detached 1")
            await app.until("sessions") { $0.contains("detached(hostID:") }
            await app.send(["reattach"])
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            let back = await app.until("scrollback") { (Self.history($0) ?? 0) >= 50 }
            app.expect((Self.history(back) ?? 0) >= 50, "the old host gave its history back: \(back)")
        }
    }
}
