import Foundation
import Testing

/// Links a Probe's rules make of a program's output: a file target, whose readout names the Probe and whose click looks the
/// file up in the terminal's folder, and a URL target built from the match. Read from what the terminal's view says is under
/// the pointer with ⌘ held (`link`), what a click asked to open (`link-click`, `launches`: nothing is opened), with the
/// Probe approved through the owner's question (`WORMHOLE_AUTH=allow`, never a prompt).
extension EndToEnd {
    private static let probe = """
        [probe]
        id = "links"
        name = "Links"
        description = "File and issue links"
        author = "a test"
        version = "0.1.0"
        wormhole = ">=1.3"
        api = 1

        [permissions]
        open = ["issues.example.com"]

        [[contributes.rules]]
        id = "toml"
        pattern = '[A-Za-z]+\\.toml:[0-9]+'
        hover = "Open {0}"
        action = { type = "open-path" }

        [[contributes.rules]]
        id = "issue"
        pattern = 'ISSUE-([0-9]+)'
        hover = "Issue {1}"
        action = { type = "open-url", url = "https://issues.example.com/browse/{1}" }
        """

    @Test("a Probe's rules link a file and an issue number in the output: the readout names the Probe, a click looks the file up and builds the address; before approval and once turned off there is no link")
    @MainActor func probeRuleLinks() async throws {
        try await Instance.with(shell: .zsh, auth: "allow", config: #"{"feature_flags": {"unlocked": true, "probes": true}}"#) { app in
            try app.scratch.write(app.scratch.data, "probes/links/probe.toml", Self.probe + "\n")
            try app.scratch.write(app.scratch.home, "scratch.toml", "a = 1\nb = 2\nc = 3\n")
            await app.shell("echo 'see scratch.toml:3 and ISSUE-42'")
            /// The row of the output line that starts with `word`, and the column of the issue number in it.
            @MainActor func place(_ word: String) async -> (row: Int, issue: Int)? {
                let screen = await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("\(word) scratch.toml:3") } }
                let lines = screen.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
                guard let row = lines.firstIndex(where: { $0.hasPrefix("\(word) scratch.toml:3") }), let issue = lines[row].range(of: "ISSUE-42") else { return nil }
                return (row, lines[row].distance(from: lines[row].startIndex, to: issue.lowerBound) + 2)
            }
            let file = 6
            guard let first = await place("see") else { return app.fail("the output line") }
            // The Probe is in its folder and nobody approved it: its rules mark nothing.
            app.expect(await app.reply("link \(first.row) \(file)"), contains: "no link")

            // Approved (the owner's question is answered by the instance, never asked): in what the program prints from then
            // on the file's name is a link that is not an address, with the Probe's name on its readout.
            let approved = await app.later("probe approve links", answer: "probe approve links")
            app.expect(approved.contains("links:") && !approved.contains("invalid"), "the Probe is valid and approved: \(approved)")
            var found: (row: Int, issue: Int)?
            var link = ""
            let start = Date()
            repeat {
                // Printed again until its rules are in force (the Probe is loaded a moment after the approval).
                await app.shell("clear; echo 'now scratch.toml:3 and ISSUE-42'")
                found = await place("now")
                link = found == nil ? "" : (await app.send(["frames 2", "link \(found?.row ?? 0) \(file)"]))[1]
            } while !link.hasPrefix("link scratch.toml:3") && Date().timeIntervalSince(start) < 20
            guard let (row, number) = found else { return app.fail("the output line after the approval") }
            app.expect(link.hasPrefix("link scratch.toml:3"), "the file's name is a link: \(link)\n\(approved)")
            app.expect(link, contains: "file=true")
            app.expect(link.contains("readout=Open scratch.toml:3") && link.contains("· Links"), "the readout says what it opens and names the Probe: \(link)")
            // A click looks the file up in the terminal's folder and asks for it there (recorded, not opened).
            await app.send(["link-click \(row) \(file)"])
            let opened = await app.until("launches") { $0.contains("scratch.toml") }
            app.expect(opened.contains(app.scratch.home.lastPathComponent + "/scratch.toml"), "the file in the terminal's folder is the one asked for:\n\(opened)")

            // The issue number is a link to the address its rule builds from the match.
            let address = await app.reply("link-click \(row) \(number)")
            app.expect(address.hasPrefix("link https://issues.example.com/browse/42"), "the address is built from the match: \(address)")
            app.expect(address.contains("· Links") && address.contains("opens=url(\"https://issues.example.com/browse/42\")"), "the Probe is named and the click asks for that address: \(address)")

            // Turned off: the output is plain text again.
            await app.later("probe off links", answer: "probe off links")
            await app.until("link \(row) \(file)") { $0.contains("no link") }
        }
    }
}
