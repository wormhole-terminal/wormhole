import Foundation
import Testing

/// Persistent terminals turned off and on while the app runs: the switch decides where the next terminal's program runs, and
/// leaves the terminals that are open where they are. The hosts are read from `sessions` and from the hosts folder's own files.
extension EndToEnd {
    @Test(
        "Persistent terminals switched off while running: the next terminal runs in the app and the hosted one stays hosted; on again, the next is hosted",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func persistentTerminalsSwitchedWhileRunning() async throws {
        try await Instance.with(hosts: true) { app in
            let first = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 })
            let processes = app.scratch.liveHosts()
            await app.send(["type in-the-first-host\\n"])
            await app.screen(showing: "in-the-first-host")

            // Off: a new terminal starts no host and runs all the same.
            await app.flags(["persistent_terminals": false])
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["type runs-in-the-app\\n"])
            await app.screen(showing: "runs-in-the-app")
            app.expect(Instance.hostIDs(in: await app.reply("sessions")) == first, "only the first terminal is hosted")
            app.expect(app.scratch.liveHosts() == processes, "no host was started, and the first one runs on: \(app.scratch.liveHosts())")

            // On again: the next terminal is in a host of its own; the one opened meanwhile stays in the app.
            await app.flags(["persistent_terminals": true])
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 3 }
            let both = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 })
            app.expect(both.first == first.first && Set(app.scratch.liveHosts().keys) == Set(both), "two hosts, the first among them: \(both) \(app.scratch.liveHosts())")
            guard let earliest = (await app.rows()).live.first else { return }
            await app.send(["select \(earliest)", "type still-there\\n"])
            let screen = await app.screen(showing: "still-there")
            app.expect(screen, contains: "in-the-first-host")
        }
    }
}
