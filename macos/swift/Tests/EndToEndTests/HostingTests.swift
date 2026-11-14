import Foundation
import Testing

/// Persistent terminals in the built app with the built session host beside it (`Instance.with(hosts:)`). Each run ends with
/// Quit and End All, and `Instance.finish` then finds no host of its folder running.
extension EndToEnd {
    @Test(
        "a terminal in a session host outlives a quit that keeps it and comes back attached with its screen",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostedTerminalOutlivesTheApp() async throws {
        try await Instance.with(hosts: true) { app in
            let before = await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            guard let host = Instance.hostIDs(in: before).first else { return }
            await app.send(["type remember-me\\n"])
            await app.until("screen") { $0.contains("remember-me") }
            await app.send(["save-windows"])
            // `quit` keeps the hosts: the program runs on without the app.
            await app.quit(command: "quit")
            app.expect(app.scratch.liveHosts().keys.contains(host), "the host runs on after the app quit: \(app.scratch.liveHosts())")
            try await app.start()
            await app.until("sessions") { Instance.hostIDs(in: $0) == [host] }
            await app.until("screen") { $0.contains("remember-me") }
            app.expect(await app.reply("hosts-list"), contains: host)
        }
    }

    @Test(
        "Quit and End All ends every session host, one still starting included",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func endAllLeavesNoHost() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let two = await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            app.expect(Set(app.scratch.liveHosts().keys) == Set(Instance.hostIDs(in: two)), "each terminal in a host of its own:\n\(two)")
            // A third terminal and the quit at once: its host may still be starting when the hosts are ended.
            await app.send(["key cmd-t"])
            await app.quit(command: "quit-end-all")
            let left = await app.hostsLeft()
            app.expect(left.isEmpty, "no host runs after Quit and End All: \(left.keys.sorted())")
        }
    }

    @Test(
        "the Session Hosts list names a terminal's host, host-kill ends it, and a host that died shows its bar",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostsListKillAndDeath() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let two = await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            let hosts = Instance.hostIDs(in: two)
            guard hosts.count == 2 else { return }
            let listed = await app.reply("hosts-list")
            for host in hosts { app.expect(listed, contains: host) }

            // Ended as `Wormhole sessions end <id>` ends it: its process goes, its card with it.
            let killed = await app.reply("host-kill \(hosts[0])")
            app.expect(killed, contains: "host-kill \(hosts[0])")
            let folder = app.scratch.hostsFolder, first = hosts[0]
            app.expect(await app.watch(folder, timeout: 15) { Scratch.liveHosts(in: folder)[first] == nil }, "host-kill ended \(first)")

            // The other dies behind the app's back: its terminal's shell went with it, and the pane's bar says so.
            guard let rows = Listing(await app.reply("rows")).live.last, let pid = app.scratch.liveHosts()[hosts[1]] else {
                return app.fail("no live host \(hosts[1]) to end")
            }
            await app.send(["select \(rows)"])
            kill(pid, SIGKILL)
            await app.until("sessions") { $0.contains("lost(hostID: \"\(hosts[1])\")") }
            let bar = await app.until("pane-bar") { $0.contains("exited") }
            app.expect(bar, contains: "host-lost")
        }
    }
}

extension Instance {
    /// The ids of the hosts the terminals of `sessions`' lines run in (`hosting=[1:persistent(hostID: "abc…")]`), in order.
    static func hostIDs(in sessions: String) -> [String] {
        let mark = "persistent(hostID: \""
        var ids: [String] = []
        var rest = Substring(sessions)
        while let start = rest.range(of: mark) {
            rest = rest[start.upperBound...]
            ids.append(String(rest.prefix { $0 != "\"" }))
        }
        return ids
    }
}
