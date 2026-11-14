import Foundation
import Testing

/// The status bar's items read with `status-items` (`Readout`'s shape): the button for the terminal waiting longest and its
/// tooltip, the counts, the selected terminal's state words and size, and what goes as the window narrows. TO-VERIFY.md lines
/// 196, 197, 198, 806, 1165.
extension EndToEnd {
    /// Three terminals of an agent, oldest first.
    @MainActor static func threeAgents(_ app: Instance) async -> [Int] {
        await app.send(["key cmd-t", "key cmd-t"])
        return Listing(await app.until("rows") { Listing($0).live.count == 3 }).live
    }

    @Test("three terminals need input: the button is the one waiting longest, its tooltip names the others; narrower, words go (196, 1165)")
    @MainActor func statusAttentionAndWidth() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            let ids = await Self.threeAgents(app)
            guard ids.count == 3 else { return app.fail("three terminals: \(ids)") }
            for id in ids { try await app.report(agent: "codex", event: "PermissionRequest", terminal: id, becomes: "needsInput") }
            await app.send(["status-bar on"])
            let wide = Readout(await app.until("status-items") { Readout($0).first("attention") != nil })
            app.expect(wide.first("status-items")?["detail"] == "words", "all words at 1600 points:\n\(wide.text)")
            let button = wide.first("attention")
            app.expect(button?.words.first == String(ids[0]), "the button is the terminal waiting longest (\(ids[0])):\n\(wide.text)")
            app.expect(button?["words"]?.hasPrefix("needs input") == true, "it says needs input:\n\(wide.text)")
            let tooltip = button?["tooltip"] ?? ""
            app.expect(tooltip.contains(", then ") && tooltip.contains("go to the first"), "the tooltip names who waits next: \(tooltip)")
            let waiting = wide.all("count").first { $0.words.first == "needsInput" }
            app.expect((waiting?.words.dropFirst().first.flatMap { Int($0) } ?? 0) >= 2, "the others waiting are counted:\n\(wide.text)")

            await app.send(["size 1000x900"])
            let narrow = Readout(await app.until("status-items") { Readout($0).first("status-items")?["detail"] != "words" })
            app.expect(narrow.first("status-items")?["detail"] == "numbers", "under 1100 points the words go:\n\(narrow.text)")
            app.expect(narrow.all("count").allSatisfy { Int($0["words"] ?? "") != nil }, "the counts keep only their numbers:\n\(narrow.text)")
            app.expect(narrow.first("size") != nil, "the size stays at 1000 points:\n\(narrow.text)")
        }
    }

    @Test("the bar's end says working, then done, after the selected terminal's state, and the size names the program (197, 198, 806)")
    @MainActor func statusStateWords() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            await app.send(["status-bar on"])
            try await app.report(agent: "codex", event: "UserPromptSubmit", terminal: 1, becomes: "working")
            let working = Readout(await app.until("status-items") { Readout($0).first("state") != nil })
            app.expect(working.first("state")?.label.hasPrefix("working") == true, "working:\n\(working.text)")
            let size = working.first("size")?.label ?? ""
            app.expect(size.contains(" · ") && size.contains("×"), "the program and its grid: \(size)")
            try await app.report(agent: "codex", event: "Stop", terminal: 1, becomes: "finished")
            let done = Readout(await app.until("status-items") { Readout($0).first("state")?.label.hasPrefix("done") == true })
            app.expect(done.first("attention") == nil, "nothing needs you:\n\(done.text)")
        }
    }
}
