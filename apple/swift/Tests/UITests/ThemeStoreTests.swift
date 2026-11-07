import DesignSystem
import SwiftUI
import XCTest

@MainActor
final class ThemeStoreTests: XCTestCase {
    private func store() -> ThemeStore {
        let store = ThemeStore(follows: false)
        store.systemAppearance = .dark
        return store
    }

    func testMatchSystemShowsTheThemeForTheSystemsAppearance() {
        let store = store()
        XCTAssertEqual(store.theme.id, "wormhole.dark")
        store.systemAppearance = .light
        XCTAssertEqual(store.theme.id, "wormhole.light")
        store.mode = .dark
        XCTAssertEqual(store.theme.id, "wormhole.dark", "a fixed mode ignores macOS")
    }

    func testSetThemeIsThePickForItsAppearanceAndShowsIt() {
        let store = store()
        store.setTheme("nord")
        XCTAssertEqual(store.theme.id, "nord")
        store.setTheme("github_light")
        XCTAssertEqual(store.theme.id, "github_light", "the light theme moves the mode so it is seen")
        XCTAssertEqual(store.mode, .light)
        store.mode = .dark
        XCTAssertEqual(store.theme.id, "nord", "the dark pick is kept")
        store.setTheme("no_such_theme")
        XCTAssertEqual(store.theme.id, "nord")
    }

    func testIncreaseContrastSelectsTheHighContrastTokens() {
        let store = store()
        let normal = store.tokens.rgba(named: "border")
        store.increaseContrast = true
        XCTAssertNotEqual(store.tokens.rgba(named: "border"), normal)
        XCTAssertEqual(store.tokens.metrics.focusRingWidth, 3)
    }

    /// The point of the whole system: one open view tree recolours when the theme changes, and no view
    /// is rebuilt or touched. Both the environment's tokens and `Palette` (what older views read) follow.
    func testSwitchingThemeRecoloursALiveViewTree() throws {
        let store = ThemeStore.shared
        let saved = (store.mode, store.darkThemeID, store.lightThemeID, store.systemAppearance)
        defer { (store.mode, store.systemAppearance) = (saved.0, saved.3); store.setTheme(saved.1); store.setTheme(saved.2); store.mode = saved.0 }
        store.mode = .dark
        store.setTheme("wormhole.dark")
        let render = ThemeRender(SampleView().themed(store), size: CGSize(width: 40, height: 20), dark: true)
        func shown() throws -> (palette: NSColor, environment: NSColor) {
            (try render.pixel(x: 5, y: 10), try render.pixel(x: 30, y: 10))
        }
        // The bitmap is Display P3, so a token is compared with the same colour drawn plainly through
        // the same pipeline rather than with its hex.
        func plain(_ name: String) throws -> NSColor {
            let color = try XCTUnwrap(store.tokens.rgba(named: name)).color
            return try ThemeRender(color, size: CGSize(width: 4, height: 4), dark: true).pixel(x: 2, y: 2)
        }

        let dark = try shown()
        assertEqual(dark.palette, try plain("sidebar.background"))
        assertEqual(dark.environment, try plain("primary"))

        store.setTheme("nord")
        let nord = try shown()
        assertEqual(nord.palette, try plain("sidebar.background"))
        assertEqual(nord.environment, try plain("primary"))
        XCTAssertGreaterThan(distance(dark.palette, nord.palette), 0.02, "the surface changed")
        XCTAssertGreaterThan(distance(dark.environment, nord.environment), 0.1, "the primary changed")
    }

    private func distance(_ a: NSColor, _ b: NSColor) -> Double {
        abs(a.redComponent - b.redComponent) + abs(a.greenComponent - b.greenComponent) + abs(a.blueComponent - b.blueComponent)
    }

    private func assertEqual(_ a: NSColor, _ b: NSColor, line: UInt = #line) {
        XCTAssertLessThan(distance(a, b), 0.02, "\(a) vs \(b)", line: line)
    }
}

/// Left half through `Palette`, right half through the environment.
private struct SampleView: View {
    @Environment(\.tokens) private var tokens
    var body: some View {
        HStack(spacing: 0) {
            Palette.sidebarBackground.frame(width: 20)
            tokens.primary
        }
    }
}
