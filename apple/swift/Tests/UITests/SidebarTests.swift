import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

@MainActor
final class SidebarTests: XCTestCase {
    /// What is on screen is listed (groups and terminals as stops, a folded group's rows left out); where a key
    /// goes is the core's, tested through the FFI (`FocusFFITests`).
    func testKeysListGroupsAndTerminalsOnScreen() {
        XCTAssertEqual(
            SidebarWalk(SidebarFixture.state.sections).items.map(\.stop),
            [.row(1), .row(2), .group(1), .row(3), .row(4), .row(5), .row(6)])
        XCTAssertEqual(SidebarWalk(SidebarFixture.folded.sections).items.map(\.stop), [.row(1), .row(2), .group(1), .row(6)])
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
            TerminalSummary(id: 3, title: long, folder: "~", hasBell: true),
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
        let aim = DropAim(action: .dropTerminal(6, over: .row(4, .top)), mark: .line(y: 188, x: 15...335))
        let drag = LiveDrag(item: .terminal(6), point: CGPoint(x: 150, y: 186), aim: aim)
        try SidebarSnapshot.write(theme: "wormhole.dark", name: "moment-drag", caught: .init(drag: drag))
        let join = LiveDrag(item: .terminal(6), point: CGPoint(x: 150, y: 130), aim: DropAim(action: .dropTerminal(6, over: .head(1, .middle)), mark: .card(1)))
        try SidebarSnapshot.write(theme: "wormhole.dark", name: "moment-drag-join", caught: .init(drag: join))
    }

    func testTheGroupPopoverAndMenuContents() throws {
        let section = try XCTUnwrap(SidebarFixture.state.sections.first { $0.head != nil })
        let context = SidebarContext(
            selected: nil, groups: [], send: { _ in }, renaming: nil, rename: { _ in }, shifts: [:], drag: .none,
            everyTerminal: [], newInGroup: { _ in }, closeGroup: { _ in }, popover: nil, openPopover: { _ in }, frames: RowFrames())
        for id in ["wormhole.dark", "wormhole.light"] {
            let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: id))
            try SidebarSnapshot.render(GroupPopover(head: try XCTUnwrap(section.head), rows: section.rows, context: context, close: {}), theme: theme, name: "moment-popover-\(id)", size: CGSize(width: 248, height: 140))
        }
    }

    func testEveryStatusMotionAndNoMotionRender() throws {
        for motion in StatusMotion.allCases {
            try SidebarSnapshot.write(theme: "wormhole.dark", appearance: SidebarAppearance(statusMotion: motion), name: "motion-\(motion.rawValue)")
        }
    }
}
