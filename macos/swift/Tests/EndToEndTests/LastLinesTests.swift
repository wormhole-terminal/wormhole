import Foundation
import Testing

/// The last lines the wave-8 lists waited for that a test instance can reach: the palette's ⌘⌫ on a running terminal, a block
/// out of range, ⌘C on a commit in History's list, and which profile's database holds a hosted terminal's row (read with the
/// system's `sqlite3` through `Spawn.run`, on the scratch folder only). TO-VERIFY.md lines 58, 193, 194, 682, 773, 1158.
extension EndToEnd {
    /// `select host_id from tabs` of a profile's database (`profiles/<id>/wormhole.db`, the default profile's at the data folder).
    @MainActor static func hostRows(_ app: Instance, profile: String?) throws -> [String] {
        let folder = profile.map { app.scratch.data.appendingPathComponent("profiles/\($0)") } ?? app.scratch.data
        let database = folder.appendingPathComponent("wormhole.db")
        guard FileManager.default.fileExists(atPath: database.path) else { return [] }
        let done = try Spawn.run("/usr/bin/sqlite3", ["-readonly", database.path, "select host_id from tabs where host_id is not null"], environment: app.scratch.cleanEnvironment)
        return done.output.split(separator: "\n").map(String.init)
    }

    @Test("⌘⌫ on a running terminal in the palette asks before closing it (58)")
    @MainActor func paletteCloseAsks() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.shell("sleep 300")
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            await app.send(["palette", "wait-idle 3000", "text sleep", "wait-idle 3000", "key cmd-backspace"])
            app.expect(await app.asked(), contains: "title=")
            await app.send(["question cancel", "key esc"])
            app.expect((await app.rows()).live.count == 2, "nothing closed without the answer")
        }
    }

    @Test("a block that is not there says so (682)")
    @MainActor func blockOutOfRange() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.markPrompts()
            await app.shell("echo only-block")
            await app.until("blocks") { $0.contains("echo only-block") }
            app.expect(await app.reply("block 9 select"), contains: "no block 9")
        }
    }

    @Test("⌘C on a commit in History's list copies its hash (773)")
    @MainActor func historyListCopy() async throws {
        try await Instance.with { app in
            let repo = try app.scratch.repository("list-copy")
            await app.terminal(in: repo)
            await app.send(["tab history"])
            await app.until("history-state") { $0.contains("commits=1") }
            let hash = try app.scratch.git(repo, "rev-parse", "HEAD").trimmingCharacters(in: .whitespacesAndNewlines)
            for _ in 0..<8 {
                if (await app.reply("keys")).hasPrefix("focus=panel-content") { break }
                await app.send(["key f6"])
            }
            await app.send(["key down", "key cmd-c"])
            await app.until("pasteboard") { $0.contains(hash) }
        }
    }

    @Test(
        "a hosted terminal opened in a window of another profile has its row in that profile's database, none in the other (193, 194, 1158)",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostRowsPerProfile() async throws {
        try await Instance.with(hosts: true) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            let before = try Self.hostRows(app, profile: nil)
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            await app.send(["profile-move \(work)"], window: 1)
            await app.until("sessions") { $0.contains("profile=\(work) ") }
            await app.send(["key cmd-t"], window: 1)
            let hosted = await app.until("sessions") { Instance.hostIDs(in: $0).count == 2 }
            guard let new = Instance.hostIDs(in: hosted).last else { return }
            await app.send(["save-windows"])
            await app.until("delay 200") { _ in (try? Self.hostRows(app, profile: work))?.contains(new) == true }
            app.expect(!(try Self.hostRows(app, profile: nil)).contains(new), "the profile the window left has no row for it; it had \(before)")
        }
    }

    @Test("a panel switch flipped in a window of another profile is written to that profile's config.json, never the other's (497)")
    @MainActor func panelSwitchInOtherProfile() async throws {
        try await Instance.with { app in
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            await app.send(["profile-open \(work)"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            let sessions = await app.reply("sessions")
            let window = sessions.split(separator: "\n").firstIndex { $0.contains("profile=\(work) ") }.map { $0 + 1 } ?? 2
            await app.send(["panel-option split on"], window: window)
            let file = app.scratch.data.appendingPathComponent("profiles/\(work)/config.json")
            let written = await app.watch(file.deletingLastPathComponent(), timeout: 15) {
                ((try? String(contentsOf: file, encoding: .utf8)) ?? "").contains("\"diff_split\"")
            }
            app.expect(written, "the Work profile's file has the switch: \(file.path)")
            app.expect(!(app.scratch.dataFile("config.json") ?? "").contains("\"diff_split\": true"), "the other profile's file does not")
        }
    }
}
