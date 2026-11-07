@testable import SettingsBridge
import SettingsModel
import WormholeBindings
import XCTest

@MainActor
final class SearchAndShortcutTests: BridgeTestCase {
    func testSearchRanksByCoreRulesAndMarksTheLabel() throws {
        let kit = try makeKit()
        let hits = kit.searcher.search("line height", values: [:])
        XCTAssertEqual(hits.first?.entry.id, "line-height")
        XCTAssertNotNil(hits.first?.labelRange)
        XCTAssertTrue(kit.searcher.search("   ", values: [:]).isEmpty)
        XCTAssertTrue(kit.searcher.search("zzzznothing", values: [:]).isEmpty)
    }

    func testByteSpansBecomeCharacterRanges() {
        let span = { (start: UInt32, end: UInt32) in SettingsTextSpan(start: start, end: end) }
        // "Größe": ö and ß are two bytes each.
        XCTAssertEqual(RustSettingsSearch.characters(span(0, 4), in: "Größe"), 0..<3)
        XCTAssertEqual(RustSettingsSearch.characters(span(4, 6), in: "Größe"), 3..<4)
        XCTAssertNil(RustSettingsSearch.characters(span(2, 3), in: "Größe"), "inside ö")
        XCTAssertNil(RustSettingsSearch.characters(span(0, 99), in: "Größe"))
    }

    func testShortcutsFollowCoresRules() {
        let rules = RustShortcuts()
        XCTAssertEqual(rules.check(key: "hotkey", spec: "shift-cmd-space"), .accepted("shift-cmd-space"))
        XCTAssertEqual(rules.check(key: "hotkey", spec: "cmd-space"), .refused("macOS uses ⌘Space for Spotlight"))
        XCTAssertEqual(rules.check(key: "hotkey", spec: "alt-space"), .refused("The shell uses ⌥Space"))
        XCTAssertEqual(rules.check(key: "hotkey", spec: "j"), .needsModifier)
        XCTAssertEqual(rules.check(key: "hotkey", spec: "escape"), .cancel)
        XCTAssertEqual(rules.check(key: "hotkey", spec: "backspace"), .clear)
        XCTAssertEqual(rules.check(key: "jump_waiting_key", spec: "alt-cmd-j"), .accepted("alt-cmd-j"))
        guard case .refused = rules.check(key: "hotkey", spec: "alt-cmd-j") else { return XCTFail("the jump key's own") }
    }
}
