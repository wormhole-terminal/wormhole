import DesignSystem
@testable import SettingsUI
import XCTest

/// What a theme card's menu holds (`theme_picker/card.rs:84` `card_menu`), the one list the right-click menu and VoiceOver's
/// actions on the card's radio button are made from. That VoiceOver reads those actions out is for a person to hear.
final class ThemeCardMenuTests: XCTestCase {
    func testTheMenuHidesAllButThePickAndTrashesOnlyAThemeWithAFile() throws {
        let theme = try XCTUnwrap(ThemeLibrary.bundled.themes.first)
        let builtIn = ThemeCardEntry(theme: theme, value: theme.name)
        XCTAssertEqual(builtIn.menu(picked: "another"), [.hide])
        XCTAssertEqual(builtIn.menu(picked: theme.name), [], "the pick keeps its place, and a built-in has no file to trash")

        let file = "/tmp/Wormhole-swift-card-menu/themes/mine.json"
        let own = ThemeCardEntry(theme: theme, value: "mine", path: file)
        XCTAssertEqual(own.menu(picked: "another"), [.hide, .trash])
        XCTAssertEqual(own.menu(picked: file), [.trash], "picked by its file: not hidden, still its own to trash")
    }
}
