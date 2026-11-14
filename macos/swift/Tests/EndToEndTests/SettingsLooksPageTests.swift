import Foundation
import Testing

/// Settings → Appearance and Advanced by clicks on their own controls (`settings-reveal` scrolls to a row, `settings-press`
/// clicks its control): what the click writes, and what the window then draws with, read from the window's own look
/// (`window-look`, `status-items`) and from what the instance would have opened (`launches`).
extension EndToEnd {
    /// The row brought on show and its control clicked once it stands still.
    @MainActor private static func click(_ app: Instance, _ id: String, at fraction: Double? = nil) async {
        await app.send(["settings-reveal \(id)"])
        await app.settledRow(id)
        await app.until("settings-press \(id)" + (fraction.map { " at \($0)" } ?? "")) { !$0.hasPrefix("error") }
    }

    @MainActor private static func look(_ app: Instance) async -> Readout.Line? { Readout(await app.reply("window-look")).first("window-look") }

    @Test("Appearance by clicks: the status bar's switch shows the bar, Light draws the window and the terminal in the light theme and Dark gives the dark one back, icon animations go off")
    @MainActor func appearancePageClicked() async throws {
        try await Instance.with { app in
            // Dark first, set on the page itself, so the scenario starts from a known side whatever the Mac is in.
            await Self.click(app, "theme-mode", at: 0.12)
            await app.until("get theme_mode") { $0.contains("dark") }
            guard let dark = await Self.look(app), let darkTheme = dark["theme"], let darkTerminal = dark["terminal"] else { return app.fail("the window's look") }
            app.expect(dark["status_bar"] == "false", "the status bar is off as it comes: \(dark.text)")

            // The status bar's switch: written, and the window draws its bar.
            await app.clickSwitch("status-bar", key: "status_bar", becomes: true)
            _ = await app.waitForConfig { $0.range(of: #""status_bar"\s*:\s*true"#, options: .regularExpression) != nil }
            await app.until("window-look") { Readout($0).first("window-look")?["status_bar"] == "true" }
            await app.until("status-items") { Readout($0).first("status-items")?.has("shown") == true }

            // Light (the middle of the three segments): the window and its terminal are drawn in the light pick.
            var chosen = false
            for fraction in [0.5, 0.42, 0.58, 0.36] where !chosen {
                await Self.click(app, "theme-mode", at: fraction)
                await app.send(["delay 300"])
                chosen = (await app.setting("theme_mode")).contains("light")
            }
            app.expect(chosen, "a click on the Light segment chose it")
            let light = Readout(await app.until("window-look") { Readout($0).first("window-look")?["terminal"] != darkTerminal }).first("window-look")
            app.expect(light?["theme"] != darkTheme && light?["window"] != dark["window"], "the window wears the light theme: \(light?.text ?? "")")
            // Dark again: exactly what it was.
            await Self.click(app, "theme-mode", at: 0.12)
            await app.until("window-look") { Readout($0).first("window-look")?["theme"] == darkTheme && Readout($0).first("window-look")?["terminal"] == darkTerminal }

            // Icon animations off: the look the icons read says so.
            app.expect(await app.reply("look-keys"), contains: "icon_animations=true")
            await app.clickSwitch("icon-animations", key: "icon_animations", becomes: false)
            await app.until("look-keys") { $0.contains("icon_animations=false") }
            _ = await app.waitForConfig { $0.range(of: #""icon_animations"\s*:\s*false"#, options: .regularExpression) != nil }
        }
    }

    @Test("the pages' Open and Show in Finder buttons, clicked, ask for the instance's own config.json, data folder, logs and icon themes folder")
    @MainActor func settingsFolderButtonsClicked() async throws {
        try await Instance.with { app in
            let data = app.scratch.data.path.replacingOccurrences(of: "/private/tmp", with: "/tmp")
            /// What the instance recorded instead of opening it, without the difference between /tmp and /private/tmp.
            @MainActor func opened() async -> String { (await app.reply("launches")).replacingOccurrences(of: "/private/tmp", with: "/tmp") }
            app.expect(!(await opened()).contains("open "), "nothing was opened yet")
            for (row, place) in [("open-config", "\(data)/config.json"), ("open-data", data), ("open-logs", "\(data)/logs"), ("icon-themes-folder", "\(data)/icon-themes")] {
                // Clicked again when the click missed: a page still scrolling to the row moves the button from under it (the
                // driver's aim, `settledRow`), and a click that lands is recorded at once.
                var said = ""
                for _ in 0..<3 where !said.split(separator: "\n").contains("open \(place)") {
                    await Self.click(app, row)
                    for _ in 0..<12 where !said.split(separator: "\n").contains("open \(place)") {
                        said = (await app.send(["delay 150", "launches"]))[1].replacingOccurrences(of: "/private/tmp", with: "/tmp")
                    }
                }
                app.expect(said.split(separator: "\n").contains("open \(place)"), "\(row) asks for \(place):\n\(said)")
            }
        }
    }
}
