import Foundation
import Testing

/// Several windows and the command line (`control [--from <terminal>] <words…>`, the request `Wormhole <words…>` sends, its
/// reply's JSON line): keys reaching the window they are for, a split's share, a worktree asked for by a script, windows of a
/// second profile and what they write, and a port a program listens on.
extension EndToEnd {
    @Test("keys sent to the first window act there while the second is in front; `new-terminal` and `list` from the command line")
    @MainActor func keysInTheirWindow() async throws {
        // The command line's verbs need the Automation flag and agent access, set before launch as a person's config has them.
        try await Instance.with(config: Self.access("open")) { app in
            guard let first = await app.live(1)?.first else { return }
            await app.send(["new-window"])
            await app.until("sessions") { $0.split(separator: "\n").count == 2 }
            await app.send(["key cmd-t"], window: 1)
            await app.live(2, window: 1)
            app.expect((await app.rows(window: 2)).live.count == 1, "the second window kept its one terminal")

            let listed = await app.reply("control --from \(first) list --json")
            app.expect(listed, contains: "front")
            let said = await app.reply("control --from \(first) new-terminal")
            app.expect(!said.hasPrefix("error"), "the request was taken: \(said)")
            // It opens in the window the core has in front, one of the two.
            await app.until("sessions") { text in
                text.split(separator: "\n").compactMap { Self.terminalCount(in: String($0)) }.reduce(0, +) == 4
            }
        }
    }

    /// The number of terminals a `sessions` line lists (`terminals=[1,2]`).
    private static func terminalCount(in line: String) -> Int? {
        guard let start = line.range(of: "terminals=["), let end = line[start.upperBound...].firstIndex(of: "]") else { return nil }
        return line[start.upperBound..<end].split(separator: ",").count
    }

    @Test("`Wormhole split --size 30` gives the new pane about 30 %; a split with no room is refused and changes nothing")
    @MainActor func splitFromCommandLine() async throws {
        // The command line's verbs need the Automation flag and agent access, set before launch as a person's config has them.
        try await Instance.with(config: Self.access("open")) { app in
            guard let first = await app.live(1)?.first else { return }
            let said = await app.reply("control --from \(first) split --terminal \(first) --size 30")
            app.expect(said, contains: "size")
            let panes = Panes(await app.until("panes") { Panes($0).ids.count == 2 })
            let widths = panes.sizes
            if let mine = widths[first]?.width, let new = widths.first(where: { $0.key != first })?.value.width, mine + new > 0 {
                let share = new / (mine + new)
                app.expect(share > 0.22 && share < 0.38, "the new pane has about 30 %: \(Int(share * 100)) %")
            } else {
                app.fail("both panes with their sizes:\n\(panes.ids)")
            }

            await app.send(["size 420x300"])
            let refused = await app.reply("control --from \(first) split --terminal \(first) --size 99")
            app.expect(refused, contains: "no_room")
            app.expect(Panes(await app.reply("panes")).ids.count == 2, "nothing was split")
        }
    }

    @Test("`Wormhole new-terminal --worktree` asks for the worktree in the window, and Return makes it")
    @MainActor func worktreeFromCommandLine() async throws {
        // The command line's verbs need the Automation flag and agent access, set before launch as a person's config has them.
        try await Instance.with(shell: .zsh, config: Self.access("open")) { app in
            let repo = try app.scratch.repository()
            await app.send(["new \(repo.path)"])
            guard let two = await app.live(2) else { return }
            let said = await app.reply("control --from \(two[1]) new-terminal --worktree fix-x --command \"echo hi\"")
            app.expect(!said.contains("unsupported"), "the worktree is asked for: \(said)")
            await app.until("windows") { $0.contains("sheet=true") }
            await app.send(["key return"])
            let three = Listing(await app.until("rows") { Listing($0).live.count == 3 })
            app.expect(three.rows.contains { $0.folder.contains("fix-x") }, "a terminal in the worktree:\n\(three.text)")
        }
    }

    @Test("a window of a second profile: ⌘N there opens another of that profile, and its View switches write that profile's file")
    @MainActor func profileWindows() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("profile-new Work"), contains: "profile-new Work")
            let profiles = await app.until("profiles") { $0.contains("\"Work\"") }
            guard let line = profiles.split(separator: "\n").first(where: { $0.contains("\"Work\"") }) else { return }
            let words = line.split(separator: " ").map(String.init)
            guard let id = words.first == "*" ? words.dropFirst().first : words.first else { return }

            // Making a profile enters it (the Default one's window goes out of sight, a Work one comes): `profile-open` adds
            // another window of it, and ⌘N in a Work window one more.
            let work: (String) -> Int = { $0.components(separatedBy: "profile=\(id) ").count - 1 }
            let entered = work(await app.until("sessions") { work($0) >= 1 })
            await app.send(["profile-open \(id)"])
            await app.until("sessions") { work($0) == entered + 1 }
            // Among the windows in sight (all Work's now), the second.
            await app.send(["key cmd-n"], window: 2)
            await app.until("sessions") { work($0) == entered + 2 }

            let file = app.scratch.data.appendingPathComponent("profiles/\(id)/config.json")
            await app.send(["action toggleStatusBar"], window: 2)
            let written = await app.watch(file.deletingLastPathComponent(), timeout: 15) {
                ((try? String(contentsOf: file, encoding: .utf8)) ?? "").contains("\"status_bar\"")
            }
            app.expect(written, "the Work profile's config.json has the switch: \(file.path)")
            app.expect(!(app.scratch.dataFile("config.json") ?? "").contains("\"status_bar\": true"), "nothing of it went to the Default profile's file")
        }
    }

    @Test("a port a program listens on is a chip, and choosing it is a launch of its address (recorded, not opened)")
    @MainActor func listeningPort() async throws {
        try await Instance.with(shell: .zsh) { app in
            let port = 38_000 + Int.random(in: 0..<900)
            await app.shell("/usr/bin/nc -l 127.0.0.1 \(port)")
            await app.until("ports", timeout: 40) { $0.contains("\(port)") }
            await app.send(["ports 1"])
            await app.until("launches") { $0.contains("\(port)") }
            await app.send(["type \u{03}"])
        }
    }
}
