@testable import DesignSystem
import Foundation
import XCTest

/// Reduce Transparency, the colour picks and user themes through a resolver, persistence and the interface font.
@MainActor
final class ThemeLookTests: XCTestCase {
    private func store(storage: ThemeSettingsStorage? = nil) -> ThemeStore {
        let store = ThemeStore(library: .bundled, follows: false, storage: storage)
        store.systemAppearance = .dark
        return store
    }

    func testTranslucentWindowsDrawTextJudgedOnWhatShowsThrough() throws {
        // The numbers of core's own design test (`tokyo_night_at_80_percent_matches_the_design`, 1 step per channel).
        let store = store()
        store.setTheme("tokyo_night")
        for (token, hex) in [("sidebar.row.selected.muted", "#a3abcc"), ("text.faint", "#7c819b"), ("git.branch", "#c0a9f7")] {
            let got = try XCTUnwrap(store.tokens.rgba(named: token))
            let want = try XCTUnwrap(ThemeStoreFixtures.rgba(hex: hex))
            XCTAssertLessThan(abs(got.red - want.red) + abs(got.green - want.green) + abs(got.blue - want.blue), 0.02, token)
        }
        XCTAssertEqual(try XCTUnwrap(store.tokens.rgba(named: "surface.window")).alpha, 0.8, accuracy: 0.001)
    }

    func testReduceTransparencyMakesTheWindowOpaqueWithTheOpaqueRulesText() throws {
        let store = store()
        store.setTheme("tokyo_night")
        let translucent = store.tokens
        store.reduceTransparency = true
        XCTAssertEqual(try XCTUnwrap(store.tokens.rgba(named: "surface.window")).alpha, 1)
        XCTAssertEqual(store.tokens.rgba(named: "text.muted"), store.theme.tokens().rgba(named: "text.muted"))
        XCTAssertNotEqual(translucent.rgba(named: "text.muted"), store.tokens.rgba(named: "text.muted"))
        store.reduceTransparency = false
        XCTAssertEqual(store.tokens, translucent)
    }

    func testIncreaseContrastWindowsAreOpaqueToo() throws {
        let store = store()
        store.increaseContrast = true
        XCTAssertEqual(try XCTUnwrap(store.tokens.rgba(named: "surface.window")).alpha, 1)
        XCTAssertEqual(store.tokens.metrics.focusRingWidth, 3)
    }

    /// A resolver that answers from the dump's own JSON, to test the store's side of the seam without Rust.
    private struct DumpResolver: ThemeResolving {
        let requests = ThemeStoreFixtures.Log()
        func resolve(_ request: ThemeRequest) throws -> Data {
            requests.add(request)
            guard request.theme == "nord" || request.theme == "mine" else {
                throw NSError(domain: "core", code: 1, userInfo: [NSLocalizedDescriptionKey: "unknown theme"])
            }
            return try ThemeStoreFixtures.resolved(id: "nord")
        }
        func userThemes(in directory: URL) -> Data {
            Data(#"[{"id": "mine", "name": "Mine", "appearance": "dark", "error": null}]"#.utf8)
        }
    }

    func testAUserThemeAndPicksGoThroughTheResolver() throws {
        let store = store()
        let resolver = DumpResolver()
        store.resolver = resolver
        store.themesDirectory = URL(fileURLWithPath: "/themes")
        XCTAssertEqual(store.userThemes.map(\.id), ["mine"])
        store.setTheme("mine")
        let last = try XCTUnwrap(resolver.requests.last)
        XCTAssertEqual([last.theme, last.themesDirectory?.path], ["mine", "/themes"])
        XCTAssertEqual(store.tokens.terminal.background.hex, "#2e3440")
        store.settings.primary = "#ff6600"
        store.settings.branch = .custom("#00ff00")
        let picks = try XCTUnwrap(resolver.requests.last)
        XCTAssertEqual([picks.primary, picks.branch], ["#ff6600", "#00ff00"])
    }

    func testWithoutAResolverAUserThemeIsReportedNotHidden() {
        let store = store()
        store.settings.darkTheme = "mine"
        XCTAssertNotNil(store.problem)
        XCTAssertEqual(store.theme.id, "wormhole.dark")
    }

    func testAResolverFailureFallsBackToTheDefaultAndSaysWhy() {
        let store = store()
        store.resolver = DumpResolver()
        store.settings.darkTheme = "gone"
        XCTAssertEqual(store.problem?.contains("unknown theme"), true)
        XCTAssertEqual(store.theme.id, "wormhole.dark")
    }

    func testTheChoiceModeAndPicksArePersistedInTheIsolatedSuite() throws {
        let name = "wh-theme-test-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let first = store(storage: ThemeSettingsStorage(defaults: defaults))
        first.setTheme("nord")
        first.settings.primary = "#123456"
        first.settings.branch = .primary
        first.settings.look = WindowLook(opacity: 0.6, blur: false)
        let second = store(storage: ThemeSettingsStorage(defaults: defaults))
        XCTAssertEqual(second.settings, first.settings)
        XCTAssertEqual(second.darkThemeID, "nord")
        // Foundation refuses a suite named like the running bundle (it logs and persists nothing), and the real
        // app's preferences are the bundle id `app.getwormhole.terminal`.
        for bundle in [Bundle.main.bundleIdentifier, "app.getwormhole.terminal.swift", "app.getwormhole.terminal"] {
            XCTAssertNotEqual(ThemeSettingsStorage.suite, bundle)
        }
        XCTAssertNotNil(UserDefaults(suiteName: ThemeSettingsStorage.suite))
    }

    func testTheInterfaceFontSizeScalesTheTypeRoles() {
        XCTAssertEqual(TextRole.allCases.map { $0.rems * 13 * 16 / 13 }, [12, 13, 14, 16, 20])
        XCTAssertEqual(ThemeStore.shared.tokens.metrics.uiFontSize, 13)
        XCTAssertNil(ThemeStore.shared.tokens.metrics.uiFontFamily)
    }
}
