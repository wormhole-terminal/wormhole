import Foundation
import Testing

/// The Session Hosts window as VoiceOver gets it (`a11y Session Hosts`): the heading, and each host's card as a button named by
/// its standing, its id, its window and its program, with the card's own buttons after it: Copy details and End while the host
/// runs, Forget in place of End for one that is gone.
extension EndToEnd {
    /// The dump's lines, trimmed, of the window that is open.
    @MainActor private static func tree(_ app: Instance) async -> [String] {
        _ = await app.reply("a11y Session Hosts")
        return (await app.reply("a11y Session Hosts")).split(separator: "\n").map { $0.trimmingCharacters(in: .whitespaces) }
    }

    /// The names of the buttons of the lines `from` until the next card.
    private static func buttons(after card: String, in lines: [String]) -> [String] {
        guard let at = lines.firstIndex(where: { $0.hasPrefix("Button \"\(card)") }) else { return [] }
        return lines[(at + 1)...].prefix { !$0.hasPrefix("Button \"Attached") && !$0.hasPrefix("Button \"Detached") && !$0.hasPrefix("Button \"Stale") && !$0.hasPrefix("Button/") }
            .compactMap { $0.hasPrefix("Button \"") ? $0.split(separator: "\"").dropFirst().first.map(String.init) : nil }
    }

    @Test(
        "the Session Hosts window's heading and cards are named: standing, id, window and program, then Copy details and End, or Forget when the host is gone",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostsWindowIsSpoken() async throws {
        try await Instance.with(hosts: true) { app in
            await app.send(["key cmd-t"])
            let hosts = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 })
            guard hosts.count == 2 else { return }
            await app.send(["hosts"])
            await app.until("hosts-window") { Readout($0).all("card").count == 2 }
            var lines = await Self.tree(app)
            app.expect(lines.contains { $0.hasPrefix("Heading \"Session Hosts\"") }, "the window has its heading:\n\(lines.joined(separator: "\n"))")
            for host in hosts {
                app.expect(lines.contains { $0.hasPrefix("Button \"Attached: \(host), Window 1, ") }, "\(host)'s card is named by what it is:\n\(lines.joined(separator: "\n"))")
                app.expect(Self.buttons(after: "Attached: \(host)", in: lines) == ["Copy details", "End"], "\(host)'s card has Copy details and End:\n\(Self.buttons(after: "Attached: \(host)", in: lines))")
            }

            // Let go of: the name follows the standing.
            app.expect(await app.reply("detach"), contains: "detached 2")
            await app.until("hosts-window") { said in hosts.allSatisfy { Readout(said).line("card", $0)?.has("attached") == false } }
            lines = await Self.tree(app)
            app.expect(hosts.allSatisfy { host in lines.contains { $0.hasPrefix("Button \"Detached: \(host), ") } }, "the cards say Detached:\n\(lines.joined(separator: "\n"))")

            // Gone behind the app's back: Forget instead of End.
            await app.send(["reattach"])
            await app.until("hosts-window") { said in hosts.allSatisfy { Readout(said).line("card", $0)?.has("attached") == true } }
            guard let pid = app.scratch.liveHosts()[hosts[1]] else { return }
            kill(pid, SIGKILL)
            await app.until("hosts-window") { Readout($0).line("card", hosts[1])?.has("stale") == true }
            lines = await Self.tree(app)
            let gone = Self.buttons(after: "Stale: \(hosts[1])", in: lines)
            app.expect(gone.contains("Forget") && !gone.contains("End"), "a stale card offers Forget, not End:\n\(lines.joined(separator: "\n"))")
        }
    }
}
