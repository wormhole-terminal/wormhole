import Foundation
import Testing

/// What a sidebar row draws besides its name, read from the views the window built (`marked`, inspect mode's probes, and
/// `colour-count` over the window's own picture): an agent's tile in its brand's colour, the ⌘1–9 keycaps while ⌘ is held,
/// the open notes' count and the "±N" of changed files.
extension EndToEnd {
    /// The elements `marked` lists, top to bottom: `marked <id> <x> <y> <w> <h>`.
    static func frames(_ said: String) -> [(id: String, rect: CGRect)] {
        said.split(separator: "\n").compactMap { line in
            let words = line.split(separator: " ").map(String.init)
            let numbers = words.dropFirst(2).compactMap(Double.init)
            guard words.count == 6, words[0] == "marked", numbers.count == 4 else { return nil }
            return (words[1], CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3]))
        }
    }

    /// `x y w h` of a frame, as `colour-count` takes a region.
    static func region(_ rect: CGRect) -> String { "\(Int(rect.minX)) \(Int(rect.minY)) \(Int(rect.width)) \(Int(rect.height))" }

    /// How many of the `id` elements stand inside each row of the list, top to bottom.
    static func perRow(_ said: String, _ id: String) -> [Int] {
        let all = frames(said)
        return all.filter { $0.id == "sidebar.row" }.map { row in all.filter { $0.id == id && row.rect.contains(CGPoint(x: $0.rect.midX, y: $0.rect.midY)) }.count }
    }

    @Test("an agent's tile is its brand's colour; a plain terminal's is not")
    @MainActor func agentTileWearsItsBrand() async throws {
        try await Instance.with(shell: .zsh) { app in
            // Claude's tile is its own orange, beside its logo (wormhole-term agents/mod.rs:109).
            let agent = try app.scratch.agentShell(named: "claude")
            await app.shell(agent.path)
            await app.until("rows") { $0.split(separator: "\n").contains { $0.contains("row 1 ") && $0.lowercased().contains("claude") } }
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["inspect on"])
            let tiles = Self.frames(await app.until("marked status.tile") { Self.frames($0).count == 2 })
            guard tiles.count == 2 else { return }
            // The list keeps the order the terminals opened in: the agent's row is the upper one.
            let orange = await app.reply("colour-count #d97757 \(Self.region(tiles[0].rect))")
            app.expect(Self.pixels(orange) > 20, "the agent's tile is Claude's orange:\n\(orange)")
            let plain = await app.reply("colour-count #d97757 \(Self.region(tiles[1].rect))")
            app.expect(Self.pixels(plain) == 0, "the shell's tile is not:\n\(plain)")
        }
    }

    @Test("⌘ held shows each numbered row's keycap; let go, they leave")
    @MainActor func hintChipsWhileCommandHeld() async throws {
        try await Instance.with { app in
            await app.send(["key cmd-t", "key cmd-t", "set sidebar_cmd_hints=true"])
            await app.until("rows") { Listing($0).live.count == 3 }
            await app.until("look-keys") { $0.contains("cmd_hints=true") }
            await app.send(["inspect on"])
            await app.until("marked") { Self.perRow($0, "keycap").count == 3 }
            // ⌘ alone, held: the hints come after a moment (sidebar/hints.rs:14), a keycap on each numbered row (row/face.rs:178).
            await app.send(["hold-cmd"])
            let held = await app.until("marked") { Self.perRow($0, "keycap") == [1, 1, 1] }
            app.expect(Self.perRow(held, "keycap") == [1, 1, 1], "one keycap a row while ⌘ is held:\n\(held)")
            await app.send(["release-cmd"])
            await app.until("marked") { Self.perRow($0, "keycap") == [0, 0, 0] }
        }
    }

    @Test("a row counts its open notes and tasks, and the repository's changed files on its topmost row, as the settings say")
    @MainActor func countBadgesOnRows() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository()
            try app.scratch.write(repo, "README.md", "one\ntwo\n")
            await app.send(["set sidebar_changes_scope=\"repository\"", "set sidebar_changes=true", "set sidebar_notes=true"])
            await app.terminal(in: repo)
            await app.terminal(in: repo)
            await app.send(["inspect on"])

            // The ±N of a repository is said once, on the topmost of its rows (row/face.rs:78, :145).
            let changes = await app.until("marked") { Self.perRow($0, "row.changes") == [0, 1, 0] }
            app.expect(Self.perRow(changes, "row.changes") == [0, 1, 0], "the repository's count on its upper row only:\n\(changes)")

            // Open steps of the selected terminal (the lowest row): its notes count (row/face.rs:132), gone with the setting.
            await app.send(["task-seed 2 0"])
            await app.until("marked") { Self.perRow($0, "row.notes") == [0, 0, 1] }
            await app.send(["set sidebar_notes=false"])
            await app.until("marked") { Self.perRow($0, "row.notes") == [0, 0, 0] }
            await app.send(["set sidebar_changes=false"])
            await app.until("marked") { Self.perRow($0, "row.changes") == [0, 0, 0] }
        }
    }
}
