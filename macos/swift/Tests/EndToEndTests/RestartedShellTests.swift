import Foundation
import Testing

/// A terminal whose shell ended, started again from its bar (Restart): it is the same terminal, with its note, in the app or
/// in a new host of its own, and it is still there with its note at the next launch (TO-VERIFY line 1569). Read from the
/// list, the notes' store, `sessions` and the hosts folder.
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

    /// A line typed at the restarted terminal's shell, and its screen once a line of it starts with `word`. The screen is read
    /// from frames forced on the terminal's view (`frames`): the view of a restarted terminal is not put back into its pane
    /// (`restartedTerminalIsBackInItsPane` below), so nothing draws it by itself.
    @MainActor private static func answer(_ app: Instance, to line: String, startingWith word: String) async -> String {
        var screen = ""
        let start = Date()
        repeat {
            if !(await app.send(["frames 3", "wait-idle 1500", "screen"]))[2].allSatisfy(\.isWhitespace) { break }
        } while Date().timeIntervalSince(start) < 15
        await app.shell(line)
        repeat {
            screen = (await app.send(["frames 2", "delay 200", "screen"]))[2]
        } while !screen.split(separator: "\n").contains { $0.hasPrefix(word) } && Date().timeIntervalSince(start) < 30
        app.expect(screen.split(separator: "\n").contains { $0.hasPrefix(word) }, "the new shell answered `\(line)`:\n\(screen)")
        return screen
    }

    @Test("Restart on an ended shell's bar: the same terminal runs a new shell, its row is idle again and it keeps its note, over a relaunch too")
    @MainActor func restartedShellKeepsItsNote() async throws {
        try await Instance.with(shell: .zsh) { app in
            await app.flags(["notes_and_tasks": true])
            guard let id = await app.live(1)?.first else { return }
            app.expect(await app.reply("pane-restart"), contains: "error")
            await app.send(["tab notes", "note-add survives the restart"])
            await app.until("notes") { $0.contains("survives the restart") }
            await app.shell("echo shell-one-$$")
            let first = await app.screen { $0.contains("\nshell-one-") || $0.hasPrefix("shell-one-") }

            await Self.endShell(app, id)
            await Self.restart(app, id)
            // A new shell (another process) in the same terminal.
            let second = await Self.answer(app, to: "echo shell-two-$$", startingWith: "shell-two-")
            let pid = { (screen: String, word: String) in screen.split(separator: "\n").first { $0.hasPrefix(word) }.map { $0.dropFirst(word.count).trimmingCharacters(in: .whitespaces) } }
            app.expect(pid(first, "shell-one-") != nil && pid(second, "shell-two-") != nil && pid(first, "shell-one-") != pid(second, "shell-two-"), "another shell answers: \(String(describing: pid(first, "shell-one-"))) then \(String(describing: pid(second, "shell-two-")))")
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
        "Restart on a hosted terminal's ended shell: a new host of its own, the old one gone, the note kept; after a quit that keeps the hosts it is attached to that host with its note",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func restartedHostedShellKeepsItsNote() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.flags(["persistent_terminals": true, "notes_and_tasks": true])
            guard let old = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }).first, let id = await app.live(1)?.first else { return }
            await app.send(["tab notes", "note-add survives the restart"])
            await app.until("notes") { $0.contains("survives the restart") }

            await Self.endShell(app, id)
            await Self.restart(app, id)
            // A new host for the same terminal; the one whose shell ended is gone from the folder's live hosts.
            let now = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 && !Instance.hostIDs(in: $0).contains(old) })
            let folder = app.scratch.hostsFolder
            app.expect(await app.watch(folder, timeout: 15) { Set(Scratch.liveHosts(in: folder).keys) == Set(now) }, "only the new host runs: \(app.scratch.liveHosts()) for \(now)")
            app.expect((await app.rows()).live == [id], "it is the same terminal")
            _ = await Self.answer(app, to: "echo in-the-new-host", startingWith: "in-the-new-host")
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

    // Disabled: the screen of a hosted terminal that was restarted does not come back whole. Before the quit the new shell's
    // line is on its screen (read from the terminal's frames); after a quit that keeps the hosts and a launch the terminal is
    // attached to the same host and shows a prompt only. A hosted terminal that was not restarted keeps its text over the same
    // quit (`hostedTerminalOutlivesTheApp`). Probably the same cause as `restartedTerminalIsBackInItsPane`: the restarted
    // terminal's view is never in its pane, so its host keeps the size it started with and is resized at the next attach.
    // Not fixed here: saved terminals. The scenario asserts what a person expects: the text is still there.
    @Test(
        "a restarted hosted terminal's text is still on its screen after a quit that keeps the hosts",
        .disabled("after Restart and a quit that keeps the hosts the terminal comes back with a prompt only: see the comment"),
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func restartedHostedShellKeepsItsText() async throws {
        try await Instance.with(shell: .zsh, hosts: true) { app in
            await app.flags(["persistent_terminals": true])
            guard await app.until("sessions", { Instance.hostIDs(in: $0).count == 1 }).isEmpty == false, let id = await app.live(1)?.first else { return }
            await Self.endShell(app, id)
            await Self.restart(app, id)
            _ = await Self.answer(app, to: "echo in-the-new-host", startingWith: "in-the-new-host")
            await app.send(["save-windows"])
            await app.quit(command: "quit-keep")
            try await app.start()
            await app.until("rows") { Listing($0).live == [id] }
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("in-the-new-host") } }
        }
    }

    // Disabled: an app defect. After Restart the new terminal runs (it answers what is typed into it, its row is idle) but
    // its view is not in the pane: the pane's container has no terminal view under it, the screen stays empty unless frames
    // are forced, a click on the pane gives nothing the keys (`keys`: `focus=none responder=WorkspaceWindow`) and typed keys
    // reach no terminal. The GPUI app makes a new pane in the old one's place and gives it the focus when it is the active
    // one (`window/panes/mod.rs` `restart_pane`). Not fixed here: the pane's host and the window's focus are being worked on
    // in another lane this round (`TerminalHost.swift`, `WindowFocus.swift`). The scenario asserts GPUI's behaviour.
    @Test(
        "after Restart the terminal is back in its pane: drawn there, with the keys, and typing reaches the new shell",
        .disabled("the restarted terminal's view is not put back into its pane and gets no keys: see the comment"))
    @MainActor func restartedTerminalIsBackInItsPane() async throws {
        try await Instance.with(shell: .zsh) { app in
            guard let id = await app.live(1)?.first, let frame = await app.terminalFrames().first else { return app.fail("a terminal on screen") }
            let middle = "\(Int(frame.midX)) \(Int(frame.midY))"
            app.expect(await app.reply("bounds \(middle) 1"), contains: "MetalTerminalView")
            await Self.endShell(app, id)
            await Self.restart(app, id)
            // Its view is the one under the pane's middle again, and it has the keys.
            await app.until("bounds \(middle) 1") { $0.contains("MetalTerminalView") }
            await app.until("keys") { $0.contains("focus=terminal") }
            // Typed as the keyboard types, with nothing forced: the new shell's answer is drawn.
            await app.send(["text echo typed-after-restart", "key return"])
            await app.screen { $0.split(separator: "\n").contains { $0.hasPrefix("typed-after-restart") } }
        }
    }
}
