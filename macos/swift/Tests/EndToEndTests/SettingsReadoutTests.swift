import Foundation
import Testing

/// What a Settings page says beside its controls, read with `settings-page` (`Readout`'s shape): the banners over a page, a
/// row's caption, the notes and swatches under it, a control drawn off, a shortcut's problem, a menu's subtitles. The
/// wave-8 lists' "waiting for `settings-page`" lines, by their TO-VERIFY.md numbers.
extension EndToEnd {
    /// The page `section` as `settings-page` reads it once Settings shows it.
    @MainActor static func page(_ app: Instance, _ section: String, until done: (Readout) -> Bool = { _ in true }) async -> Readout {
        await app.send(["settings-section \(section)"])
        return Readout(await app.until("settings-page") { Readout($0).first("settings-page")?.words.first == section && done(Readout($0)) })
    }

    @Test("an unreadable config.json: Advanced says why with Open, every other page that changes are not saved (127)")
    @MainActor func settingsUnreadableConfig() async throws {
        try await Instance.with(config: #"{ "blur": "#) { app in
            let advanced = await Self.page(app, "advanced") { !$0.all("banner").isEmpty }
            app.expect(advanced.all("banner").contains { $0.words.first == "warning" }, "a warning on Advanced:\n\(advanced.text)")
            let appearance = await Self.page(app, "appearance") { !$0.all("banner").isEmpty }
            let warning = appearance.all("banner").first { $0.words.first == "warning" }
            app.expect(warning != nil, "Appearance says changes are not saved:\n\(appearance.text)")
            app.expect(warning?.label != advanced.all("banner").first?.label, "the two pages say different things")
        }
    }

    @Test("a corrupt profiles.json is set aside and rebuilt, and the Profile page says so (128)")
    @MainActor func settingsProfilesRebuilt() async throws {
        try await Instance.with { app in
            await app.quit(command: "quit")
            try Data("{ not json".utf8).write(to: app.scratch.data.appendingPathComponent("profiles.json"))
            try await app.start()
            let profile = await Self.page(app, "profile") { !$0.all("banner").isEmpty }
            app.expect(profile.all("banner").contains { $0.words.first == "warning" }, "the Profile page's banner:\n\(profile.text)")
        }
    }

    @Test("a hotkey macOS keeps for itself, set by hand, shows its problem under the recorder (148)")
    @MainActor func settingsHotkeyProblem() async throws {
        try await Instance.with(config: #"{"hotkey": "cmd-space"}"#) { app in
            let page = await Self.page(app, "hotkey") { $0.line("row", "hotkey") != nil }
            let problem = page.under("row", "hotkey").first { $0.kind == "problem" }
            app.expect(problem != nil && !(problem?.label.isEmpty ?? true), "the hotkey row says why ⌘Space does not work:\n\(page.text)")
        }
    }

    @Test("Zen mode puts its note on Appearance and Terminal, not on Fonts (445)")
    @MainActor func settingsZenNote() async throws {
        try await Instance.with { app in
            app.expect(await app.reply("zen on"), contains: "zen on")
            for section in ["appearance", "terminal"] {
                let page = await Self.page(app, section) { !$0.all("banner").isEmpty }
                app.expect(page.all("banner").contains { $0.label.contains("Zen") }, "\(section) has the Zen note:\n\(page.text)")
            }
            let fonts = await Self.page(app, "fonts")
            app.expect(!fonts.all("banner").contains { $0.label.contains("Zen") }, "Fonts has no Zen note:\n\(fonts.text)")
            await app.send(["zen off"])
            await app.until("settings-page") { !Readout($0).all("banner").contains { $0.label.contains("Zen") } }
        }
    }

    @Test("Notifications in a test instance: the permission row says Outside the app bundle with its hint (845)")
    @MainActor func settingsNotifyOutsideBundle() async throws {
        try await Instance.with { app in
            await app.send(["notify-permission outside"])
            // The row asks the notification centre when the page shows ("Asking macOS…") and has its answer a moment later.
            let page = await Self.page(app, "notifications") { page in
                page.line("row", "notify-permission").map { !$0.label.hasPrefix("Asking") } ?? false
            }
            let row = page.line("row", "notify-permission")
            app.expect(row?.label == "Outside the app bundle", "the row's label:\n\(page.text)")
            app.expect(page.under("row", "notify-permission").contains { $0.kind == "caption" }, "its hint under it:\n\(page.text)")
        }
    }

    @Test("Branch colour Custom: swatches and a well, \"Pick a colour below\" until one is picked (851)")
    @MainActor func settingsBranchSwatches() async throws {
        try await Instance.with { app in
            await app.send(["set branch_color=\"custom\""])
            let page = await Self.page(app, "appearance") { $0.under("row", "branch-color").contains { $0.kind == "swatches" } }
            let under = page.under("row", "branch-color")
            let swatches = under.first { $0.kind == "swatches" }
            app.expect(swatches?.has("well") == true && (swatches?.words.count ?? 0) > 1, "swatches and a well:\n\(page.text)")
            app.expect(under.first { $0.kind == "caption" }?.label.hasPrefix("Pick a colour below") == true, "the caption asks for a pick:\n\(page.text)")
            guard let first = swatches?.words.first(where: { $0.hasPrefix("#") }) else { return }
            await app.send(["set branch_color_custom=\"\(first)\""])
            let picked = Readout(await app.until("settings-page") { Readout($0).under("row", "branch-color").first { $0.kind == "swatches" }?["chosen"] == first })
            app.expect(!picked.under("row", "branch-color").contains { $0.kind == "caption" && $0.label.hasPrefix("Pick a colour below") }, "picked: the caption goes")
            app.expect(picked.line("row", "branch-color")?["value"] == "Custom", "Custom stays chosen")
        }
    }

    @Test("blur off at 80 %: a warning under Blur with Turn on blur (852)")
    @MainActor func settingsBlurWarning() async throws {
        try await Instance.with { app in
            await app.send(["set blur=false", "set opacity=0.8"])
            let page = await Self.page(app, "appearance") { $0.under("row", "blur").contains { $0.kind == "note" } }
            let note = page.under("row", "blur").first { $0.kind == "note" }
            app.expect(note?.words.first == "warning" && note?["button"] == "Turn on blur", "the blur warning:\n\(page.text)")
            await app.send(["set blur=true"])
            await app.until("settings-page") { !Readout($0).under("row", "blur").contains { $0.kind == "note" } }
        }
    }

    @Test("Reduce Transparency: the opacity slider is off with its note; Liquid Glass wears Beta (853, 1143)")
    @MainActor func settingsOpaqueSlider() async throws {
        try await Instance.with { app in
            let before = await Self.page(app, "appearance") { $0.line("row", "opacity") != nil }
            app.expect(before.line("row", "opacity")?.has("off") == false, "the slider is on while the system is not opaque")
            if let glass = before.line("row", "liquid-glass") { app.expect(glass["chip"] == "Beta", "Liquid Glass wears Beta: \(glass.text)") }
            await app.send(["reduce-transparency on"])
            let page = Readout(await app.until("settings-page") { Readout($0).line("row", "opacity")?.has("off") == true })
            app.expect(page.under("row", "opacity").contains { $0.kind == "note" && $0.label.contains("Reduce Transparency") }, "the note says why:\n\(page.text)")
            await app.send(["reduce-transparency off"])
            await app.until("settings-page") { Readout($0).line("row", "opacity")?.has("off") == false }
        }
    }

    @Test("Primary colour, the theme's own off: Pick a colour below with swatches, then Yours · #hex (854)")
    @MainActor func settingsPrimaryCaption() async throws {
        try await Instance.with { app in
            await app.send(["set theme_primary=false", "set primary_color=\"\""])
            let page = await Self.page(app, "appearance") { $0.under("row", "primary-color").contains { $0.kind == "swatches" } }
            app.expect(page.under("row", "primary-color").first { $0.kind == "caption" }?.label.hasPrefix("Pick a colour below") == true, "asks for a pick:\n\(page.text)")
            await app.send(["set primary_color=\"#ff8800\""])
            let yours = Readout(await app.until("settings-page") { Readout($0).under("row", "primary-color").contains { $0.kind == "caption" && $0.label.hasPrefix("Yours") } })
            app.expect(yours.under("row", "primary-color").first { $0.kind == "caption" }?.label.contains("#ff8800") == true, "Yours · #ff8800:\n\(yours.text)")
        }
    }

    @Test("the terminal font's summary and the custom fonts found are captions (855)")
    @MainActor func settingsFontCaptions() async throws {
        try await Instance.with { app in
            let terminal = await Self.page(app, "terminal") { $0.line("row", "terminal-font") != nil }
            let summary = terminal.under("row", "terminal-font").first { $0.kind == "caption" }?.label ?? ""
            app.expect(summary.contains(" pt · line height ") && summary.contains("ligatures"), "the font's summary: \(summary)")
            let fonts = await Self.page(app, "fonts") { $0.line("row", "custom-fonts") != nil }
            let found = fonts.under("row", "custom-fonts").first { $0.kind == "caption" }?.label ?? ""
            app.expect(found.hasPrefix("None yet") || found.contains("found:"), "the custom fonts' caption: \(found)")
        }
    }

    @Test("values set by hand: 75,000 lines, a missing shell profile and a removed app say what happens instead (856, 1144)")
    @MainActor func settingsHandWrittenValues() async throws {
        let config = #"{"scrollback_lines": 75000, "default_shell": "no-such-shell-profile", "open_in": "com.example.removed-app"}"#
        try await Instance.with(config: config) { app in
            let page = await Self.page(app, "terminal") { $0.line("row", "scrollback") != nil }
            app.expect(page.line("row", "scrollback")?["value"] == "75,000 lines", "the stepper's words:\n\(page.text)")
            app.expect(page.under("row", "default-shell").contains { $0.kind == "caption" && $0.label.hasPrefix("Not found") }, "the shell row:\n\(page.text)")
            app.expect(page.under("row", "open-in").contains { $0.kind == "caption" && $0.label.hasPrefix("Not installed") }, "the app row:\n\(page.text)")
            // Each shell choice carries its command line under its name in the menu.
            let options = page.under("row", "default-shell").filter { $0.kind == "option" }
            app.expect(!options.isEmpty && options.allSatisfy { $0.quoted.count == 2 && !$0.quoted[1].isEmpty }, "the shell menu's subtitles:\n\(page.text)")
        }
    }

    @Test("a section picked during a search keeps its matches marked (865)")
    @MainActor func settingsSearchMarks() async throws {
        try await Instance.with { app in
            await app.send(["settings-search ligatures"])
            let page = await Self.page(app, "fonts") { $0.lines.contains { $0.kind == "row" && $0.has("found") } }
            app.expect(page.line("row", "font-ligatures")?.has("found") == true, "the match is marked:\n\(page.text)")
            app.expect(page.line("nav", "fonts")?["badge"] != nil, "the nav counts the page's results:\n\(page.text)")
        }
    }
}
