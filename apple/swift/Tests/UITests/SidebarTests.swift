import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

@MainActor
final class SidebarTests: XCTestCase {
    func testSecondLineSaysWhatTheTerminalReports() {
        let needs = RowLine.of(TerminalSummary(id: 1, title: "a", subtitle: "Approve cargo test?", activity: .needsInput), twoLines: true)
        XCTAssertEqual(needs.text, "Needs input · Approve cargo test?")
        XCTAssertEqual(needs.tone, .attention)
        XCTAssertEqual(RowLine.of(TerminalSummary(id: 1, title: "a", subtitle: "1m ago", activity: .finished), twoLines: true).text, "Finished · 1m ago")
        XCTAssertEqual(RowLine.of(TerminalSummary(id: 1, title: "a"), twoLines: true).text, "Shell", "no row is a single line")
        XCTAssertEqual(RowLine.of(TerminalSummary(id: 1, title: "a", unread: 2), twoLines: true).trail, "new output")
        XCTAssertNil(RowLine.of(TerminalSummary(id: 1, title: "a", subtitle: "x"), twoLines: false).text, "compact rows have one line")
    }

    func testTileStatesFollowCoreUrgencyOrder() {
        let states = [TileState.shell, .done, .unread, .working, .failed, .needsYou]
        XCTAssertEqual(states.sorted { $0.urgency < $1.urgency }, [.needsYou, .failed, .working, .unread, .done, .shell])
        XCTAssertTrue(TileState.needsYou.isAlert && TileState.failed.isAlert && !TileState.working.isAlert)
        XCTAssertEqual(TileState(TerminalSummary(id: 1, title: "a", unread: 1)), .unread)
    }

    func testKeysWalkGroupsAndTerminals() {
        let walk = SidebarWalk(SidebarListing.sections(for: SidebarFixture.state))
        XCTAssertEqual(walk.items.map(\.stop), [.row(1), .group("g1"), .row(3), .row(4), .row(5), .row(2), .row(6)])
        let group = 1
        XCTAssertEqual(walk.move(from: group, .left), .fold(group))
        XCTAssertEqual(walk.move(from: group, .right), .to(2), "an open group goes to its first terminal")
        XCTAssertEqual(walk.move(from: 3, .left), .to(group), "a terminal goes to its group")
        XCTAssertEqual(walk.move(from: 0, .up), .stay)
        XCTAssertEqual(walk.move(from: nil, .up), .to(6))
        XCTAssertEqual(walk.move(from: 0, .last), .to(6))
        XCTAssertEqual(walk.typeAhead(from: 0, "sc"), 6)
        XCTAssertEqual(walk.typeAhead(from: 6, "sc"), 6, "the only match stays")
    }

    func testFoldedGroupsAreSkippedByKeys() {
        var state = SidebarFixture.state
        state.groups[0].isCollapsed = true
        let walk = SidebarWalk(SidebarListing.sections(for: state))
        XCTAssertEqual(walk.items.map(\.stop), [.row(1), .group("g1"), .row(2), .row(6)])
        XCTAssertEqual(walk.move(from: 1, .right), .open(1))
    }

    func testHintsShowOnlyAfterTheHoldAndNeverForAShortcut() {
        let t0 = Date(timeIntervalSinceReferenceDate: 0)
        var hold = CommandHold()
        hold.modifiers(commandAlone: true, at: t0)
        XCTAssertFalse(hold.showing(at: t0.addingTimeInterval(0.2)))
        XCTAssertEqual(try XCTUnwrap(hold.due(at: t0.addingTimeInterval(0.2))), 0.05, accuracy: 0.001)
        XCTAssertTrue(hold.showing(at: t0.addingTimeInterval(0.25)))
        hold.key()
        XCTAssertFalse(hold.showing(at: t0.addingTimeInterval(0.4)), "any other key hides them")
        hold.modifiers(commandAlone: false, at: t0.addingTimeInterval(0.5))
        hold.modifiers(commandAlone: true, at: t0.addingTimeInterval(0.6))
        XCTAssertTrue(hold.showing(at: t0.addingTimeInterval(0.9)))
        hold.blur()
        XCTAssertFalse(hold.showing(at: t0.addingTimeInterval(1)))
    }

    func testTypedLettersJoinWithinASecond() {
        var typed = TypedPrefix()
        let t0 = Date()
        typed.add("s", at: t0); typed.add("c", at: t0.addingTimeInterval(0.5))
        XCTAssertEqual(typed.text, "sc")
        typed.add("x", at: t0.addingTimeInterval(2))
        XCTAssertEqual(typed.text, "x")
    }

    func testRendersEveryThemeInLightAndDark() throws {
        let themes = ["wormhole.dark", "wormhole.light", "tokyo_night", "catppuccin_latte", "nord", "github_light"]
        for id in themes {
            let url = try SidebarSnapshot.write(theme: id, name: "sidebar-\(id)")
            XCTAssertGreaterThan(try Data(contentsOf: url).count, 1000)
        }
        try SidebarSnapshot.write(theme: "wormhole.dark", contrast: true, name: "sidebar-wormhole.dark-contrast")
        try SidebarSnapshot.write(theme: "wormhole.light", contrast: true, name: "sidebar-wormhole.light-contrast")
        for density in SidebarDensity.allCases {
            try SidebarSnapshot.write(theme: "wormhole.dark", appearance: SidebarAppearance(density: density), name: "sidebar-dark-\(density.rawValue)")
        }
        try SidebarSnapshot.write(theme: "wormhole.dark", appearance: SidebarAppearance(forcesHints: true), name: "sidebar-dark-hints")
    }
}
