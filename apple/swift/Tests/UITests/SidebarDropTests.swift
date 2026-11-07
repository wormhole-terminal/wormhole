import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// What the pointer is over, and where the core's answer is drawn, on a layout measured as the cards stand (`cardGap` 8, rows
/// 48, a card's hairline and padding 5). Which drop does what is the core's, tested there and through the FFI.
final class SidebarDropTests: XCTestCase {
    private let gap: CGFloat = 8
    private let width: CGFloat = 330

    /// Frames for `sections` stacked from y 4: a lone card is 50 high, a group's head 36 in a card padded by 5.
    private func frames(_ sections: [SidebarSection]) -> [DropSlot: CGRect] {
        var frames: [DropSlot: CGRect] = [:]
        var y: CGFloat = 4
        for section in sections {
            guard let head = section.head else {
                for row in section.rows { frames[.row(row.id)] = CGRect(x: 10, y: y, width: width, height: 50); y += 50 + gap }
                continue
            }
            let top = y
            y += 5
            frames[.head(head.id)] = CGRect(x: 15, y: y, width: width - 10, height: 36)
            y += 36
            for row in section.rows { frames[.row(row.id)] = CGRect(x: 15, y: y, width: width - 10, height: 48); y += 48 }
            y += 5
            frames[.card(head.id)] = CGRect(x: 10, y: top, width: width, height: y - top)
            y += gap
        }
        return frames
    }

    private func lines(_ state: WorkspaceState = SidebarFixture.state) -> (DropLines, [DropSlot: CGRect]) {
        let frames = frames(state.sections)
        return (DropLines(sections: state.sections, frames: frames), frames)
    }

    private func over(_ slot: DropSlot, at fraction: CGFloat) throws -> DropOver? {
        let (lines, frames) = lines()
        let frame = try XCTUnwrap(frames[slot])
        return lines.over(at: frame.minY + frame.height * fraction)
    }

    func testARowIsOverByItsQuarters() throws {
        XCTAssertEqual(try over(.row(2), at: 0.1), .row(2, .top))
        XCTAssertEqual(try over(.row(2), at: 0.5), .row(2, .middle))
        XCTAssertEqual(try over(.row(4), at: 0.9), .row(4, .bottom))
    }

    func testAHeadIsOverByItsQuarters() throws {
        XCTAssertEqual(try over(.head(1), at: 0.1), .head(1, .top))
        XCTAssertEqual(try over(.head(1), at: 0.6), .head(1, .middle))
    }

    func testTheGapUnderACardIsAfterThatCard() throws {
        let (lines, frames) = lines()
        let lone = try XCTUnwrap(frames[.row(2)])
        XCTAssertEqual(lines.over(at: lone.maxY + 3), .after(.loose(2)))
        let last = try XCTUnwrap(frames[.row(5)])
        XCTAssertEqual(lines.over(at: last.maxY + 3), .after(.group(1)), "below a group's last row: under its card")
    }

    func testBelowTheLastCardIsTheTail() throws {
        let (lines, frames) = lines()
        let last = try XCTUnwrap(frames[.row(6)])
        XCTAssertEqual(lines.over(at: last.maxY + 200), .tail)
    }

    func testASplitCardIsNamedByItsFirstTerminal() throws {
        let state = WorkspaceState(
            terminals: [
                TerminalSummary(id: 1, title: "a", splitID: 9), TerminalSummary(id: 2, title: "b", splitID: 9),
                TerminalSummary(id: 3, title: "c"),
            ], selected: 1)
        let (lines, frames) = lines(state)
        let first = try XCTUnwrap(frames[.row(2)])
        XCTAssertEqual(lines.over(at: first.maxY + 3), .after(.split(1)))
    }

    func testTheMarkFollowsTheCoresPlan() throws {
        let (lines, frames) = lines()
        let row = try XCTUnwrap(frames[.row(4)])
        let before = lines.mark(for: .move(.group(1, before: 4), leavesSplit: false, withMates: false), over: .row(4, .top), gap: gap)
        XCTAssertEqual(before, .line(y: row.minY, x: row.minX...row.maxX))
        let inside = lines.mark(for: .move(.group(1, before: 3), leavesSplit: false, withMates: false), over: .head(1, .middle), gap: gap)
        XCTAssertEqual(inside, .card(1), "over a head the drop joins the group")
        let end = try XCTUnwrap(frames[.row(5)])
        XCTAssertEqual(
            lines.mark(for: .move(.group(1, before: nil), leavesSplit: false, withMates: false), over: .row(5, .bottom), gap: gap),
            .line(y: end.maxY, x: end.minX...end.maxX))
        let loose = try XCTUnwrap(frames[.row(2)])
        XCTAssertEqual(
            lines.mark(for: .move(.top(before: .terminal(2)), leavesSplit: false, withMates: false), over: .row(2, .top), gap: gap),
            .line(y: loose.minY - gap / 2, x: loose.minX...loose.maxX))
        XCTAssertEqual(lines.mark(for: .join(2), over: .row(2, .middle), gap: gap), .row(2))
    }

    func testAGroupsMarkIsBeforeOrAfterTheCardUnderThePointer() throws {
        let (lines, frames) = lines()
        let loose = try XCTUnwrap(frames[.row(2)])
        XCTAssertEqual(lines.groupMark(over: .row(2, .top), at: loose.minY + 5, gap: gap), .line(y: loose.minY - gap / 2, x: loose.minX...loose.maxX))
        XCTAssertEqual(lines.groupMark(over: .row(2, .bottom), at: loose.maxY - 5, gap: gap), .line(y: loose.maxY + gap / 2, x: loose.minX...loose.maxX))
    }
}
