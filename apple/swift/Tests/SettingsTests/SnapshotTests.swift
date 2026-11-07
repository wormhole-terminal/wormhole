import DesignSystem
import SettingsModel
@testable import SettingsUI
import SwiftUI
import XCTest

/// Offscreen renders of the pages, looked at by eye against the GPUI screenshots (PNGs under SETTINGS_SNAPSHOTS).
@MainActor
final class SnapshotTests: XCTestCase {
    /// The page as the settings of `themeID` show it: the stored mode and pick are that theme's.
    func render(
        _ section: String, theme id: String, _ overrides: [String: JSONValue] = [:], query: String = "",
        contrast: Bool = false, name: String? = nil, records: PanelRecords = .sample,
        size: CGSize = CGSize(width: 1100, height: 760)
    ) throws {
        let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: id))
        let dark = theme.appearance == .dark
        var values = overrides
        values["theme_mode"] = dark ? "dark" : "light"
        values[dark ? "theme" : "theme_light"] = .string(theme.name)
        let model = SettingsModel(
            backend: MockSettingsBackend(overrides: values), host: MockSettingsHost(records: records), selection: section)
        model.query = query
        try Snapshot.write(SettingsView(model: model), name: name ?? section, theme: id, contrast: contrast, size: size)
    }

    func testPagesInDarkAndLight() throws {
        let pages = ["appearance", "terminal", "fonts", "sessions", "notifications", "git", "agents", "menuBar", "hotkey", "updates", "advanced", "profile", "import", "changed"]
        for page in pages {
            for theme in ["wormhole.dark", "wormhole.light"] { try render(page, theme: theme) }
        }
    }

    func testOtherThemesAndContrast() throws {
        for theme in ["nord", "dracula", "github_light"] { try render("appearance", theme: theme) }
        try render("sessions", theme: "wormhole.dark", contrast: true)
    }

    /// The pages that read from the disk: the `wh` row with its notes, the approvals, a project's marked row, a problem.
    func testTheLivePages() throws {
        for theme in ["wormhole.dark", "wormhole.light"] {
            let tall = CGSize(width: 1100, height: 2400)
            try render("agents", theme: theme, name: "live-agents", records: .live, size: tall)
            try render("terminal", theme: theme, name: "live-terminal", records: .live)
            try render("fonts", theme: theme, name: "live-fonts", records: .live)
            try render("profile", theme: theme, name: "live-profile", records: .live)
            let model = SettingsModel(backend: MockSettingsBackend(), host: MockSettingsHost(records: .live), selection: "profile")
            try Snapshot.write(
                NewProfileSheet(model: model), name: "live-new-profile", theme: theme, size: CGSize(width: 420, height: 300))
        }
    }

    func testChangedAndSearch() throws {
        try render("changed", theme: "wormhole.dark", ["blur": false, "density": "dense", "opacity": 0.5, "font_size": 16], name: "changed-some")
        try render("appearance", theme: "wormhole.dark", query: "font", name: "search-font")
        try render("appearance", theme: "wormhole.dark", query: "zzzz", name: "search-none")
    }
}
