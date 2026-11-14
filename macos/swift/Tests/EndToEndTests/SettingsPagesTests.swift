import Foundation
import Testing

/// Settings taking effect and being saved: written through the control folder as a control writes them (`set`), read back as the
/// window holds them (`get`, `state`, `fonts`, `look-keys`) and as `config.json` keeps them; Reset all asks first (`question`).
extension EndToEnd {
    /// `number(14.0)` as 14.
    static func number(_ said: String) -> Int? {
        Double(said.replacingOccurrences(of: "number(", with: "").replacingOccurrences(of: ")", with: "")).map { Int($0) }
    }

    /// How many settings Reset all would put back: 0 when the button is off. Its question is held (`question`) unless `keep` is
    /// false, when it is cancelled at once.
    @MainActor static func changedCount(_ app: Instance, keep: Bool = false) async -> Int? {
        let said = await app.reply("settings-reset-confirm")
        if said.contains("every setting is at its default") { return 0 }
        if !keep {
            // The question is put a moment after the press: cancelled once it is up.
            await app.until("question") { $0.contains("title=") }
            await app.send(["question cancel"])
        }
        return said.components(separatedBy: "settings-reset-confirm ").dropFirst().first?.split(separator: " ").first.flatMap { Int($0) }
    }

    @Test("three settings change the window and config.json; Reset all asks, Cancel keeps them, Reset puts them back")
    @MainActor func settingsSavedAndReset() async throws {
        try await Instance.with { app in
            let size = Self.number(await app.setting("font_size")) ?? 13
            let blur = await app.setting("blur").contains("true")
            let minimap = await app.setting("minimap").contains("true")
            let before = await Self.changedCount(app) ?? -1
            // Nothing is asked while nothing differs from the default.
            if before == 0 { app.expect((await app.reply("question")).contains("no question is up"), "no question for nothing to reset") }

            let said = await app.send(["set font_size=\(size + 1)", "set blur=\(!blur)", "set minimap=\(!minimap)"])
            app.expect(!said.joined().contains("refused"), "the three writes are taken:\n\(said.joined(separator: "\n"))")
            await app.until("fonts") { $0.contains("size=\(size + 1)") }
            await app.until("state") { Instance.value("blur", in: $0) == "\(!blur)" }
            _ = await app.waitForConfig { text in
                ["\"font_size\"\\s*:\\s*\(size + 1)", "\"blur\"\\s*:\\s*\(!blur)", "\"minimap\"\\s*:\\s*\(!minimap)"].allSatisfy {
                    text.range(of: $0, options: .regularExpression) != nil
                }
            }
            app.expect(await Self.changedCount(app) == max(before, 0) + 3, "Reset all counts the three")

            _ = await Self.changedCount(app, keep: true)
            app.expect(await app.until("question") { $0.contains("title=") }, contains: "confirm=")
            await app.send(["question cancel"])
            app.expect(Self.number(await app.setting("font_size")) == size + 1, "Cancel resets nothing")

            _ = await Self.changedCount(app, keep: true)
            await app.until("question") { $0.contains("title=") }
            await app.send(["question confirm"])
            await app.until("get font_size") { Self.number($0.components(separatedBy: " = ").last ?? "") == size }
            await app.until("fonts") { $0.contains("size=\(size)") }
            app.expect(await app.setting("minimap").contains("\(minimap)"), "minimap is back")
            _ = await app.waitForConfig { $0.range(of: "\"font_size\"\\s*:\\s*\(size + 1)", options: .regularExpression) == nil }
        }
    }

    @Test("Settings search finds rows by their words, says when nothing matches, and reveals a row on its page")
    @MainActor func settingsSearchAndReveal() async throws {
        try await Instance.with { app in
            let nothing = await app.reply("settings-search zzqqxxvv")
            app.expect(Instance.value("hits", in: nothing) == "0", "nothing matches a word no row has:\n\(nothing)")
            let fonts = await app.reply("settings-search ligatures")
            app.expect(fonts, contains: "font-ligatures")
            await app.send(["settings-search"])
            let revealed = await app.reply("settings-reveal font-code")
            app.expect(revealed, contains: "section=fonts")
            await app.until("settings-rows") { $0.contains("font-code | ") }
            app.expect(await app.reply("settings-reveal no-such-row"), contains: "error: no setting no-such-row")
        }
    }

    @Test("a new profile is made and entered; deleting it puts Wormhole back in Default and removes its folder")
    @MainActor func profiles() async throws {
        try await Instance.with { app in
            let made = await app.reply("profile-new Work")
            app.expect(made, contains: "profile-new Work ok")
            let listed = await app.until("profiles") { $0.contains("\"Work\"") && $0.split(separator: "\n").first { $0.contains("\"Work\"") }?.hasPrefix("*") == true }
            guard let line = listed.split(separator: "\n").first(where: { $0.contains("\"Work\"") }),
                let id = line.split(separator: " ").dropFirst().first.map(String.init)
            else { return }
            let folder = app.scratch.data.appendingPathComponent("profiles/\(id)")
            app.expect(FileManager.default.fileExists(atPath: folder.path), "the profile's folder is made: \(folder.path)")
            await app.until("sessions") { $0.contains("profile=\(id) visible=true") }
            // The Profile page's fields, committed as a blur commits them: the profile Wormhole is in takes them.
            let fields = await app.reply("profile color=green git=Tester")
            app.expect(fields, contains: "|green|")
            app.expect(fields, contains: "problem=none")

            app.expect(await app.reply("profile-delete default"), contains: "error")
            await app.send(["profile-delete \(id)"])
            let back = await app.until("profiles") { !$0.contains("\"Work\"") }
            app.expect(back.split(separator: "\n").first { $0.hasPrefix("*") }?.contains("default") == true, "Wormhole is in Default:\n\(back)")
            app.expect(await app.waitFor(folder, present: false), "the profile's folder is gone")
            let sessions = await app.until("sessions") { !$0.contains("profile=\(id)") }
            app.expect(sessions, contains: "profile=default")
        }
    }

    @Test("the command line tool installs into the instance's own folder only, and uninstalls")
    @MainActor func commandLineTool() async throws {
        try await Instance.with { app in
            let tool = app.scratch.data.appendingPathComponent("cli-bin/wh")
            app.expect(await app.reply("settings-cli-tool"), contains: "settings-cli-tool notInstalled")
            let installed = await app.until("settings-cli-tool install") { $0.contains("settings-cli-tool installed") }
            app.expect(installed, contains: app.scratch.data.lastPathComponent)
            app.expect(FileManager.default.fileExists(atPath: tool.path), "wh is in the instance's folder: \(tool.path)")
            await app.until("settings-cli-tool uninstall") { $0.contains("settings-cli-tool notInstalled") }
            app.expect(!FileManager.default.fileExists(atPath: tool.path), "wh is gone again")
        }
    }

    @Test("Import reads another terminal's settings from a stand-in home, and applies what is ticked")
    @MainActor func importSettings() async throws {
        try await Instance.with { app in
            let home = app.scratch.root.appendingPathComponent("import-home")
            try app.scratch.write(home, ".config/ghostty/config", "font-size = 17\ncursor-style = bar\n")
            let found = await app.reply("import-scan \(home.path)")
            app.expect(found, contains: "ghostty")
            let open = await app.reply("import-open ghostty")
            app.expect(open, contains: "open=ghostty")
            let applied = await app.reply("import-apply ghostty")
            app.expect(!applied.contains("import-apply nothing"), "something was applied:\n\(applied)")
            _ = await app.waitForConfig { $0.contains("\"font_size\"") }
        }
    }

    @Test("the hotkey and the jump key are recorded from their rows; a key the app uses is refused")
    @MainActor func shortcutRecorders() async throws {
        try await Instance.with { app in
            let before = await app.setting("hotkey")
            await app.send(["settings-reveal hotkey"])
            await app.until("settings-rows") { $0.contains("hotkey | ") && $0.contains("jump-waiting-key | ") }
            await app.settledRow("hotkey")
            await app.send(["settings-record hotkey shift-cmd-space"])
            let taken = await app.until("get hotkey") { $0 != "hotkey = \(before)" }
            app.expect(taken, contains: "space")
            await app.send(["settings-record hotkey cmd-t"])
            app.expect(await app.setting("hotkey") == taken.components(separatedBy: " = ").dropFirst().joined(separator: " = "), "⌘T is New Terminal: refused")
            let jumpBefore = await app.setting("jump_waiting_key")
            await app.send(["settings-record jump-waiting-key alt-cmd-k"])
            let jump = await app.until("get jump_waiting_key") { $0 != "jump_waiting_key = \(jumpBefore)" }
            app.expect(jump, contains: "k")
            _ = await app.waitForConfig { $0.contains("\"hotkey\"") && $0.contains("\"jump_waiting_key\"") }
        }
    }

    @Test("a font picker opens on the Fonts page with its query and lists families")
    @MainActor func fontPicker() async throws {
        try await Instance.with { app in
            await app.send(["settings-section fonts"])
            let picker = await app.until("font-picker terminal Menlo") { $0.hasPrefix("font-picker terminal") }
            let entries = Instance.value("entries", in: picker).flatMap { Int($0) } ?? 0
            app.expect(entries > 0, "Menlo is found among the families:\n\(picker)")
            app.expect(await app.reply("font-picker nowhere"), contains: "error")
        }
    }

    @Test("notifications off in config.json reads every kind off; turning one on turns notifications on")
    @MainActor func notificationKinds() async throws {
        try await Instance.with(config: #"{"notifications": false, "notify_bell": true}"#) { app in
            app.expect(await app.setting("notify_bell"), contains: "false")
            await app.send(["set notify_needs_you=true"])
            _ = await app.waitForConfig { $0.range(of: #""notifications"\s*:\s*true"#, options: .regularExpression) != nil }
            app.expect(await app.setting("notify_needs_you"), contains: "true")
        }
    }

    /// All the terminal ids `sessions` lists, over every window.
    static func allTerminals(_ sessions: String) -> Int {
        sessions.split(separator: "\n").reduce(0) { total, line in
            let ids = line.components(separatedBy: "terminals=[").dropFirst().first?.prefix { $0 != "]" } ?? ""
            return total + ids.split(separator: ",").count
        }
    }

    @Test("a Settings row's menu copies its key; ⌃⌥⌘F unlocks the feature flags; the app's keys work with Settings in front")
    @MainActor func settingsRowMenuAndKeys() async throws {
        try await Instance.with { app in
            await app.send(["settings-reveal minimap"])
            let page = await app.until("settings-rows") { $0.contains("minimap | ") }
            let line = page.split(separator: "\n").first { $0.hasPrefix("minimap | ") }
            let frame = line?.components(separatedBy: " | ").last?.split(separator: " ").compactMap { Double($0) } ?? []
            app.expect(frame.count == 4, "the minimap row with its frame:\n\(page)")
            guard frame.count == 4 else { return }
            let menu = await app.send(["pointer-window Settings", "right-click \(Int(frame[0] + frame[2] / 2)) \(Int(frame[1] + frame[3] / 2))", "pointer-window"])
            app.expect(menu[1], contains: "Copy Key")
            app.expect(menu[1], contains: "Reset to Default")
            app.expect(await app.reply("menu-pick Copy Key"), contains: "picked Copy Key")
            app.expect(await app.reply("pasteboard"), contains: "minimap")

            var unlocked = await app.reply("flags-unlock")
            if unlocked.contains("unlocked=false") { unlocked = await app.reply("flags-unlock") }
            app.expect(unlocked, contains: "featureFlags")
            _ = await app.waitForConfig { $0.range(of: #""unlocked"\s*:\s*true"#, options: .regularExpression) != nil }

            // ⇧⌘N with the Settings window taking the keys opens a terminal, as in the workspace window.
            let before = Self.allTerminals(await app.reply("sessions"))
            await app.send(["pointer-window Settings", "key shift-cmd-n", "pointer-window"])
            await app.until("sessions") { Self.allTerminals($0) == before + 1 }
        }
    }
}
