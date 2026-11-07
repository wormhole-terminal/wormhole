import XCTest

@testable import PanelModel

final class PanelLogicTests: XCTestCase {
    func testAddTrimsAndIgnoresBlank() {
        var list = TaskList()
        XCTAssertNil(list.add("   \n"))
        XCTAssertEqual(list.add("  write tests ")?.text, "write tests")
        XCTAssertEqual(list.items.count, 1)
    }

    func testToggleDeleteAndRemaining() throws {
        var list = TaskList()
        let one = try XCTUnwrap(list.add("one"))
        let two = try XCTUnwrap(list.add("two"))
        list.toggle(one.id)
        XCTAssertEqual(list.remaining, 1)
        list.toggle(one.id)
        XCTAssertEqual(list.remaining, 2)
        list.delete(one.id)
        XCTAssertEqual(list.items.map(\.id), [two.id])
        list.toggle(UUID())
        XCTAssertEqual(list.remaining, 1)
    }

    func testClearDoneKeepsOrder() throws {
        var list = TaskList()
        let ids = try ["a", "b", "c"].map { try XCTUnwrap(list.add($0)).id }
        list.toggle(ids[1])
        list.clearDone()
        XCTAssertEqual(list.items.map(\.text), ["a", "c"])
    }

    func testFileNameAndFolder() {
        let file = ChangedFile(path: "crates/wormhole/src/lib.rs", status: .modified, staged: false)
        XCTAssertEqual(file.name, "lib.rs")
        XCTAssertEqual(file.folder, "crates/wormhole/src/")
        XCTAssertEqual(ChangedFile(path: "README.md", status: .added, staged: true).folder, "")
    }

    func testStagedAndUnstagedCopiesOfOneFileHaveDifferentIds() {
        let a = ChangedFile(path: "a", status: .modified, staged: true)
        let b = ChangedFile(path: "a", status: .modified, staged: false)
        XCTAssertNotEqual(a.id, b.id)
    }

    func testListedOrdersStagedFirst() {
        XCTAssertEqual(PanelSamples.changes.listed.map(\.staged), [true, true, false, false, false, false])
    }

    func testRelativeDates() {
        let now = PanelSamples.now
        func text(_ seconds: Double) -> String { RelativeDate.text(now.addingTimeInterval(-seconds), now: now) }
        XCTAssertEqual(text(5), "just now")
        XCTAssertEqual(text(300), "5 min ago")
        XCTAssertEqual(text(7_200), "2 h ago")
        XCTAssertEqual(text(86_400 * 3), "3 d ago")
        XCTAssertEqual(text(86_400 * 60), "2 mo ago")
        XCTAssertEqual(text(86_400 * 800), "2 y ago")
        XCTAssertEqual(RelativeDate.text(now.addingTimeInterval(30), now: now), "just now")
    }

    func testListNavigationStopsAtEnds() {
        let ids = ["a", "b", "c"]
        XCTAssertEqual(ListNavigation.move(in: ids, from: nil, by: 1), "a")
        XCTAssertEqual(ListNavigation.move(in: ids, from: nil, by: -1), "c")
        XCTAssertEqual(ListNavigation.move(in: ids, from: "b", by: 1), "c")
        XCTAssertEqual(ListNavigation.move(in: ids, from: "c", by: 1), "c")
        XCTAssertEqual(ListNavigation.move(in: ids, from: "a", by: -1), "a")
        XCTAssertNil(ListNavigation.move(in: [String](), from: nil, by: 1))
    }

    func testShortHash() {
        XCTAssertEqual(PanelSamples.history.commits[0].shortHash, "9f3c1ab")
    }
}
