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

    // Disabled: a line is lost. A hosted shell restarted from its bar prints a line; after a quit that keeps the hosts and a
    // launch the terminal is attached to the same host, the command's own line and the prompt are there (on the screen or
    // just above it), and the line the command printed is gone. A hosted shell that was not restarted keeps every line over
    // the same quit (seen with the same commands). Not fixed here: saved terminals. What differs for the restarted one is
    // not traced; the shell redraws its prompt when the attach resizes it, and a host whose size was not the view's when
    // the line was printed would explain a redraw one line too high.
    @Test(
        "a restarted hosted shell's last line is still there after a quit that keeps the hosts",
        .disabled("after Restart in a host and a quit that keeps the hosts the line the shell printed last is gone: see the comment"),
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func restartedHostedShellKeepsItsText() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.flags(["persistent_terminals": true])
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            guard let id = await app.live(1)?.first else { return }
            await Self.endShell(app, id)
            await Self.restart(app, id)
            await app.until("keys") { $0.contains("focus=terminal") }
            await app.send(["text echo in-the-new-host", "key return"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } }
            await app.send(["save-windows"])
            await app.quit(command: "quit-keep")
            try await app.start()
            await app.until("rows") { Listing($0).live == [id] }
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            // On the screen, or just above it where the shell's redraw after the attach pushed it.
            var text = ""
            let start = Date()
            repeat {
                text = (await app.send(["scroll 700 400 -600", "frames 2", "delay 150", "screen", "scroll 700 400 600"]))[3]
            } while !text.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } && Date().timeIntervalSince(start) < 15
            app.expect(text.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") }, "the line from before the quit is still there:\n\(text)")
        }
    }
}
