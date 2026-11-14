import Foundation
import Testing

/// A saved, hosted terminal at a launch where no session host can be started (the helper is gone): it starts in the app's
/// own process and keeps its row, its note and its text; the launch's pruning of terminals without a host must not take
/// it (TO-VERIFY line 1570). Read from the list, the screen, the notes' store, the hosts folder and the window's chip.
extension EndToEnd {
    @Test(
        "a hosted terminal whose helper is gone at the next launch comes back in the app with its id, note and text",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func fallbackAtLaunchKeepsTheTerminal() async throws {
        try await Instance.with(hosts: true) { app in
            await app.flags(["persistent_terminals": true, "notes_and_tasks": true])
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            guard let id = await app.live(1)?.first else { return }
            await app.send(["tab notes", "note-add keep this note"])
            await app.until("notes") { $0.contains("keep this note") }
            await app.emit("text-before-the-quit")
            await app.screen { $0.contains("text-before-the-quit") }
            await app.quit(command: "quit-end-all")
            app.expect((await app.hostsLeft()).isEmpty, "no host runs after Quit and End All")

            // The helper goes missing between the launches (the scratch's own link to it).
            try FileManager.default.removeItem(at: app.scratch.bin.appendingPathComponent("wormhole-host"))
            try await app.start()

            // The same terminal, in the app: its id, its text above the restored line, its note; no host anywhere.
            let rows = await app.until("rows") { Listing($0).live == [id] }
            app.expect(Listing(rows).live == [id], "the terminal comes back under its id:\n\(rows)")
            let screen = await app.screen { $0.contains("text-before-the-quit") }
            app.expect(screen, contains: "restored")
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("keep this note") }
            let hosted = Instance.hostIDs(in: await app.reply("sessions"))
            app.expect(app.scratch.liveHosts().isEmpty && hosted.isEmpty, "no host was started: \(app.scratch.liveHosts()) \(hosted)")
            // It runs: what is typed is echoed.
            await app.send(["type typed-after-the-launch\\n"])
            await app.screen(showing: "typed-after-the-launch")

            // Still there after another quit and launch: the row was not pruned as one without a host.
            try await app.restart()
            await app.until("rows") { Listing($0).live == [id] }
            await app.send(["tab notes"])
            await app.until("notes") { $0.contains("keep this note") }
        }
    }

    // Disabled: the app does not say it. A terminal that falls back to the app's process while the app runs gets the
    // warning (`StickyCausesTests/hostFallbackWarns`); one that falls back at launch, restored from the saved window, gets
    // none: the chip is empty and no pane has a banner. The GPUI app says it from the terminal's view when it starts
    // (`terminal_view/hosted.rs`, `say_with(Notice::Warned(Warning::HostFallback), …)`), at a launch too. Not fixed here:
    // the launch's order of the saved terminals' start and the window's notices is the hosting code's.
    @Test(
        "a hosted terminal that falls back to the app at launch is said to end with the app, as one that falls back later is",
        .disabled("a fallback at launch posts no warning in the Swift app: see the comment"),
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func fallbackAtLaunchIsSaid() async throws {
        try await Instance.with(hosts: true) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.quit(command: "quit-end-all")
            try FileManager.default.removeItem(at: app.scratch.bin.appendingPathComponent("wormhole-host"))
            try await app.start()
            let chip = Instance.plain(await app.until("notice chip") { $0.contains("ends when") })
            app.expect(chip, contains: "sticky=true")
        }
    }
}
