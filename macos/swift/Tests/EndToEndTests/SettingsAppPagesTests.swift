import Foundation
import Testing

/// Settings' App and Preview pages (Notifications, Menu bar, Feature flags) changed by clicks on their controls
/// (`settings-press`, `at <x> [y]` along a segmented control), read in `config.json` and where they act: what the core's rules
/// decide for a terminal's event with the settings the notifier holds (`notify-sim`, the same `settingsNotice` the real feed
/// uses), what the menu bar extra is built from, which pages the nav offers.
extension EndToEnd {
    @Test("Notifications, clicked: no sound, then no Dock bounce, then Needs input off: what the core decides for a terminal needing input follows each")
    @MainActor func notificationsPageClicked() async throws {
        try await Instance.with { app in
            // A background instance is never the active app: a terminal needing input is a banner with a sound and a bounce
            // (core `notify/mod.rs:163, 174, 255`).
            let before = await app.reply("notify-sim needs-you 1")
            app.expect(before, contains: "-> Banner")
            app.expect(before, contains: "sound=true")
            app.expect(before, contains: "bounce=true")

            // Sound → None (core `settings_index/table/notifications.rs:58`).
            await app.reveal("notify-sound")
            await app.pressAlong("notify-sound", at: [(0.9, 0.5), (0.85, 0.5), (0.95, 0.5), (0.8, 0.5)]) { await app.setting("notify_sound") == #"string("none")"# }
            await app.until("notify-sim needs-you 1") { $0.contains("-> Banner") && $0.contains("sound=false") }
            _ = await app.waitForConfig { $0.range(of: #""notify_sound"\s*:\s*"none""#, options: .regularExpression) != nil }

            // Bounce in the Dock off (`notifications.rs:75`).
            await app.reveal("dock-bounce")
            _ = await Self.press(app, "dock-bounce")
            await app.until("notify-sim needs-you 1") { $0.contains("bounce=false") }
            _ = await app.waitForConfig { $0.range(of: #""dock_bounce"\s*:\s*false"#, options: .regularExpression) != nil }

            // Needs input off (`notifications.rs:24`): nothing is said at all.
            await app.reveal("notify-needs-you")
            _ = await Self.press(app, "notify-needs-you")
            await app.until("notify-sim needs-you 1") { $0.contains("-> Nothing") }
            _ = await app.waitForConfig { $0.range(of: #""notify_needs_you"\s*:\s*false"#, options: .regularExpression) != nil }
        }
    }

    @Test("Menu bar, clicked: the item off and its icon to Icon only are written; the icon the item is built from drops the count and the dot")
    @MainActor func menuBarPageClicked() async throws {
        try await Instance.with(shell: .agent("codex")) { app in
            // One terminal needing input: Status and count shows its dot and a 1 (GPUI `status_item/mod.rs:156`).
            try await app.report(agent: "codex", event: "PermissionRequest", terminal: 1, becomes: "needsInput")
            let counted = await app.until("ambient") { $0.contains("count: Optional(\"1\")") }
            app.expect(counted, contains: "icon ")

            await app.reveal("menu-bar")
            _ = await Self.press(app, "menu-bar")
            _ = await app.waitForConfig { $0.range(of: #""menu_bar"\s*:\s*false"#, options: .regularExpression) != nil }

            // Icon only (core `settings_index/table/menu_bar.rs:15`): the third segment.
            await app.reveal("menu-bar-icon")
            await app.pressAlong("menu-bar-icon", at: [(0.88, 0.5), (0.82, 0.5), (0.94, 0.5), (0.76, 0.5)]) { await app.setting("menu_bar_icon") == #"string("plain")"# }
            let plain = await app.until("ambient") { $0.contains("count: nil") }
            app.expect(plain, contains: "StatusMark.none")
            _ = await app.waitForConfig { $0.range(of: #""menu_bar_icon"\s*:\s*"plain""#, options: .regularExpression) != nil }
        }
    }

    @Test("Feature flags, clicked: Project folders switched on the page is written to the flags, and its page comes and goes in the nav")
    @MainActor func featureFlagsPageClicked() async throws {
        try await Instance.with { app in
            var unlocked = await app.reply("flags-unlock")
            if unlocked.contains("unlocked=false") { unlocked = await app.reply("flags-unlock") }
            app.expect(unlocked, contains: "featureFlags")
            let page = Readout(await app.until("settings-page") { Readout($0).first("settings-page")?.words.first == "featureFlags" })
            let had = page.line("nav", "projects") != nil

            // The flag's switch (GPUI `settings_window/sections/feature_flags.rs:5`: a flag on shows its feature's own section).
            await app.settledRow("flag.project_folders")
            _ = await Self.press(app, "flag.project_folders")
            _ = await app.waitForConfig { $0.range(of: "\"project_folders\"\\s*:\\s*\(!had)", options: .regularExpression) != nil }
            await app.until("settings-page") { (Readout($0).line("nav", "projects") != nil) == !had }
            _ = await Self.press(app, "flag.project_folders")
            await app.until("settings-page") { (Readout($0).line("nav", "projects") != nil) == had }
        }
    }
}
