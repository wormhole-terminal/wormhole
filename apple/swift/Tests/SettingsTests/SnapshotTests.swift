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
        contrast: Bool = false, name: String? = nil
    ) throws {
        let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: id))
        let dark = theme.appearance == .dark
        var values = overrides
        values["theme_mode"] = dark ? "dark" : "light"
        values[dark ? "theme" : "theme_light"] = .string(theme.name)
        let model = SettingsModel(backend: MockSettingsBackend(overrides: values), host: MockSettingsHost(), selection: section)
        model.query = query
        try Snapshot.write(SettingsView(model: model), name: name ?? section, theme: id, contrast: contrast)
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

    func testChangedAndSearch() throws {
        try render("changed", theme: "wormhole.dark", ["blur": false, "density": "dense", "opacity": 0.5, "font_size": 16], name: "changed-some")
        try render("appearance", theme: "wormhole.dark", query: "font", name: "search-font")
        try render("appearance", theme: "wormhole.dark", query: "zzzz", name: "search-none")
    }
}
