@testable import SettingsUI
import AppKit
import XCTest

final class KeySpecTextTests: XCTestCase {
    func testAShortcutIsSpelledAsTheConfigDoes() {
        let mods: NSEvent.ModifierFlags = [.command, .option]
        XCTAssertEqual(KeySpecText.spec(keyCode: 38, characters: "j", modifiers: mods), "alt-cmd-j")
        XCTAssertEqual(KeySpecText.spec(keyCode: 49, characters: " ", modifiers: [.shift, .command]), "shift-cmd-space")
        XCTAssertEqual(KeySpecText.spec(keyCode: 111, characters: nil, modifiers: []), "f12")
        XCTAssertEqual(KeySpecText.spec(keyCode: 38, characters: "j", modifiers: [.control, .option, .shift, .command]), "ctrl-alt-shift-cmd-j")
    }

    func testAKeyShowsAsCaps() {
        XCTAssertEqual(KeySpecText.caps("alt-cmd-j"), ["⌥", "⌘", "J"])
        XCTAssertEqual(KeySpecText.caps("shift-cmd-space"), ["⇧", "⌘", "Space"])
        XCTAssertEqual(KeySpecText.caps(""), [])
    }

    func testAShortcutNeedsAModifierBesidesShiftOrAnFKey() {
        XCTAssertTrue(KeySpecText.isUsable("alt-cmd-j"))
        XCTAssertTrue(KeySpecText.isUsable("f12"))
        XCTAssertFalse(KeySpecText.isUsable("j"))
        XCTAssertFalse(KeySpecText.isUsable("shift-j"))
    }
}
