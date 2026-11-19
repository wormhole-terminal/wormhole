import PanesModel
import XCTest

@testable import WorkspaceBridge

/// The tab bar's sets through the real core: a key's step and number, and the switcher's search.
final class TabSetsFFITests: XCTestCase {
    private let rules = CoreTabSetRules()
    private let sets = [
        TabSet(groupID: 1, label: "A", members: [0]), TabSet(groupID: 4, label: "Docs", members: []),
        TabSet(groupID: 2, label: "B", members: [1, 2]), TabSet(groupID: nil, label: "Ungrouped", members: [3]),
    ]

    func testAStepWrapsAndPassesOverAnEmptySet() {
        XCTAssertEqual(rules.step(sets, from: 0, forward: true), 2)
        XCTAssertEqual(rules.step(sets, from: 3, forward: true), 0)
        XCTAssertEqual(rules.step(sets, from: 0, forward: false), 3)
        XCTAssertNil(rules.step([sets[0]], from: 0, forward: true))
        XCTAssertNil(rules.step(sets, from: 9, forward: true), "a set that is not there")
    }

    func testTheCurrentSetAndTheNumberedOne() {
        XCTAssertEqual(rules.current(sets, containing: 2), 2)
        XCTAssertNil(rules.current(sets, containing: 9))
        XCTAssertEqual(rules.numbered(sets, 4), 3, "empty sets count")
        XCTAssertNil(rules.numbered(sets, 5))
        XCTAssertNil(rules.numbered(sets, 0))
    }

    func testSearchFindsSetsByNameThenTerminals() {
        let names = ["zero", "docs reader", "two", "three"]
        XCTAssertEqual(rules.search(sets, "doc", name: { names[$0] }), [.set(1), .terminal(1, set: 2)])
        XCTAssertTrue(rules.search(sets, "  ", name: { names[$0] }).isEmpty)
    }
}
