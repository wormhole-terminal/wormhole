import AppKit
import Foundation
import Testing

/// A terminal whose shell ended, started again from its bar (Restart): it is the same terminal, back in its pane with the
/// keys, its row idle again, with its note, in the app or in a new host of its own, and it is still there with its note at
/// the next launch (TO-VERIFY line 1569). Read from the window's views (`bounds`), where the keys are (`keys`), the list,
/// the notes' store, `sessions` and the hosts folder.
extension EndToEnd {
    /// zsh told to exit, and the bar over the terminal once it says the shell ended.
    @MainActor private static func endShell(_ app: Instance, _ id: Int) async {
        await app.shell("exit")
        await app.until("exited-words") { $0 != "exited-words none" }
        await app.until("rows") { Listing($0).row(id)?.activity == "exited" }
    }

    /// Restart pressed, then the new shell once it runs: the bar is gone and the row is no longer an ended shell's.
    @MainActor private static func restart(_ app: Instance, _ id: Int) async {
        app.expect(await app.reply("pane-restart"), contains: "pane-restart \(id)")
        await app.until("exited-words") { $0 == "exited-words none" }
        await app.until("rows") { Listing($0).row(id)?.activity == "idle" }
    }

    /// The number after `word` at the start of a line of the screen (`echo word$$` printed the shell's process id).
    private static func process(_ word: String, in screen: String) -> String? {
        screen.split(separator: "\n").first { $0.hasPrefix(word) }.map { $0.dropFirst(word.count).trimmingCharacters(in: .whitespaces) }
    }

    @Test("Restart on an ended shell's bar: the terminal is back in its pane with the keys, another shell answers, its row is idle and its note is kept, over a relaunch too")
    @MainActor func restartedShellKeepsItsPaneKeysAndNote() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.flags(["notes_and_tasks": true])
            guard let id = await app.live(1)?.first, let frame = await app.terminalFrames().first else { return app.fail("a terminal on screen") }
            let middle = "\(Int(frame.midX)) \(Int(frame.midY))"
            app.expect(await app.reply("bounds \(middle) 1"), contains: "MetalTerminalView")
            // While the shell runs its bar offers no Restart.
            app.expect(await app.reply("pane-restart"), contains: "error")
            await app.send(["tab notes", "note-add survives the restart"])
            await app.until("notes") { $0.contains("survives the restart") }
            // The keys back in the terminal (the note's field had them): a click on it.
            await app.send(["click \(middle)"])
            await app.until("keys") { $0.contains("focus=terminal") }
            await app.shell("echo shell-one-$$")
            let first = Self.process("shell-one-", in: await app.screen { Self.process("shell-one-", in: $0) != nil })

            await Self.endShell(app, id)
            await Self.restart(app, id)
            // Its new view is the one under the pane's middle, and it has the keys the terminal had.
            await app.until("bounds \(middle) 1") { $0.contains("MetalTerminalView") }
            await app.until("keys") { $0.contains("focus=terminal") }
            // Typed as the keyboard types, with nothing forced: another shell (another process) answers, and it is drawn.
            await app.send(["text echo shell-two-$$", "key return"])
            let second = Self.process("shell-two-", in: await app.screen { Self.process("shell-two-", in: $0) != nil })
            app.expect(first != nil && second != nil && first != second, "another shell answers: \(String(describing: first)) then \(String(describing: second))")
            app.expect((await app.rows()).live == [id], "it is the same terminal")
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("survives the restart") }

            await app.send(["save-windows"])
            try await app.restart()
            await app.until("rows") { Listing($0).live == [id] }
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("survives the restart") }
        }
    }

    @Test(
        "Restart on a hosted terminal's ended shell: a new host of its own with the keys, the old one gone, the note kept; after a quit that keeps the hosts it is attached to that host with its note",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func restartedHostedShellKeepsItsNote() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.flags(["persistent_terminals": true, "notes_and_tasks": true])
            guard let old = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }).first, let id = await app.live(1)?.first,
                let frame = await app.terminalFrames().first
            else { return }
            await app.send(["tab notes", "note-add survives the restart"])
            await app.until("notes") { $0.contains("survives the restart") }
            // The keys back in the terminal (the note's field had them): a click on it.
            await app.send(["click \(Int(frame.midX)) \(Int(frame.midY))"])
            await app.until("keys") { $0.contains("focus=terminal") }

            await Self.endShell(app, id)
            await Self.restart(app, id)
            // A new host for the same terminal; the one whose shell ended is gone from the folder's live hosts.
            let now = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 && !Instance.hostIDs(in: $0).contains(old) })
            let folder = app.scratch.hostsFolder
            app.expect(await app.watch(folder, timeout: 15) { Set(Scratch.liveHosts(in: folder).keys) == Set(now) }, "only the new host runs: \(app.scratch.liveHosts()) for \(now)")
            app.expect((await app.rows()).live == [id], "it is the same terminal")
            await app.until("keys") { $0.contains("focus=terminal") }
            await app.send(["text echo in-the-new-host", "key return"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } }
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("survives the restart") }

            // Quit keeping the hosts and come back: the terminal is attached to that host again, with its note.
            await app.send(["save-windows"])
            await app.quit(command: "quit-keep")
            try await app.start()
            await app.until("rows") { Listing($0).live == [id] }
            app.expect(Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }) == now, "attached to the host it was restarted in")
            app.expect(Set(app.scratch.liveHosts().keys) == Set(now), "and no other host runs: \(app.scratch.liveHosts())")
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("survives the restart") }
        }
    }

    // The line the shell printed last survives a quit that keeps the hosts and a launch, the window coming back as it was. It was lost
    // with every hosted zsh, restarted or not: at a launch (and at any change of the window's size) the pane area animated its frames
    // from the old layout to the new one, so the terminal was resized at each step of the motion, from 2 columns up (30 sizes in half a
    // second), and zsh redrew its prompt over the rows it believed were its own at each. GPUI lays the panes out at the new size at once.
    //
    // The window is one a small screen can hold as it is: the runner's usable area is 1024 x 674, and a window saved wider is clamped to it
    // at the relaunch. So 1000 x 600, with the panel and the sidebar hidden, which leaves the terminal the whole width (over 110
    // columns, room for a 62-character host name's prompt and the command: 84). The relaunch does not resize it: a harness that
    // widened a window the screen had narrowed was the runner's own cause of the loss (`PromptWidenedTests`, which is about that).
    /// The columns of a `geometry` reply's grid.
    private static func columns(_ geometry: String) -> Int {
        Instance.value("grid", in: geometry)?.split(separator: "x").first.flatMap { Int($0) } ?? 0
    }

    @Test(
        "a restarted hosted shell's last line is still there after a quit that keeps the hosts",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func restartedHostedShellKeepsItsText() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.flags(["persistent_terminals": true])
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            guard let id = await app.live(1)?.first else { return }
            // A window the smallest screen of the runners can hold unchanged (1024 x 674 usable), the columns hidden so that the prompt
            // does not wrap.
            for column in ["panel", "sidebar"] where (await app.reply("state")).contains("\(column)=true") { await app.send([column]) }
            await app.send(["size 1000x600", "frames 2"])
            let before = await app.until("geometry") { $0.contains("content=(1000.0, 600.0)") && Self.columns($0) >= 90 }
            app.expect(Self.columns(before) >= 90, "the pane is wide enough that the prompt does not wrap: \(before)")
            await Self.endShell(app, id)
            await Self.restart(app, id)
            await app.until("keys") { $0.contains("focus=terminal") }
            await app.send(["text echo in-the-new-host", "key return"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } }
            await app.send(["save-windows"])
            await app.quit(command: "quit-keep")
            try await app.start(keepingTheWindow: true)
            await app.until("rows") { Listing($0).live == [id] }
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            // The window came back as it was: its size is what the screen allows it (said here, so a run says what the screen was).
            let back = await app.until("geometry") { $0.contains("content=") }
            app.expect(back.contains("content=(1000.0, 600.0)") && Self.columns(back) >= 90, "the window came back as it was (1000 x 600, 90 columns or more); the screens' usable areas are \(NSScreen.screens.map(\.visibleFrame)):\n\(back)")
            // On the screen, or just above it where the shell's redraw after the attach pushed it.
            var text = ""
            let start = Date()
            repeat {
                text = (await app.send(["scroll 700 400 -600", "frames 2", "delay 150", "screen", "scroll 700 400 600"]))[3]
            } while !text.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } && Date().timeIntervalSince(start) < 15
            app.expect(text.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") }, "the line from before the quit is still there (the window: \(back)):\n\(text)")
        }
    }
}
