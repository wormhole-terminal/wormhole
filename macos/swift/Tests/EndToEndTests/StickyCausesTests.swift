import Foundation
import Testing

/// The warnings that stay until they are closed, posted by what causes them instead of by `notice sticky`: a session host
/// that cannot be started, and a profile whose data folder cannot be opened. (An installed update's is
/// `UpdateFeedTests/updateReadyChipAndPage`.) Read from the chip the window shows (`notice chip`).
extension EndToEnd {
    @Test(
        "a session host that cannot start: the terminal runs in the app, and a warning that stays says so once, with why",
        .enabled(if: Instance.hostProgram() != nil, "no session host is built: cargo build -p wormhole-host"))
    @MainActor func hostFallbackWarns() async throws {
        try await Instance.with(hosts: true) { app in
            let hosts = Instance.hostIDs(in: await app.until("sessions") { Instance.hostIDs(in: $0).count == 1 })
            app.expect(await app.reply("notice chip"), contains: "shown=none")

            // The helper goes missing (the scratch's own link to it): the next terminal has no host to start in.
            try FileManager.default.removeItem(at: app.scratch.bin.appendingPathComponent("wormhole-host"))
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 2 }
            let chip = Instance.plain(await app.until("notice chip") { $0.contains("ends when") })
            app.expect(chip, contains: "shown=This terminal ends when Wormhole quits")
            app.expect(chip, contains: "sticky=true")
            app.expect(!chip.contains("detail=-"), "the warning carries why the helper did not start: \(chip)")

            // The terminal runs all the same, in the app: it echoes, and no second host exists.
            await app.send(["type runs-in-the-app\\n"])
            await app.screen(showing: "runs-in-the-app")
            app.expect(Set(app.scratch.liveHosts().keys) == Set(hosts), "no host was started for it: \(app.scratch.liveHosts())")
            app.expect(Instance.hostIDs(in: await app.reply("sessions")) == hosts, "only the first terminal is hosted")

            // It stays until closed: still there after other things happened; its ⓘ opens the explanation; × closes it.
            await app.send(["delay 2500"])
            app.expect(await app.reply("notice chip"), contains: "ends when")
            await app.send(["notice-click info"])
            await app.until("notice chip") { $0.contains("info=true") }
            await app.send(["notice-click close"])
            await app.until("notice chip") { $0.contains("shown=none") }

            // Said once a run: the next terminal that falls back does not say it again.
            await app.send(["key cmd-t"])
            await app.until("rows") { Listing($0).live.count == 3 }
            await app.send(["type third-runs-too\\n"])
            await app.screen(showing: "third-runs-too")
            app.expect(await app.reply("notice chip"), contains: "shown=none")
        }
    }

    // GPUI records a profile whose database cannot be opened where its data is opened (`app_state/profiles.rs`, `ProfileData::open`,
    // `data_folder::record`: the log, standard error, and the first window's chip at a launch) and then runs that profile
    // with no store, so it saves nothing. The Swift app does not enter a profile it cannot save: `profile-switch` fails in
    // `Windows.beside`, before any window of it opens and before anything of its is written, says so on the chip and stays.
    // The fixture is a file where the profile's folder should be: a folder the owner loses every permission on is made
    // private again by core's `ensure_private_dir`, in both apps, and then opens.
    @Test(
        "a profile whose data folder cannot be opened: Wormhole stays where it is and a warning that stays names the profile, the folder and why")
    @MainActor func unopenableProfileFolderWarns() async throws {
        try await Instance.with { app in
            // A second profile, then back to Default: its folder is the one that will not open.
            app.expect(await app.reply("profile-new Work"), contains: "profile-new Work")
            let listed = await app.until("profiles") { $0.contains("\"Work\"") }
            guard let id = listed.split(separator: "\n").first(where: { $0.contains("\"Work\"") })?.split(separator: " ").first(where: { $0 != "*" }).map(String.init) else {
                return app.fail("the new profile's id:\n\(listed)")
            }
            await app.send(["profile-switch default"])
            await app.until("profiles") { $0.split(separator: "\n").contains { $0.hasPrefix("* default ") } }
            let folder = app.scratch.data.appendingPathComponent("profiles/\(id)")
            app.expect(FileManager.default.fileExists(atPath: folder.path), "the profile has a folder of its own: \(folder.path)")
            // Quit, and a file stands where its folder was: the next launch finds a profile it cannot open.
            await app.quit(command: "quit")
            try FileManager.default.removeItem(at: folder)
            try Data("not a folder".utf8).write(to: folder)
            try await app.start()
            await app.until("profiles") { $0.split(separator: "\n").contains { $0.hasPrefix("* default ") } }

            await app.send(["profile-switch \(id)"])
            let chip = Instance.plain(await app.until("notice chip") { $0.contains("data folder") })
            app.expect(chip, contains: "sticky=true")
            app.expect(chip, contains: "can't open a profile's data folder, so it saves nothing there")
            app.expect(chip.contains(id) && chip.contains("profiles/\(id)"), "the explanation names the profile and its folder: \(chip)")
            // Nothing else changed: Wormhole is still in Default, with its window and terminal.
            app.expect((await app.reply("profiles")).split(separator: "\n").contains { $0.hasPrefix("* default ") }, "Wormhole stays in Default")
            app.expect((await app.rows()).live.count == 1, "the window keeps its terminal")
            app.expect(FileManager.default.contents(atPath: folder.path) == Data("not a folder".utf8), "nothing was written where the profile's folder was")
            await app.send(["notice-click close"])
            await app.until("notice chip") { $0.contains("shown=none") }
        }
    }
}
