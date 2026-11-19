import Foundation
import Testing

/// Wormhole launched in a profile whose data folder cannot be opened (a file stands where it was). GPUI runs it with no
/// store, says so on the first window's chip (`app::launch` `tell_data_folder`) and saves nothing; what the profile's folder
/// held is not touched.
///
/// Disabled, diagnosed only. The Swift app does not start: `AppController.init` opens the shown profile's saved windows
/// (`Windows::in_profile`, whose `open_store` fails with `FfiError::Start`), the error reaches `main.swift`, which writes it to
/// standard error and exits 2 with no window. GPUI's rule is `ProfileData::open` (`app_state/profiles.rs`): the open's error is
/// recorded (`data_folder::record`) and dropped (`.ok()`), the store is `None`, nothing is saved, and `app::launch`'s
/// `tell_data_folder` puts the warning on the first window. Matching it means a windows registry with no store behind it in the
/// bridge (`crates/wormhole-ffi/src/windows`), which is how windows and terminals are SAVED: not changed here.
extension EndToEnd {
    @Test(
        "Wormhole launched in a profile whose data folder cannot be opened runs, says so on a chip that stays, and writes nothing there",
        .disabled("the app exits at launch when the shown profile's folder cannot be opened, where GPUI runs without a store: see the comment"))
    @MainActor func unopenableShownProfileAtLaunch() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("profile-new Work"), contains: "profile-new Work")
            let listed = await app.until("profiles") { $0.contains("\"Work\"") }
            guard let id = listed.split(separator: "\n").first(where: { $0.contains("\"Work\"") })?.split(separator: " ").first(where: { $0 != "*" }).map(String.init) else {
                return app.fail("the new profile's id:\n\(listed)")
            }
            await app.send(["profile-switch \(id)"])
            await app.until("profiles") { $0.split(separator: "\n").contains { $0.hasPrefix("* \(id) ") } }
            await app.send(["save-windows"])
            let folder = app.scratch.data.appendingPathComponent("profiles/\(id)")
            await app.quit(command: "quit")
            try FileManager.default.removeItem(at: folder)
            try Data("not a folder".utf8).write(to: folder)
            try await app.start()
            let profiles = await app.until("profiles") { $0.contains("\"Work\"") }
            app.expect(profiles.contains("* \(id) ") || profiles.contains("* default "), "the app runs:\n\(profiles)")
            let chip = Instance.plain(await app.until("notice chip") { $0.contains("data folder") })
            app.expect(chip, contains: "sticky=true")
            app.expect(chip.contains(id) && chip.contains("profiles/\(id)"), "the explanation names the profile and its folder: \(chip)")
            app.expect(FileManager.default.contents(atPath: folder.path) == Data("not a folder".utf8), "nothing was written where the profile's folder was")
        }
    }
}
