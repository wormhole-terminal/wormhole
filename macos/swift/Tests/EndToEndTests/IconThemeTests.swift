import Foundation
import Testing

/// File icon themes: a family in the profile's `icon-themes` folder is offered in Settings → Appearance, the `icon_theme`
/// setting makes it the one the rows draw from, a file type it does not know draws no icon of its, None draws none at all,
/// and the bundled theme comes back. Read from the icon source the Files, Changes and History rows ask (`file-icon`).
extension EndToEnd {
    @Test("an icon theme in the icon-themes folder is offered, picked by the setting and drawn from; None and the bundled theme again")
    @MainActor func iconThemePicked() async throws {
        try await Instance.with { app in
            // The bundled theme first: a Rust file's icon is one of the app's own.
            let bundled = Readout(await app.until("file-icon main.rs") { Readout($0).first("file-icon")?["icon"] != "-" }).first("file-icon")?["icon"] ?? ""
            app.expect(bundled.hasSuffix(".svg") && !bundled.contains(app.scratch.data.path), "the bundled theme's icon is the app's: \(bundled)")

            // A family of the person's own, in the icon-themes folder of the data folder (Zed's format).
            let family = app.scratch.data.appendingPathComponent("icon-themes/e2e")
            let svg = #"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16"/></svg>"#
            for name in ["rust.svg", "folder.svg", "folder-open.svg"] { try app.scratch.write(family, "icons/\(name)", svg) }
            try app.scratch.write(family, "icon-theme.json", """
                { "name": "E2E Icons", "author": "a test", "themes": [ { "name": "E2E Icons", "appearance": "dark",
                  "directory_icons": { "collapsed": "./icons/folder.svg", "expanded": "./icons/folder-open.svg" },
                  "file_suffixes": { "rs": "rust" },
                  "file_icons": { "rust": { "path": "./icons/rust.svg" } } } ] }
                """)

            // Settings → Appearance offers it by its name, beside the bundled one and None.
            let offered = Readout(await app.until("file-icon themes") { $0.contains("E2E Icons") }).all("theme").map(\.label)
            app.expect(offered.first == "Wormhole Icons" && offered.last == "None" && offered.contains("E2E Icons"), "the menu's choices: the bundled theme, the person's, None: \(offered)")

            // While the File icon themes feature is off (a release's default) the setting is kept and not in force: the
            // rows keep the bundled icons.
            await app.flags(["icon_themes": false])
            await app.send([#"set icon_theme="E2E Icons""#])
            _ = await app.waitForConfig { $0.contains("E2E Icons") }
            await app.send(["delay 1200"])
            app.expect(Readout(await app.reply("file-icon main.rs")).first("file-icon")?["icon"] == bundled, "the bundled icons stay while the feature is off")

            // The feature on: the rows draw its icons, a folder's too; a file type it does not know has no icon of its.
            await app.flags(["icon_themes": true])
            let own = family.appendingPathComponent("icons").path.replacingOccurrences(of: "/private/tmp", with: "/tmp")
            let picked = await app.until("file-icon main.rs") { $0.replacingOccurrences(of: "/private/tmp", with: "/tmp").contains("\(own)/rust.svg") }
            app.expect(picked.replacingOccurrences(of: "/private/tmp", with: "/tmp"), contains: "\(own)/rust.svg")
            app.expect((await app.reply("file-icon dir:src open")).replacingOccurrences(of: "/private/tmp", with: "/tmp"), contains: "\(own)/folder-open.svg")
            app.expect((await app.reply("file-icon dir:src")).replacingOccurrences(of: "/private/tmp", with: "/tmp"), contains: "\(own)/folder.svg")
            app.expect(await app.reply("file-icon notes.py"), contains: #"icon="-""#)
            // The Appearance page's row says the theme by its name.
            await app.send(["settings-section appearance"])
            await app.until("settings-page") { Readout($0).line("row", "icon-theme")?["value"] == "E2E Icons" }

            // None: no icon for anything. The bundled theme again: the app's own icon as before.
            await app.send([#"set icon_theme="None""#])
            await app.until("file-icon main.rs") { $0.contains(#"icon="-""#) }
            await app.send([#"set icon_theme="Wormhole Icons""#])
            await app.until("file-icon main.rs") { Readout($0).first("file-icon")?["icon"] == bundled }
        }
    }
}
