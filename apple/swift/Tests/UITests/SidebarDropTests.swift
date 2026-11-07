import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// Where a drop lands, on a layout measured as the cards stand (`cardGap` 8, rows 48, a card's hairline and padding 5).
final class SidebarDropTests: XCTestCase {
    private let gap: CGFloat = 8
    private let width: CGFloat = 330

    /// Frames for `sections` stacked from y 4: a lone card is 50 high, a group's head 36 in a card padded by 5.
    private func frames(_ sections: [SidebarSection]) -> [DropSlot: CGRect] {
        var frames: [DropSlot: CGRect] = [:]
        var y: CGFloat = 4
        for section in sections {
            if section.isHeadless {
                for row in section.rows { frames[.row(row.id)] = CGRect(x: 10, y: y, width: width, height: 50); y += 50 + gap }
                continue
            }
            let top = y
            y += 5
            frames[.head(section.id)] = CGRect(x: 15, y: y, width: width - 10, height: 36)
            y += 36
            for row in section.rows { frames[.row(row.id)] = CGRect(x: 15, y: y, width: width - 10, height: 48); y += 48 }
            y += 5
            frames[.card(section.id)] = CGRect(x: 10, y: top, width: width, height: y - top)
            y += gap
        }
        return frames
    }

    private func map(_ state: WorkspaceState = SidebarFixture.state) -> (DropMap, [DropSlot: CGRect]) {
        let sections = SidebarListing.sections(for: state)
        let frames = frames(sections)
        return (DropMap(sections: sections, frames: frames), frames)
    }

    private func drop(_ terminal: TerminalID, onto slot: DropSlot, at fraction: CGFloat, _ state: WorkspaceState = SidebarFixture.state) throws -> WorkspaceAction? {
        let (map, frames) = map(state)
        let frame = try XCTUnwrap(frames[slot])
        return map.plan(dropping: terminal, at: CGPoint(x: frame.midX, y: frame.minY + frame.height * fraction), gap: gap)?.action
    }

    func testTheTopOfALooseRowDropsBeforeItAndTheBottomAfter() throws {
        XCTAssertEqual(try drop(6, onto: .row(2), at: 0.1), .moveTerminal(6, to: .top(before: .terminal(2))))
        XCTAssertEqual(try drop(2, onto: .row(6), at: 0.9), .moveTerminal(2, to: .top(before: nil)), "after the last card: last")
        XCTAssertEqual(try drop(6, onto: .row(1), at: 0.9), .moveTerminal(6, to: .top(before: .terminal(2))), "after a card: before what follows it")
    }

    func testARowInAGroupDropsInsideTheGroup() throws {
        XCTAssertEqual(try drop(6, onto: .row(4), at: 0.1), .moveTerminal(6, to: .group("g1", before: 4)))
        XCTAssertEqual(try drop(6, onto: .row(4), at: 0.9), .moveTerminal(6, to: .group("g1", before: 5)))
        XCTAssertEqual(try drop(6, onto: .row(5), at: 0.9), .moveTerminal(6, to: .group("g1", before: nil)))
    }

    func testTheMiddleOfARowGoesToTheNearerEdge() throws {
        XCTAssertEqual(try drop(6, onto: .row(4), at: 0.4), .moveTerminal(6, to: .group("g1", before: 4)))
        XCTAssertEqual(try drop(6, onto: .row(4), at: 0.6), .moveTerminal(6, to: .group("g1", before: 5)))
    }

    func testAGroupsHeadTakesATerminalIntoItAtTheTopOrBeforeTheGroup() throws {
        XCTAssertEqual(try drop(6, onto: .head("g1"), at: 0.1), .moveTerminal(6, to: .top(before: .group("g1"))))
        XCTAssertEqual(try drop(6, onto: .head("g1"), at: 0.6), .moveTerminal(6, to: .group("g1", before: 3)))
        var folded = SidebarFixture.state
        folded.groups[0].isCollapsed = true
        XCTAssertEqual(try drop(6, onto: .head("g1"), at: 0.6, folded), .moveTerminal(6, to: .group("g1", before: nil)), "a folded group takes it at its end")
    }

    func testBelowAGroupsLastRowIsTheGapUnderItsCard() throws {
        let (map, frames) = map()
        let last = try XCTUnwrap(frames[.row(5)])
        let plan = map.plan(dropping: 3, at: CGPoint(x: last.midX, y: last.maxY + 3), gap: gap)
        XCTAssertEqual(plan?.action, .moveTerminal(3, to: .top(before: .terminal(6))))
    }

    func testBelowTheListIsLastAndWhereItAlreadyIsIsNothing() throws {
        let (map, frames) = map()
        let last = try XCTUnwrap(frames[.row(6)])
        XCTAssertEqual(map.plan(dropping: 2, at: CGPoint(x: 100, y: last.maxY + 200), gap: gap)?.action, .moveTerminal(2, to: .top(before: nil)))
        XCTAssertNil(map.plan(dropping: 6, at: CGPoint(x: 100, y: last.maxY + 200), gap: gap), "already last")
        XCTAssertNil(try drop(4, onto: .row(4), at: 0.5), "on itself")
        XCTAssertNil(try drop(1, onto: .row(2), at: 0.1), "just before the card that follows it is where it stands")
    }

    func testAGroupLandsAtTheTopLevelBesideTheRowItIsOver() throws {
        let (map, frames) = map()
        func plan(_ slot: DropSlot, _ fraction: CGFloat) throws -> WorkspaceAction? {
            let frame = try XCTUnwrap(frames[slot])
            return map.plan(dropping: "g1", at: CGPoint(x: frame.midX, y: frame.minY + frame.height * fraction), gap: gap)?.action
        }
        XCTAssertEqual(try plan(.row(2), 0.1), .moveGroup("g1", before: .terminal(2)))
        XCTAssertEqual(try plan(.row(1), 0.9), .moveGroup("g1", before: .terminal(2)))
        XCTAssertNil(try plan(.row(6), 0.1), "before its own next neighbour is where it is")
        XCTAssertEqual(try plan(.row(6), 0.9), .moveGroup("g1", before: nil))
        XCTAssertNil(try plan(.head("g1"), 0.5), "over itself")
    }

    func testAPlansActionDoesWhatTheMarkSays() throws {
        let state = SidebarFixture.state
        let action = try XCTUnwrap(try drop(6, onto: .row(4), at: 0.1))
        let moved = reduce(state, action)
        XCTAssertEqual(moved.groups[0].members, [3, 6, 4, 5])
        let sections = SidebarListing.sections(for: moved)
        XCTAssertEqual(sections.first { $0.id == "g1" }?.rows.map(\.id), [3, 6, 4, 5])
    }
}
