import Foundation
import Testing

/// A hook that arrives while the app's main thread is busy, as it is on a loaded Mac, and a look at the terminal's process that comes
/// before the main thread has read the row: the status the hook made must still reach the notification rules, which compare looks at
/// the rows. Before, the look (every 400 ms for a new terminal) handed the status back to idle first and the banner was never made;
/// the GPUI app decides at the hook, on the thread that looks at the process too. The main thread is held with `stall` (a test seam of
/// the control folder), the hook sent while it is.
extension EndToEnd {
    @Test("a hook that arrives while the main thread is busy still makes its banner, though a look at the process comes first")
    @MainActor func hookDuringAStallMakesItsBanner() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t"])
            let two = Listing(await app.until("rows") { Listing($0).live.count == 2 })
            guard let first = two.live.first, let other = two.live.last else { return app.fail("a second terminal") }
            await app.send(["select \(first)"])

            // The main thread is held for 1.5 s: the control file is taken, then the hook is sent, and the look at the new
            // terminal's process (every 400 ms) runs in between.
            let stalled = Task { @MainActor in await app.send(["stall 1500"], timeout: 60) }
            let taken = await app.waitFor(app.scratch.control.appendingPathComponent("cmd"), present: false, timeout: 10)
            app.expect(taken, "the app took the stall")
            let sent = try app.hook(agent: "codex", event: "PermissionRequest", terminal: other)
            app.expect(sent == 0, "the hook client was taken by the app (status \(sent))")
            _ = await stalled.value

            // The pull that follows saw the status: the banner is made, whatever the look makes of the row after it.
            let log = await app.until("notice-log", timeout: 10) { $0.contains("codex needs input") }
            app.expect(log, contains: "wormhole/tab/\(other)")
        }
    }
}
