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

    // A terminal restored at a launch whose helper is found but does not start is said to end with the app, as one that falls
    // back while the app runs is (`StickyCausesTests/hostFallbackWarns`). GPUI's rule (`window/hosting.rs` `start_with`): the
    // host program is looked for once, and a helper that is missing then is a silent `Start::Local`, so a launch with no helper
    // says nothing in either app; the warning comes from a host that was to start and failed (`terminal_view/hosted.rs`
    // `host_ready`, told once per run). This scenario's helper is a program that exits at once: found, and no host.
    @Test(
        "a hosted terminal restored at a launch where the helper is found but does not start is said to end with the app",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func fallbackAtLaunchIsSaid() async throws {
        try await Instance.with(hosts: true) { app in
            await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 }
            await app.quit(command: "quit-end-all")
            let helper = app.scratch.bin.appendingPathComponent("wormhole-host")
            try FileManager.default.removeItem(at: helper)
            try Data("#!/bin/sh\nexit 1\n".utf8).write(to: helper)
            try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: helper.path)
            try await app.start()
            let chip = Instance.plain(await app.until("notice chip") { $0.contains("ends when") })
            app.expect(chip, contains: "sticky=true")
            app.expect(!chip.contains("detail=-"), "the warning carries why the helper did not start: \(chip)")
            app.expect((await app.rows()).live.count == 1, "the terminal came back in the app")
        }
    }
}
