import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

@MainActor
final class SidebarTests: XCTestCase {
    func testKeysWalkGroupsAndTerminals() {
        let walk = SidebarWalk(SidebarListing.sections(for: SidebarFixture.state))
        XCTAssertEqual(walk.items.map(\.stop), [.row(1), .row(2), .group("g1"), .row(3), .row(4), .row(5), .row(6)])
        let group = 2
        XCTAssertEqual(walk.move(from: group, .left), .fold(group))
        XCTAssertEqual(walk.move(from: group, .right), .to(3), "an open group goes to its first terminal")
        XCTAssertEqual(walk.move(from: 4, .left), .to(group), "a terminal goes to its group")
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
        XCTAssertEqual(walk.items.map(\.stop), [.row(1), .row(2), .group("g1"), .row(6)])
        XCTAssertEqual(walk.move(from: 2, .right), .open(2))
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

    func testTheNextIdsTheListGuessesAreTheOnesTheModelGives() {
        let state = MockWorkspace.state
        let guessedTerminal = (state.terminals.map(\.id).max() ?? 0) + 1
        XCTAssertEqual(reduce(state, .newTerminal).selected, guessedTerminal)
        let guessedGroup = (1...).lazy.map { "group-\($0)" }.first { id in !state.groups.contains { $0.id == id } }
        XCTAssertEqual(reduce(state, .newGroup("")).groups.last?.id, guessedGroup)
    }

    func testSplitMembersShareACardAndOthersStandAlone() {
        func row(_ id: Int, _ part: Bracket?) -> SidebarRow {
            SidebarRow(terminal: TerminalSummary(id: id, title: "t\(id)"), number: nil, splitMarks: SplitMarks(bracket: part))
        }
        let cards = SidebarSectionView.cards(of: [row(1, nil), row(2, .start), row(3, .mid), row(4, .end), row(5, nil)])
        XCTAssertEqual(cards.map { $0.map(\.id) }, [[1], [2, 3, 4], [5]])
    }

    func testRendersEveryAgentsTileAndTheWholeMockWindow() throws {
        try SidebarSnapshot.write(SidebarFixture.agents, theme: "wormhole.dark", name: "sidebar-agents-dark", size: CGSize(width: 350, height: 1000))
        try SidebarSnapshot.write(SidebarFixture.agents, theme: "wormhole.light", name: "sidebar-agents-light", size: CGSize(width: 350, height: 1000))
        try SidebarSnapshot.write(MockWorkspace.state, theme: "wormhole.dark", name: "sidebar-mock-dark", size: CGSize(width: 350, height: 900))
        try SidebarSnapshot.write(MockWorkspace.state, theme: "tokyo_night", appearance: SidebarAppearance(cards: false), name: "sidebar-mock-nocards", size: CGSize(width: 350, height: 900))
    }

    func testLongNamesFadeOutBesideTheirMarks() throws {
        let long = "a very long terminal name that cannot fit"
        let state = WorkspaceState(terminals: [
            TerminalSummary(id: 1, title: long, isPinned: true, agent: .claude, noteCount: 3, changeCount: 12),
            TerminalSummary(id: 2, title: "Foundation Models", folder: "~"),
            TerminalSummary(id: 3, title: long, hasBell: true, folder: "~"),
        ])
        try SidebarSnapshot.write(state, theme: "wormhole.dark", name: "long-names", size: CGSize(width: 350, height: 300))
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

@MainActor
final class SidebarMomentTests: XCTestCase {
    private let size = CGSize(width: 350, height: 560)

    func testTheRowUnderThePointerShowsItsCloseAndAHalfTypedName() throws {
        try SidebarSnapshot.write(theme: "wormhole.dark", appearance: SidebarAppearance(hovered: 2), name: "moment-hover")
        try SidebarSnapshot.write(theme: "wormhole.dark", name: "moment-rename", caught: .init(renaming: 2))
        try SidebarSnapshot.write(theme: "wormhole.light", appearance: SidebarAppearance(hovered: 4), name: "moment-hover-light")
    }

    func testADragShowsTheLiftedRowAndWhereItLands() throws {
        let plan = DropPlan(action: .moveTerminal(6, to: .group("g1", before: 4)), mark: .line(y: 188, x: 15...335))
        let drag = LiveDrag(item: .terminal(6), point: CGPoint(x: 150, y: 186), plan: plan)
        try SidebarSnapshot.write(theme: "wormhole.dark", name: "moment-drag", caught: .init(drag: drag))
        let join = LiveDrag(item: .terminal(6), point: CGPoint(x: 150, y: 130), plan: DropPlan(action: .moveTerminal(6, to: .group("g1", before: 3)), mark: .card("g1")))
        try SidebarSnapshot.write(theme: "wormhole.dark", name: "moment-drag-join", caught: .init(drag: join))
    }

    func testTheGroupPopoverAndMenuContents() throws {
        let group = try XCTUnwrap(SidebarFixture.state.groups.first)
        let edits = GroupEdits(rename: { _, _ in }, setColor: { _, _ in }, togglePin: { _ in }, ungroup: { _ in }, close: { _ in })
        for id in ["wormhole.dark", "wormhole.light"] {
            let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: id))
            try SidebarSnapshot.render(GroupPopover(group: group, edits: edits, close: {}), theme: theme, name: "moment-popover-\(id)", size: CGSize(width: 248, height: 140))
        }
    }

    func testEveryStatusMotionAndNoMotionRender() throws {
        for motion in StatusMotion.allCases {
            try SidebarSnapshot.write(theme: "wormhole.dark", appearance: SidebarAppearance(statusMotion: motion), name: "motion-\(motion.rawValue)")
        }
    }
}
