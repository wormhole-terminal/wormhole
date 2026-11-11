@testable import DesignSystem
import Foundation
import UniFFISource
import XCTest

/// The store against `wormhole-core`'s real loader and token rules, through the FFI.
@MainActor
final class RealResolverTests: XCTestCase {
    private var folder: URL!

    override func setUpWithError() throws {
        folder = FileManager.default.temporaryDirectory.appendingPathComponent("wh-themes-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        let mine = "// my theme, with a comment\n{ \"name\": \"Mine\", \"extends\": \"Nord\", \"ui\": { \"primary\": \"#ff6600\" }, }\n"
        try mine.write(to: folder.appendingPathComponent("Mine.json"), atomically: true, encoding: .utf8)
    }

    override func tearDownWithError() throws { try? FileManager.default.removeItem(at: folder) }

    private func store() -> ThemeStore {
        let store = ThemeStore(follows: false)
        store.systemAppearance = .dark
        store.resolver = FfiThemeResolver()
        store.themesDirectory = folder
        return store
    }

    func testAUserThemeFileResolvesAndIsListed() throws {
        let store = store()
        let mine = try XCTUnwrap(store.userThemes.first { $0.name == "Mine" })
        XCTAssertEqual(mine.appearance, .dark)
        store.show(mine.id)
        XCTAssertEqual(store.theme.name, "Mine")
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#ff6600")
        XCTAssertEqual(store.tokens.terminal.background.hex, "#2e3440", "it extends Nord")
        XCTAssertNil(store.problem)
    }

    func testAPrimaryPickRederivesWhatFollowsIt() {
        let store = store()
        let before = store.tokens.rgba(named: "border.focus")
        store.settings.primary = "#00ff88"
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#00ff88")
        XCTAssertEqual(store.tokens.rgba(named: "border.focus")?.hex, "#00ff88")
        XCTAssertNotEqual(store.tokens.rgba(named: "border.focus"), before)
        store.settings.primary = ""
        XCTAssertEqual(store.tokens.rgba(named: "primary")?.hex, "#0096ff")
    }

    func testABranchPickFollowsThePrimary() {
        let store = store()
        let own = store.tokens.rgba(named: "git.branch")
        store.settings.branch = .primary
        XCTAssertNotEqual(store.tokens.rgba(named: "git.branch"), own)
    }

    func testAnOpacityOtherThanTheDefaultIsJudgedByRust() throws {
        let store = store()
        store.settings.look = WindowLook(opacity: 0.5, blur: true)
        XCTAssertEqual(try XCTUnwrap(store.tokens.rgba(named: "surface.window")).alpha, 0.5, accuracy: 0.01)
        let half = store.tokens.rgba(named: "text.muted")
        store.settings.look = WindowLook(opacity: 1, blur: true)
        XCTAssertNotEqual(store.tokens.rgba(named: "text.muted"), half, "an opaque window keeps the opaque rules' muted text")
    }

    func testAMissingThemeFallsBackAndSaysWhy() {
        let store = store()
        store.settings.darkTheme = "no such theme"
        XCTAssertNotNil(store.problem)
        XCTAssertEqual(store.theme.id, "wormhole.dark")
    }
}
