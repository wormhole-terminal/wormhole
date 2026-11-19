import Foundation
import Testing

/// An agent's hook that comes before the app has looked at the terminal's process, and one that comes after. The rule is core's
/// `StatusMachine` (`status/hooks.rs`, `status/front.rs`), written once from the GPUI app's `terminal_view/status.rs`: a hook is
/// applied when it arrives (the row says it, the banner is made at once, whether or not an agent was seen there); the next look at
/// the process hands the status back to the output heuristic only when it finds no agent and the shell holds the terminal ("forget
/// it only once the shell is back"), so an agent's own process keeps what its hooks said.
extension EndToEnd {
    /// Two terminals; the first is selected, so the second's banner is not for what the person looks at. The second's number.
    @MainActor private static func second(_ app: Instance) async -> Int? {
        await app.send(["key cmd-t"])
        let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
        guard let first = two.live.first, let other = two.live.last else { return nil }
        await app.send(["select \(first)"])
        return other
    }

    @Test("a hook for a terminal that runs an agent is applied at once, makes its banner and holds past the next looks at the process")
    @MainActor func hookOnAnAgentSticks() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            guard let other = await Self.second(app) else { return app.fail("a second terminal") }
            // At once: no wait for the app to have seen the agent first.
            let sent = try app.hook(agent: "codex", event: "PermissionRequest", terminal: other)
            app.expect(sent == 0, "the hook client was taken by the app (status \(sent))")
            await app.until("rows") { Listing($0).row(other)?.activity == "needsInput" }
            let log = await app.until("notice-log") { $0.contains("codex needs input") }
            app.expect(log, contains: "wormhole/tab/\(other)")
            // The process look comes every second or so while a terminal is new: it finds the agent and leaves the status alone.
            await app.send(["delay 4000"])
            app.expect(Listing(await app.reply("rows")).row(other)?.activity == "needsInput", "the agent's own process keeps what its hook said")
        }
    }

    @Test("a hook for a terminal whose process is no agent is applied at once and made a banner, and the next look hands the status back")
    @MainActor func hookOnAPlainTerminalIsUndoneByTheLook() async throws {
        try await Instance.with { app in
            guard let other = await Self.second(app) else { return app.fail("a second terminal") }
            let sent = try app.hook(agent: "codex", event: "PermissionRequest", terminal: other)
            app.expect(sent == 0, "the hook client was taken by the app (status \(sent))")
            // Not dropped, not shown late: the row and the banner are there at the first answer.
            let log = await app.until("notice-log") { $0.contains("codex needs input") }
            app.expect(log, contains: "wormhole/tab/\(other)")
            // `cat` is the shell and holds the terminal, and it is no agent: the look forgets what the hook said.
            let rows = await app.until("rows") { Listing($0).row(other)?.activity == "idle" }
            app.expect(Listing(rows).row(other)?.activity == "idle", "the look handed the status back:\n\(rows)")
        }
    }
}
