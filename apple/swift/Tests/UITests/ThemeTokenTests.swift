import DesignSystem
import Foundation
import XCTest

/// Spot values come from the Rust token resolution (`themes.json`, written by `dump_themes`) and from the
/// theme documentation (`repo-docs/docs/themes.md`: borders 12 % / light 14 %, Increase Contrast 32 % / 40 %,
/// the focus ring 2 -> 3), so the loader and the accessors are checked against numbers written independently.
final class ThemeTokenTests: XCTestCase {
    let library = ThemeLibrary.bundled

    private func tokens(_ id: String, contrast: Bool = false) throws -> ThemeTokens {
        try XCTUnwrap(library.theme(id: id), id).tokens(increaseContrast: contrast)
    }

    func testDefaultThemesAreTheAppsDefaults() {
        XCTAssertEqual(library.defaultDark, "wormhole.dark")
        XCTAssertEqual(library.defaultLight, "wormhole.light")
        XCTAssertEqual(library.theme(id: "wormhole.dark")?.appearance, .dark)
        XCTAssertEqual(library.theme(id: "wormhole.light")?.appearance, .light)
    }

    func testDefaultDarkMatchesTheRustDump() throws {
        let t = try tokens("wormhole.dark")
        XCTAssertEqual(t.terminal.background.hex, "#08131e")
        XCTAssertEqual(t.terminal.foreground.hex, "#e3ebf2")
        XCTAssertEqual(t.sidebarBackground, t.color(named: "sidebar.background"))
        XCTAssertEqual(t.rgba(named: "sidebar.background")?.hex, "#071019")
        XCTAssertEqual(t.rgba(named: "sidebar.row.selected")?.hex, "#062239")
        XCTAssertEqual(t.rgba(named: "primary")?.hex, "#0096ff")
        XCTAssertEqual(try XCTUnwrap(t.rgba(named: "border")).alpha, 0.12, accuracy: 0.001)
    }

    func testDefaultLightMatchesTheRustDump() throws {
        let t = try tokens("wormhole.light")
        XCTAssertEqual(t.terminal.background.hex, "#f7f9fb")
        XCTAssertEqual(t.rgba(named: "sidebar.row.selected")?.hex, "#ccddea")
        XCTAssertEqual(t.rgba(named: "text.muted")?.hex, "#5e6770")
        XCTAssertEqual(try XCTUnwrap(t.rgba(named: "border")).alpha, 0.14, accuracy: 0.001)
        XCTAssertEqual(t.rgba(named: "settings.group")?.hex, "#ffffffcc")
    }

    func testImportedPalettesKeepTheirOwnColours() throws {
        let nord = try tokens("nord")
        XCTAssertEqual(nord.terminal.background.hex, "#2e3440")
        XCTAssertEqual(nord.terminal.foreground.hex, "#d8dee9")
        XCTAssertEqual(nord.terminal.ansi.count, 16)
    }

    func testIncreaseContrastDiffersWhereRustSaysSo() throws {
        for id in ["wormhole.dark", "wormhole.light"] {
            let normal = try tokens(id), high = try tokens(id, contrast: true)
            let light = id.hasSuffix("light")
            XCTAssertEqual(try XCTUnwrap(normal.rgba(named: "border")).alpha, light ? 0.14 : 0.12, accuracy: 0.001, id)
            XCTAssertEqual(try XCTUnwrap(high.rgba(named: "border")).alpha, light ? 0.40 : 0.32, accuracy: 0.001, id)
            XCTAssertEqual(try XCTUnwrap(high.rgba(named: "state.hover")).alpha, 0.10, accuracy: 0.001, id)
            XCTAssertNotEqual(normal.rgba(named: "sidebar.row.selected"), high.rgba(named: "sidebar.row.selected"), id)
            XCTAssertEqual(normal.metrics.focusRingWidth, 2, id)
            XCTAssertEqual(high.metrics.focusRingWidth, 3, id)
            // What Rust leaves alone stays: the surfaces and the primary.
            XCTAssertEqual(normal.rgba(named: "sidebar.background"), high.rgba(named: "sidebar.background"), id)
            XCTAssertEqual(normal.rgba(named: "primary"), high.rgba(named: "primary"), id)
        }
        XCTAssertEqual(
            try tokens("wormhole.light", contrast: true).rgba(named: "text.muted")?.hex, "#485159",
            "light themes lift muted text to 7:1 on the sidebar")
    }

    func testEveryThemeHasEveryCatalogToken() {
        let ui = library.catalog.filter { !$0.name.hasPrefix("metrics.") }
        // 112 since core's catalog gained tooltip.success and tooltip.danger (status text on the tooltip fill).
        XCTAssertEqual(ui.count, 112)
        XCTAssertGreaterThan(library.themes.count, 50)
        for theme in library.themes {
            for contrast in [false, true] {
                let tokens = theme.tokens(increaseContrast: contrast)
                for token in ui {
                    XCTAssertNotNil(tokens.rgba(named: token.name), "\(theme.id) lacks \(token.name)")
                }
            }
        }
    }

    func testAMissingTokenIsMagenta() throws {
        let t = try tokens("nord")
        XCTAssertNil(t.rgba(named: "no.such.token"))
        XCTAssertEqual(ThemeTokens.missing.hex, "#ff00ff")
    }

    /// The generated accessors cover the catalog: each token's name is read by exactly the accessor named for it.
    func testAccessorsCoverTheCatalog() throws {
        let theme = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
            .appendingPathComponent("../../Sources/DesignSystem/Theme").standardized
        let generated = try FileManager.default.contentsOfDirectory(atPath: theme.path)
            .filter { $0.hasPrefix("Tokens") }
            .map { try String(contentsOf: theme.appendingPathComponent($0), encoding: .utf8) }
            .joined()
        for token in library.catalog where !token.name.hasPrefix("metrics.") {
            XCTAssertTrue(generated.contains("color(named: \"\(token.name)\")"), "no accessor for \(token.name)")
        }
    }

    func testMetricsAreTheThemes() throws {
        let m = try tokens("wormhole.dark").metrics
        XCTAssertEqual([m.radiusSm, m.radiusChip, m.radius, m.radiusLg], [4, 4, 6, 10])
        XCTAssertEqual([m.borderWidth, m.dividerWidth], [1, 1])
    }
}
