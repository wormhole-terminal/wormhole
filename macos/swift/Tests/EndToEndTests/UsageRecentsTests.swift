import Foundation
import Testing

/// The Usage tab read with `usage` (`Readout`'s shape) after reports made through `usage-status` (Claude Code's status line JSON,
/// as its command hands it over), and the recent lists with `recents [projects|folders]`. TO-VERIFY.md lines 61, 73, 252, 732,
/// 791, 961, 962, 1114.
extension EndToEnd {
    /// Claude Code's status line input with a five-hour limit `used` % that resets `seconds` from now.
    static func statusLine(used: Int, resetsIn seconds: Int) -> String {
        let at = Int(Date().timeIntervalSince1970) + seconds
        return #"{"rate_limits": {"five_hour": {"used_percentage": \#(used), "resets_at": \#(at)}}}"#
    }

    /// A profile made and its id (Wormhole goes into it).
    @MainActor static func newProfile(_ app: Instance, _ name: String) async -> String? {
        await app.send(["profile-new \(name)"])
        let profiles = await app.until("profiles") { $0.contains("\"\(name)\"") }
        guard let line = profiles.split(separator: "\n").first(where: { $0.contains("\"\(name)\"") }) else { return nil }
        let words = line.split(separator: " ").map(String.init)
        return words.first == "*" ? words.dropFirst().first : words.first
    }

    @Test("a limit that resets in under a minute counts down by the second while the tab is up; Updated says just now (791, 962)")
    @MainActor func usageCountsDown() async throws {
        try await Instance.with(shell: .agent("claude")) { app in
            await app.send(["tab usage"])
            app.expect(await app.reply("usage-status \(Self.statusLine(used: 42, resetsIn: 45))"), contains: "recorded=true")
            let first = Readout(await app.until("usage") { Readout($0).first("report") != nil })
            app.expect(first.first("report")?["updated"] == "Updated just now", "the report's age:\n\(first.text)")
            let words = first.first("limit")?["words"] ?? ""
            app.expect(words.hasPrefix("42%") && words.contains("s"), "seconds left: \(words)")
            await app.send(["delay 1500"])
            let later = Readout(await app.reply("usage")).first("limit")?["words"] ?? ""
            app.expect(later != words, "the words moved by themselves: \(words) then \(later)")
        }
    }

    @Test("two windows show the same report (252)")
    @MainActor func usageInTwoWindows() async throws {
        try await Instance.with(shell: .agent("claude")) { app in
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            await app.send(["usage-status \(Self.statusLine(used: 10, resetsIn: 3600))"])
            for window in [1, 2] {
                await app.until("usage", window: window) { Readout($0).first("report")?.words.first == "claude" }
            }
        }
    }

    @Test("a report goes to the profile Wormhole is in; a window of the other profile draws that profile's reports (961)")
    @MainActor func usagePerProfile() async throws {
        try await Instance.with(shell: .agent("claude")) { app in
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            await app.send(["profile-open default"])
            await app.until("sessions") { $0.split(separator: "\n").count >= 2 }
            await app.send(["usage-status \(Self.statusLine(used: 30, resetsIn: 3600))"])
            let sessions = await app.reply("sessions")
            // The window of the profile Wormhole is in (Work) has it; the Default window does not.
            let workWindow = sessions.split(separator: "\n").firstIndex { $0.contains("profile=\(work) ") }.map { $0 + 1 } ?? 1
            let otherWindow = workWindow == 1 ? 2 : 1
            await app.until("usage", window: workWindow) { Readout($0).first("report") != nil }
            app.expect(Readout(await app.reply("usage", window: otherWindow)).first("report") == nil, "the other profile's window has none:\n\(sessions)")
        }
    }

    @Test("recent folders: kept in the database across a restart, emptied once the flag is off, a second window's visits counted (61, 73, 732)")
    @MainActor func recentFolders() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.flags(["folder_picker": true])
            let first = app.scratch.home.appendingPathComponent("recent-one")
            let second = app.scratch.home.appendingPathComponent("recent-two")
            for folder in [first, second] { try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true) }
            await app.terminal(in: first)
            await app.until("recents folders") { $0.contains("recent-one") }
            try await app.restart()
            app.expect(await app.reply("recents folders"), contains: "recent-one")

            // A second window's terminal changes folder: a visit.
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            await app.send(["type cd \(second.path)\\n"], window: 2)
            await app.until("recents folders", window: 1) { $0.contains("recent-two") }

            // The flag off and a folder change: the list is emptied once.
            await app.flags(["folder_picker": false])
            await app.send(["type cd \(first.path)\\n"], window: 2)
            await app.flags(["folder_picker": true])
            await app.until("recents folders") { !$0.contains("recent-two") }
        }
    }

    @Test("Open Recent lists the projects of each window's own profile (1114)")
    @MainActor func recentProjectsPerProfile() async throws {
        try await Instance.with { app in
            await app.flags(["project_folders": true])
            let mine = try app.scratch.repository("recent-default")
            try app.scratch.project(mine, files: ["config.json": "{}"])
            await app.terminal(in: mine)
            await app.send(["tab project"])
            await app.until("recents projects") { $0.contains("recent-default") }
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            await app.send(["profile-open \(work)"])
            await app.until("sessions") { $0.contains("profile=\(work) ") }
            let theirs = try app.scratch.repository("recent-work")
            try app.scratch.project(theirs, files: ["config.json": "{}"])
            await app.send(["new \(theirs.path)", "tab project"], window: 2)
            await app.until("recents projects", window: 2) { $0.contains("recent-work") }
            app.expect(!(await app.reply("recents projects", window: 2)).contains("recent-default"), "the Work window lists Work's only")
            app.expect(!(await app.reply("recents projects", window: 1)).contains("recent-work"), "the Default window lists Default's only")
        }
    }
}
