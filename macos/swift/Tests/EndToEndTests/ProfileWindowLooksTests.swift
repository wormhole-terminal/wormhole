import Foundation
import Testing

/// A window of another profile than the one Wormhole is in ("Open Window in"): it is drawn with its own profile's theme,
/// font and switches, follows that profile's settings while it is open, and says whose it is in its status bar; the windows
/// of the profile Wormhole is in keep theirs. Read per window from what draws it (`window-look`, `status-items`).
extension EndToEnd {
    @Test("a window of another profile wears that profile's theme, terminal font and status bar, follows its settings live, and carries its name")
    @MainActor func otherProfileWindowWearsItsOwnLook() async throws {
        try await Instance.with { app in
            let home = Readout(await app.reply("window-look")).first("window-look")
            guard let homeTheme = home?["theme"], let homeFont = home?["font"], let homeTerminal = home?["terminal"], let homeWindow = home?["window"] else {
                return app.fail("the first window's look")
            }

            // The Work profile, with settings of its own: another theme, a larger terminal font, the status bar on.
            guard let work = await Self.newProfile(app, "Work") else { return app.fail("the Work profile") }
            let theme = homeTheme == "Nord" ? "Dracula" : "Nord"
            await app.send([#"set theme_mode="dark""#, #"set theme="\#(theme)""#, "set font_size=19", "set status_bar=true"])
            let file = app.scratch.data.appendingPathComponent("profiles/\(work)/config.json")
            let written = await app.watch(file.deletingLastPathComponent(), timeout: 15) {
                let text = (try? String(contentsOf: file, encoding: .utf8)) ?? ""
                return text.contains(theme) && text.contains("19")
            }
            app.expect(written, "the Work profile's own file has them: \(file.path)")
            app.expect(!(app.scratch.dataFile("config.json") ?? "").contains(theme), "the Default profile's file does not")

            // Back in Default, a window opened in Work stands beside Default's.
            await app.send(["profile-switch default"])
            await app.until("sessions") { $0.contains("profile=default visible=true") }
            await app.send(["profile-open \(work)"])
            await app.until("sessions") { $0.split(separator: "\n").contains { $0.contains("profile=\(work) ") && $0.contains("visible=true") } }
            guard let other = await app.window(inProfile: work), let mine = await app.window(inProfile: "default") else {
                return app.fail("a window of each profile in sight:\n\(await app.reply("sessions"))")
            }

            // The Work window is drawn with Work's: its theme, its window and terminal colours, its font; the bar on.
            let worn = Readout(await app.until("window-look", window: other) { Readout($0).first("window-look")?["theme"] == theme && Readout($0).first("window-look")?["font"] == "19.0" }).first("window-look")
            app.expect(worn?["profile"] == work, "the window is Work's: \(worn?.text ?? "")")
            app.expect(worn?["status_bar"] == "true", "its status bar is on: \(worn?.text ?? "")")
            app.expect(worn?["terminal"] != homeTerminal, "its terminal has Work's background, not Default's \(homeTerminal): \(worn?.text ?? "")")
            app.expect(worn?["window"]?.hasPrefix("#") == true && worn?["window"] != homeWindow, "its window colour is Work's theme's, not Default's \(homeWindow): \(worn?.text ?? "")")
            // The Default window beside it keeps Default's.
            let kept = Readout(await app.reply("window-look", window: mine)).first("window-look")
            app.expect(
                kept?["theme"] == homeTheme && kept?["font"] == homeFont && kept?["terminal"] == homeTerminal && kept?["window"] == homeWindow && kept?["status_bar"] == "false",
                "Default's window is as it was: \(kept?.text ?? "")")

            // Each bar's profile tag names its own window's profile (the Work window's bar is on show; Default's is read as it would be drawn).
            let tag = Readout(await app.reply("status-items", window: other))
            app.expect(tag.first("profile")?.label == "Work", "the Work window's bar says Work:\n\(tag.text)")
            app.expect(tag.first("status-items")?.has("shown") == true, "and is on show:\n\(tag.text)")
            app.expect(Readout(await app.reply("status-items", window: mine)).first("profile")?.label != "Work", "Default's does not")

            // Work's settings changed while its window is the odd one (its own file, as a person edits it): that window follows,
            // the other does not.
            var json = (try JSONSerialization.jsonObject(with: Data(contentsOf: file)) as? [String: Any]) ?? [:]
            json["font_size"] = 16
            json["status_bar"] = false
            try JSONSerialization.data(withJSONObject: json, options: [.prettyPrinted, .sortedKeys]).write(to: file, options: .atomic)
            await app.until("window-look", window: other) { Readout($0).first("window-look")?["font"] == "16.0" && Readout($0).first("window-look")?["status_bar"] == "false" }
            let after = Readout(await app.reply("window-look", window: mine)).first("window-look")
            app.expect(after?["font"] == homeFont && after?["theme"] == homeTheme, "Default's window did not follow Work's file: \(after?.text ?? "")")
        }
    }
}
