import Foundation
import Testing

/// The Session Hosts window as its own model holds it (`hosts-window`: what the cards are drawn from), beside the hosts
/// folder's files: the cards' facts are the real hosts', the window follows a terminal being let go of and attached again
/// without being asked, and a stale host's Forget removes what it left behind.
extension EndToEnd {
    @Test(
        "the Session Hosts window: each card's process is the host's own, it follows detach and reattach, a killed host is stale and Forget removes its files",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostsWindowFollowsAndForgets() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let hosts = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 })
            guard hosts.count == 2 else { return }
            await app.send(["hosts"])
            let first = Readout(await app.until("hosts-window") { Readout($0).all("card").count == 2 })
            let processes = app.scratch.liveHosts()
            for host in hosts {
                guard let card = first.line("card", host) else { app.fail("a card for \(host):\n\(first.text)"); continue }
                app.expect(card.has("attached") && card.has("can-end") && !card.has("can-forget"), "\(host) is held by a terminal: \(card.text)")
                // The card's facts are that host's: its process id is the one its own file names, its link has been used.
                let facts = first.under("card", host)
                let pid = facts.first { $0.words.first == "hostPid" }?.label ?? ""
                app.expect(Int32(pid.filter(\.isNumber)) == processes[host], "the card names the host's process (\(String(describing: processes[host]))): \(pid)")
                app.expect(facts.contains { $0.words.first == "link" && !$0.label.isEmpty }, "the card says how its link stands:\n\(first.text)")
                app.expect(facts.contains { $0.words.first == "socket" && $0.label.contains(host) }, "the card names the host's socket:\n\(first.text)")
            }

            // Let go of: the window says so by itself (its watch of the hosts folder and its slow look), and the hosts run on.
            app.expect(await app.reply("detach"), contains: "detached 2")
            let letGo = Readout(await app.until("hosts-window") { said in hosts.allSatisfy { Readout(said).line("card", $0)?.has("attached") == false } })
            for host in hosts { app.expect(letGo.line("card", host)?.has("can-end") == true, "\(host) still runs:\n\(letGo.text)") }
            await app.send(["reattach"])
            await app.until("hosts-window") { said in hosts.allSatisfy { Readout(said).line("card", $0)?.has("attached") == true } }

            // A host killed behind the app's back leaves its files: its card is stale and offers Forget, the other is as it was.
            guard let pid = processes[hosts[1]] else { return }
            kill(pid, SIGKILL)
            let stale = Readout(await app.until("hosts-window") { Readout($0).line("card", hosts[1])?.has("stale") == true })
            app.expect(stale.line("card", hosts[1])?.has("can-forget") == true && stale.line("card", hosts[1])?.has("can-end") == false, "the dead host can be forgotten, not ended:\n\(stale.text)")
            app.expect(stale.line("card", hosts[0])?.has("attached") == true, "the other host is untouched:\n\(stale.text)")
            app.expect(await app.reply("hosts-forget \(hosts[0])"), contains: "error")
            let folder = app.scratch.hostsFolder
            let left = { ((try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []).filter { $0.hasPrefix(hosts[1]) } }
            app.expect(!left().isEmpty, "the dead host left files behind")

            app.expect(await app.reply("hosts-forget \(hosts[1])"), contains: "hosts-forget \(hosts[1])")
            await app.until("hosts-window") { Readout($0).line("card", hosts[1]) == nil && Readout($0).all("card").count == 1 }
            app.expect(await app.watch(folder, timeout: 10) { ((try? FileManager.default.contentsOfDirectory(atPath: folder.path)) ?? []).allSatisfy { !$0.hasPrefix(hosts[1]) } }, "its files are gone: \(left())")
            app.expect(app.scratch.liveHosts()[hosts[0]] == processes[hosts[0]], "the live host runs on")
        }
    }
}
