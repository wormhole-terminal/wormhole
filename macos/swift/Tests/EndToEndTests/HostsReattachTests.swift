import Foundation
import Testing

/// Letting go of the session hosts and attaching again brings back the SAME hosts: `detachAndReattach` shows two hosts before
/// and two after; this reads which. The hosts' ids and processes come from the instance's hosts folder (each host's
/// `<id>.meta` names its process), the terminals' hosts from `sessions`, and each shell says its own process id on its screen.
extension EndToEnd {
    /// The number after `marker` on the screen (`echo marker$$` printed the shell's process id), nil when it is not there.
    private static func number(after marker: String, in screen: String) -> Int? {
        screen.split(separator: "\n").lazy.compactMap { line -> Int? in
            guard line.hasPrefix(marker) else { return nil }
            return Int(line.dropFirst(marker.count).trimmingCharacters(in: .whitespaces))
        }.first
    }

    @Test(
        "reattach attaches each terminal to the host it had: the same ids and processes, the screens' text kept, the same shells typing on",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func reattachIsTheSameHosts() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.send(["key cmd-t"])
            let before = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 })
            guard let terminals = await app.live(2), before.count == 2 else { return }
            // Each shell writes its own process id: what must still be there, and still be the shell that answers, afterwards.
            var shells: [Int: Int] = [:]
            for (index, terminal) in terminals.enumerated() {
                await app.send(["select \(terminal)"])
                await app.shell("echo first\(index)-$$")
                shells[terminal] = Self.number(after: "first\(index)-", in: await app.screen { Self.number(after: "first\(index)-", in: $0) != nil })
            }
            // The hosts as their own files name them: two processes, the two the terminals are in.
            let processes = app.scratch.liveHosts()
            app.expect(Set(processes.keys) == Set(before), "the hosts folder names the terminals' two hosts: \(processes) against \(before)")

            app.expect(await app.reply("detach"), contains: "detached 2")
            await app.until("sessions") { $0.components(separatedBy: "detached(hostID:").count - 1 == 2 }
            app.expect(app.scratch.liveHosts() == processes, "let go, the same host processes run on: \(app.scratch.liveHosts()) against \(processes)")

            await app.send(["reattach"])
            let after = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 })
            // The same hosts, each under the terminal it was under (the lists are in the terminals' order), and no host was
            // started in their place: the folder still names those two processes and no third.
            app.expect(after == before, "each terminal is in the host it had: \(after) against \(before)")
            app.expect(app.scratch.liveHosts() == processes, "the same host processes, and no new one: \(app.scratch.liveHosts()) against \(processes)")

            for (index, terminal) in terminals.enumerated() {
                await app.send(["select \(terminal)"])
                // What was on the screen before is there, and the shell that answers now is the one that wrote it.
                await app.shell("echo again\(index)-$$")
                let screen = await app.screen { Self.number(after: "again\(index)-", in: $0) != nil }
                app.expect(Self.number(after: "first\(index)-", in: screen) == shells[terminal], "terminal \(terminal) kept its text:\n\(screen)")
                app.expect(Self.number(after: "again\(index)-", in: screen) == shells[terminal], "terminal \(terminal) is the same shell (\(String(describing: shells[terminal]))):\n\(screen)")
            }
        }
    }
}
