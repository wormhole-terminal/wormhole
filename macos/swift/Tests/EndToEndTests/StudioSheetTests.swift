import Foundation
import Testing

/// The Theme Studio's Versions sheet as AppKit has it: `state`'s `modal=versions` says a flag the `studio-versions` command
/// itself set, so this reads the Studio window's attached sheet (`windows`: `sheet=true`) while Browse All Versions is up,
/// and that Done takes it down.
extension EndToEnd {
    /// The Studio window's line of `windows` (`Theme Studio … visible=… sheet=… edited=…`).
    private static func studioWindow(_ windows: String) -> String? {
        windows.split(separator: "\n").map(String.init).first { $0.contains("Studio") }
    }

    @Test("Browse All Versions puts a sheet on the Studio window, with the saves behind the file; Done takes the sheet down")
    @MainActor func studioVersionsIsASheet() async throws {
        try await Instance.with { app in
            guard let profile = app.configFile()?.deletingLastPathComponent() else { return app.fail("the app has written no config.json yet") }
            try app.scratch.write(profile, "themes/Mine.json", "{ \"name\": \"Mine\", \"extends\": \"Nord\" }\n")
            await app.send(["studio"])
            await app.until("state") { $0.contains("\nstudio dirty=") }
            app.expect(await app.later("studio-open own:Mine", answer: "studio-open own:Mine"), contains: "name=Mine")
            // A save, so the file has a version behind it.
            await app.send(["studio-select text.muted", "studio-set #8899aa"])
            app.expect(await app.later("studio-save", answer: "studio-save"), contains: "dirty=false")
            let before = Self.studioWindow(await app.reply("windows")) ?? ""
            app.expect(before.contains("sheet=false"), "no sheet on the Studio before: \(before)")

            let versions = await app.reply("studio-versions")
            app.expect(versions, contains: "versions=")
            await app.until("windows") { Self.studioWindow($0)?.contains("sheet=true") == true }
            // The workspace window has none: the sheet is the Studio's.
            let all = await app.reply("windows")
            app.expect(all.split(separator: "\n").filter { $0.contains("sheet=true") }.count == 1, "one window has a sheet, the Studio:\n\(all)")

            await app.send(["studio-versions close"])
            await app.until("windows") { Self.studioWindow($0)?.contains("sheet=false") == true }
        }
    }
}
