import Foundation
import Testing

/// More of persistent terminals with the built session host (`Instance.with(hosts: true)`): letting go of the hosts and
/// attaching again, a host ended while let go, a hosted terminal in a folded group, the Session Hosts window's cards, and a
/// finished program's state across a quit that keeps the hosts. TO-VERIFY.md lines 105, 361, 637, 638, 729.
extension EndToEnd {
    @Test(
        "detach keeps the screens and the hosts, typing goes nowhere; reattach brings both back and typing arrives (637)",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func detachAndReattach() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let two = await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            let hosts = Instance.hostIDs(in: two)
            await app.send(["type before-detach\\n"])
            await app.screen(showing: "before-detach")
            app.expect(await app.reply("detach"), contains: "detached 2")
            await app.until("sessions") { $0.components(separatedBy: "detached(hostID:").count - 1 == 2 }
            app.expect(Set(app.scratch.liveHosts().keys) == Set(hosts), "the hosts still run")
            app.expect(await app.reply("screen"), contains: "before-detach")
            await app.send(["type lost-while-detached\\n"])
            let back = await app.reply("reattach")
            for host in hosts { app.expect(back, contains: host) }
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            await app.send(["type after-reattach\\n"])
            let screen = await app.screen(showing: "after-reattach")
            app.expect(!screen.contains("lost-while-detached\nlost-while-detached"), "what was typed while let go did not arrive:\n\(screen)")
        }
    }

    @Test(
        "reattach after the host of a let-go terminal was ended: a new terminal in its place (638)",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func reattachAfterHostEnded() async throws {
        try await Instance.with(hosts: true) { app in
            let one = await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            guard let host = Instance.hostIDs(in: one).first else { return }
            await app.send(["detach", "host-kill \(host)"])
            let folder = app.scratch.hostsFolder
            app.expect(await app.watch(folder, timeout: 15) { Scratch.liveHosts(in: folder)[host] == nil }, "the host ended")
            await app.send(["reattach"])
            let after = await app.until("sessions") { !Instance.hostIDs(in: $0).isEmpty && !Instance.hostIDs(in: $0).contains(host) }
            app.expect(Instance.hostIDs(in: after).count == 1, "one new terminal in a new host:\n\(after)")
        }
    }

    @Test(
        "a hosted terminal in a folded group is Attached in Session Hosts, and a card opens and folds (361, 729)",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostsCardsAndFoldedGroup() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let two = await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            guard let first = (await app.rows()).live.first, let host = Instance.hostIDs(in: two).first else { return }
            await app.send(["group \(first)"])
            guard let group = Listing(await app.until("rows") { !Listing($0).groups.isEmpty }).groups.first else { return }
            await app.send(["fold \(group.id)"])
            await app.until("rows") { Listing($0).groups.first?.folded == true }
            let listed = await app.reply("hosts-list")
            app.expect(listed.split(separator: "\n").contains { $0.hasPrefix(host) && $0.contains("(Attached)") }, "attached, folded or not:\n\(listed)")
            await app.send(["hosts"])
            app.expect(await app.until("hosts-toggle \(host)") { $0.contains("open=") }, contains: "open=true")
            app.expect(await app.reply("hosts-toggle \(host)"), contains: "open=false")
        }
    }

    @Test(
        "an agent that finished unseen in a hosted terminal is Done again after a quit that keeps its host (105)",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func finishedUnseenSurvivesQuit() async throws {
        try await Instance.with(shell: .agent("codex"), hosts: true) { app in
            await app.send(["key cmd-t"])
            let ids = Listing(await app.until("rows") { Listing($0).live.count == 2 }).live
            guard let watched = ids.first, let other = ids.last else { return }
            await app.send(["select \(other)"])
            try await app.report(agent: "codex", event: "UserPromptSubmit", terminal: watched, becomes: "working")
            try await app.report(agent: "codex", event: "Stop", terminal: watched, becomes: "finished")
            await app.send(["save-windows"])
            await app.quit(command: "quit-keep")
            try await app.start()
            await app.until("rows") { Listing($0).rows.contains { $0.activity == "finished" } }
        }
    }
}
