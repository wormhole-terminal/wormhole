import Foundation
import Testing

/// The Theme Studio and the person's own theme files: a theme made, saved under a name, used, saved again (a version kept), copied,
/// and refused a name that is taken; a theme file written by hand and picked, then edited on disk. Read through the `studio-*`
/// replies (the session's own state), the themes folder and `look-keys`.
extension EndToEnd {
    @Test("the Studio makes a theme, saves it under a name, uses it, keeps a version, copies it and asks before replacing a file")
    @MainActor func studioThemes() async throws {
        try await Instance.with { app in
            let themes = app.scratch.data.appendingPathComponent("themes")
            await app.send(["studio"])
            await app.until("studio-select") { !$0.hasPrefix("error") }
            app.expect(await app.later("studio-do open zz-no-such-theme", answer: "studio-do:"), contains: "could not be opened")

            app.expect(await app.later("studio-open new", answer: "studio-open new"), contains: "name=Untitled")
            app.expect(await app.reply("studio-select primary"), contains: "selected=primary")
            let refused = await app.reply("studio-set mix(")
            app.expect(refused, contains: "ok=false")
            app.expect(!refused.contains("field=-"), "the field says why:\n\(refused)")
            app.expect(await app.reply("studio-set #ff8800"), contains: "ok=true")
            let saved = await app.later("studio-save Mine", answer: "studio-save Mine")
            app.expect(saved, contains: "name=Mine")
            app.expect(saved, contains: "dirty=false")
            let file = themes.appendingPathComponent("Mine.json")
            app.expect(await app.waitFor(file), "the theme's file: \(file.path)")
            app.expect(await app.later("studio-use", answer: "studio-use"), contains: "inuse=true")
            _ = await app.waitForConfig { $0.contains("\"Mine\"") }

            // Saved again: the text it replaced is kept as a version.
            app.expect(await app.reply("studio-set #ff8801"), contains: "ok=true")
            await app.later("studio-save", answer: "studio-save")
            let versions = await app.reply("studio-versions")
            app.expect((Instance.value("versions", in: versions).flatMap { Int($0) } ?? 0) >= 2, "the older save is listed:\n\(versions)")
            app.expect((try? String(contentsOf: file, encoding: .utf8))?.lowercased().contains("ff8801") == true, "the file has the new value")

            // Copy JSON goes to the scratch folder's file, never the clipboard.
            await app.later("studio-menu json", answer: "studio-out")
            let copied = try? String(contentsOf: app.scratch.data.appendingPathComponent("studio-out/pasteboard.txt"), encoding: .utf8)
            app.expect(copied?.lowercased().contains("ff8801") == true, "the theme's JSON was copied to the file:\n\(copied ?? "none")")
            app.expect(await app.reply("pasteboard"), contains: "pasteboard empty")
            let shot = app.scratch.control.appendingPathComponent("studio1.png")
            await app.send(["studio-shot studio1"])
            app.expect(FileManager.default.fileExists(atPath: shot.path), "the Studio's picture: \(shot.path)")

            // Duplicate: a copy under a free name, in a file of its own; the original stays as saved.
            app.expect(await app.later("studio-menu duplicate", answer: "studio-menu duplicate"), contains: "copy")
            let copyFile = await app.watch(themes, timeout: 15) {
                ((try? FileManager.default.contentsOfDirectory(atPath: themes.path)) ?? []).contains { $0.contains("copy") }
            }
            app.expect(copyFile, "the copy has a file of its own")

            // Another new theme saved as Mine: the name is taken, so it asks; Cancel leaves the file alone.
            await app.later("studio-open new", answer: "studio-open new")
            await app.send(["studio-select primary", "studio-set #000001", "studio-save Mine"])
            app.expect(await app.until("question") { $0.contains("title=") }, contains: "Mine")
            await app.send(["question cancel"])
            app.expect((try? String(contentsOf: file, encoding: .utf8))?.contains("000001") == false, "the file was not replaced")
        }
    }

    @Test("a theme file put in the themes folder is picked by its name, and an edit of the file on disk recolours the app")
    @MainActor func userThemeFile() async throws {
        try await Instance.with { app in
            let themes = app.scratch.data.appendingPathComponent("themes")
            try app.scratch.write(themes, "Mine.json", ##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#ff6600"}}"##)
            // The pick for either appearance, so the system's appearance does not matter.
            await app.send(["set theme=\"Mine\"", "set theme_light=\"Mine\""])
            await app.until("look-keys") { $0.contains("primary=#ff6600") }
            try app.scratch.write(themes, "Mine.json", ##"{"name": "Mine", "extends": "Nord", "ui": {"primary": "#00aa55"}}"##)
            await app.until("look-keys") { $0.contains("primary=#00aa55") }
            app.expect(await app.reply("state"), contains: "theme=")
        }
    }
}
