import XCTest

@testable import WorkspaceModel

final class SplitActionTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func members(_ state: WorkspaceState, _ group: String) -> [TerminalID] {
        state.groups.first { $0.id == group }?.members ?? []
    }
    private func split(_ state: WorkspaceState, _ id: TerminalID) -> Int? {
        state.terminals.first { $0.id == id }?.splitID
    }

    func testJoiningMakesASplitSeatedBesideInTheSameGroup() {
        let state = reduce(mock, .joinSplit(4, with: 1))
        XCTAssertEqual(split(state, 4), split(state, 1))
        XCTAssertEqual(split(state, 1), 2, "a new split takes the next id")
        XCTAssertEqual(members(state, "wormhole"), [1, 4, 2, 3])
    }

    func testJoiningAnExistingSplitFromAnotherGroupLeavesTheOldOne() {
        let state = reduce(mock, .joinSplit(1, with: 6))
        XCTAssertEqual(split(state, 1), 1)
        XCTAssertEqual(members(state, "website"), [5, 6, 1, 7])
        XCTAssertEqual(members(state, "wormhole"), [2, 3, 4])
    }

    func testALeavingMemberTakesTheLastOfItsSplitAlong() {
        let state = reduce(mock, .joinSplit(5, with: 1))
        XCTAssertNil(split(state, 6), "a split left with one terminal is none")
        XCTAssertEqual(members(state, "wormhole"), [1, 5, 2, 3, 4])
        XCTAssertEqual(members(state, "website"), [6, 7])
    }

    func testALooseTerminalJoinsTheGroupOfItsNeighbour() {
        let state = reduce(mock, .joinSplit(13, with: 1))
        XCTAssertEqual(members(state, "wormhole"), [1, 13, 2, 3, 4])
    }

    func testAGroupedTerminalJoiningALooseOneLeavesItsGroup() {
        let state = reduce(mock, .joinSplit(1, with: 11))
        XCTAssertEqual(members(state, "wormhole"), [2, 3, 4])
        let run = SidebarListing.sections(for: state).first { $0.id == "loose-11" }
        XCTAssertEqual(run?.rows.map(\.id), [11, 1])
        XCTAssertEqual(run?.rows.map(\.splitMarks.bracket), [.start, .end])
    }

    func testJoiningItselfAnUnknownOrTheSameSplitDoesNothing() {
        XCTAssertEqual(reduce(mock, .joinSplit(1, with: 1)), mock)
        XCTAssertEqual(reduce(mock, .joinSplit(1, with: 99)), mock)
        XCTAssertEqual(reduce(mock, .joinSplit(5, with: 6)), mock)
    }

    func testLeavingASplit() {
        let pair = reduce(mock, .leaveSplit(5))
        XCTAssertNil(split(pair, 5))
        XCTAssertNil(split(pair, 6), "the other half of a pair is no split either")
        var three = reduce(mock, .joinSplit(7, with: 5))
        three = reduce(three, .leaveSplit(7))
        XCTAssertEqual(split(three, 5), 1)
        XCTAssertEqual(split(three, 6), 1)
        XCTAssertNil(split(three, 7))
        XCTAssertEqual(reduce(mock, .leaveSplit(1)), mock)
    }

    func testSplitRightOfAMemberJoinsItsSplit() {
        let state = reduce(mock, .splitRight(5))
        XCTAssertEqual(members(state, "website"), [5, 6, 14, 7])
        XCTAssertEqual(split(state, 14), 1)
    }
}
