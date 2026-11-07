import AppKit
import DesignSystem
import SettingsModel
@testable import SettingsUI
import XCTest

@MainActor
final class WindowTests: XCTestCase {
    func testTheControllerOpensOnTheSectionAskedAndFollowsTheThemeSettings() throws {
        let store = ThemeStore(follows: false)
        let backend = MockSettingsBackend(overrides: ["theme": "Nord", "theme_mode": "dark"])
        let controller = SettingsWindowController(model: SettingsModel(backend: backend, host: MockSettingsHost()), themeStore: store)
        XCTAssertEqual(store.theme.name, "Nord", "the stored pick is applied when the window is made")
        controller.show(section: "fonts")
        XCTAssertEqual(controller.model.selection, "fonts")
        XCTAssertNotNil(controller.window?.contentView)
        backend.set("theme", to: "Dracula")
        XCTAssertEqual(store.theme.name, "Dracula", "a change anywhere re-colours the window")
        controller.close()
    }

    func testThemeModeIsTheStoredOneEvenWhenAPickIsOfTheOtherAppearance() {
        let store = ThemeStore(follows: false)
        ThemeSync.apply(["theme": "Nord", "theme_light": "Wormhole Light", "theme_mode": "dark"], to: store)
        XCTAssertEqual(store.mode, .dark)
        XCTAssertEqual(store.theme.name, "Nord")
    }

    func testF6WalksTheAreasBothWays() {
        XCTAssertEqual(SettingsFocus.search.next(backwards: false), .nav)
        XCTAssertEqual(SettingsFocus.content.next(backwards: false), .search)
        XCTAssertEqual(SettingsFocus.search.next(backwards: true), .content)
    }

    func testEveryNavSectionHasAnIconAndNoneRepeats() {
        let ids = SettingsSchema.bundled.sections.map(\.id)
        XCTAssertEqual(Set(ids.map(NavIcons.symbol)).count, ids.count)
    }
}
