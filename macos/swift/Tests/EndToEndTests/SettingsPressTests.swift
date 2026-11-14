import Foundation
import Testing

/// Settings' buttons pressed as a click presses them (`settings-press <id>`, at the control's recorded frame in the Settings
/// window), and the rest of what `settings-page` reads: the title row's parts, the Fonts specimen and the picker's preview, the
/// test notification's line, the nav's warning dot, a project's rows. TO-VERIFY.md lines 123, 133, 134, 135, 717, 755, 857,
/// 858, 866, 867, 870, 943, 1145.
extension EndToEnd {
    /// Presses `id` once the page shows it (its frame recorded).
    @MainActor static func press(_ app: Instance, _ id: String) async -> String {
        await app.until("settings-press \(id)") { !$0.hasPrefix("error") }
    }

    @Test("Changed → Copy as JSON puts the listed keys on the clipboard and says Copied once (123, 755, 943)")
    @MainActor func settingsCopyChangedJSON() async throws {
        try await Instance.with { app in
            await app.send(["set blur=false", "settings-section changed"])
            await app.until("settings-rows") { $0.contains("changed.copy | ") }
            app.expect(await Self.press(app, "changed.copy"), contains: "settings-press changed.copy")
            let copied = await app.until("pasteboard") { $0.contains("blur") }
            app.expect(copied, contains: "false")
            app.expect(!copied.contains("font_size"), "only the listed keys:\n\(copied)")
            let said = await app.reply("a11y said")
            app.expect(said.components(separatedBy: "said Copied").count - 1 == 1, "Copied is said once:\n\(said)")
        }
    }

    @Test("Appearance → Theme Studio opens the Studio; Import theme picks the file; Move to Trash puts it in the data folder's trash (133, 134)")
    @MainActor func settingsThemeButtons() async throws {
        try await Instance.with { app in
            await app.send(["settings-section appearance"])
            _ = await Self.press(app, "open-studio")
            await app.until("studio-select") { !$0.hasPrefix("error") }

            let file = app.scratch.root.appendingPathComponent("Imported.json")
            try Data(##"{"name": "Imported", "extends": "Nord", "ui": {"primary": "#ff6600"}}"##.utf8).write(to: file)
            await app.send(["panel-pick \(file.path)"])
            _ = await Self.press(app, "import-theme")
            let listed = Readout(await app.until("theme-cards") { Readout($0).all("card").contains { $0.label == "Imported" && $0.has("picked") } })
            guard let value = listed.all("card").first(where: { $0.label == "Imported" })?["value"] else { return }
            let menu = await app.until("settings-press theme-card.\(value) menu") { $0.hasPrefix("menu:") }
            app.expect(menu, contains: "Move to Trash")
            await app.send(["menu-pick Move to Trash…"])
            await app.asked()
            await app.send(["question confirm"])
            await app.until("theme-cards") { !Readout($0).all("card").contains { $0.label == "Imported" } }
            let trash = app.scratch.data.appendingPathComponent("trash")
            let kept = (try? FileManager.default.contentsOfDirectory(atPath: trash.path)) ?? []
            app.expect(kept.contains { $0.contains("Imported") }, "the file went to \(trash.path): \(kept)")
        }
    }

    @Test("Terminal → Choose app… with an app picked writes its bundle id (135)")
    @MainActor func settingsChooseApp() async throws {
        try await Instance.with { app in
            await app.send(["settings-section terminal", "panel-pick /System/Applications/Utilities/Terminal.app"])
            _ = await Self.press(app, "open-in.choose")
            await app.until("get open_in") { $0.contains("com.apple.Terminal") }
        }
    }

    @Test("Send test says what happened under the row: sent while allowed, off while denied (717)")
    @MainActor func settingsSendTest() async throws {
        try await Instance.with { app in
            await app.send(["notify-permission allowed"])
            _ = await Self.page(app, "notifications") { $0.line("row", "notify-test") != nil }
            _ = await Self.press(app, "notify-test")
            await app.until("settings-page") { Readout($0).under("row", "notify-test").contains { $0.kind == "caption" && $0.label.hasPrefix("Sent") } }
            await app.send(["notify-permission denied"])
            _ = await Self.press(app, "notify-test")
            await app.until("settings-page") { Readout($0).under("row", "notify-test").contains { $0.kind == "caption" && $0.label.hasPrefix("Off in System Settings") } }
        }
    }

    @Test("the title row's parts: the flexible space says what it does, Terminal status mentions the bar while it is on (857)")
    @MainActor func settingsTitleParts() async throws {
        try await Instance.with { app in
            let page = await Self.page(app, "appearance") { $0.under("row", "title-row").contains { $0.kind == "part" } }
            let parts = page.under("row", "title-row").filter { $0.kind == "part" }
            app.expect(parts.first { $0.words.first == "space" }?["note"]?.isEmpty == false, "the space's note:\n\(page.text)")
            app.expect(parts.first { $0.words.first == "terminal_status" }?["note"] == nil, "no note while the bar is off:\n\(page.text)")
            await app.send(["set status_bar=true"])
            await app.until("settings-page") { Readout($0).under("row", "title-row").first { $0.words.first == "terminal_status" }?["note"] != nil }
        }
    }

    @Test("the Fonts specimen's three tabs at each slot's family and size; a picker's look shows in its slot (858, 1145)")
    @MainActor func settingsFontSpecimen() async throws {
        try await Instance.with { app in
            await app.send(["set code_font_size=17"])
            let page = await Self.page(app, "fonts") { $0.all("specimen").count == 3 }
            app.expect(page.line("specimen", "code")?["size"] == "17", "the Code tab at the code size:\n\(page.text)")
            app.expect(page.line("specimen", "terminal")?.label.contains("line height") == true, "the Terminal tab's facts say the line height")
            app.expect(page.line("specimen", "code")?.label.contains("line height") == false, "the others do not")
            await app.send(["font-picker code Menlo"])
            let preview = Readout(await app.until("settings-page") { Readout($0).first("preview") != nil })
            app.expect(preview.first("preview")?.words.first == "code" && preview.first("preview")?["size"] == "17", "the Code slot at its size:\n\(preview.text)")
        }
    }

    @Test("an agent hooked only in part marks Coding agents in the nav (866)")
    @MainActor func settingsAgentsNavMark() async throws {
        try await Instance.with(agents: ["claude"]) { app in
            await app.send(["settings-section agents"])
            _ = await Self.press(app, "agent.claude.hooks")
            let file = app.scratch.agentHome.appendingPathComponent(".claude/settings.json")
            app.expect(await app.waitFor(file), "the hooks were written: \(file.path)")
            let data = try Data(contentsOf: file)
            guard var json = try JSONSerialization.jsonObject(with: data) as? [String: Any], var hooks = json["hooks"] as? [String: Any],
                let first = hooks.keys.sorted().first
            else { return app.fail("hooks in \(file.path)") }
            hooks[first] = nil
            json["hooks"] = hooks
            try JSONSerialization.data(withJSONObject: json).write(to: file)
            await app.send(["settings-section agents"])
            await app.until("settings-page") { Readout($0).line("nav", "agents")?.has("attention") == true }
        }
    }

    @Test("a trusted project that sets a font and a theme: the row carries its name, the cards a notice (867, 870)")
    @MainActor func settingsProjectRows() async throws {
        try await Instance.with(auth: "allow") { app in
            let repo = try app.scratch.repository("sets-project")
            // A project's theme is its own `theme.json`; `theme` is not a setting a project's config.json may change (core
            // `PROJECT_SETTINGS`), so the picks named there are dropped.
            try app.scratch.project(repo, files: [
                "config.json": #"{"font_size": 19}"#,
                "theme.json": ##"{"name": "Sets", "extends": "Nord", "ui": {"primary": "#123456"}}"##,
            ])
            await app.flags(["project_folders": true])
            await app.terminal(in: repo)
            await app.send(["tab project"])
            await app.until("project-state") { $0.contains("root=") }
            await app.send(["project-approve"])
            await app.until("project-state") { $0.contains("status=trusted") }
            let page = await Self.page(app, "fonts") { $0.line("row", "font-terminal")?["project"] != nil }
            app.expect(page.line("row", "font-terminal")?["project"] == "sets-project", "the project's name on the row:\n\(page.text)")
            let cards = Readout(await app.until("theme-cards") { Readout($0).first("project") != nil })
            app.expect(cards.first("project")?.label == "sets-project", "the notice names the project:\n\(cards.text)")
        }
    }
}
