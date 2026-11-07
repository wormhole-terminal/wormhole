import XCTest

@testable import WorkspaceModel

final class SpokenLabelTests: XCTestCase {
    func testTerminalLabelCarriesStateDetailUnreadAndPin() {
        let terminal = TerminalSummary(
            id: 1, title: "sidebar polish", subtitle: "~/app", branch: "main", activity: .needsInput,
            isPinned: true, unread: 2)
        XCTAssertEqual(
            SidebarListing.spokenLabel(for: terminal),
            "sidebar polish, needs input, ~/app, branch main, 2 unread, pinned")
        XCTAssertEqual(SidebarListing.spokenLabel(for: TerminalSummary(id: 2, title: "zsh")), "zsh, idle")
    }

    func testEveryActivityHasItsOwnWord() {
        let words = [Activity.idle, .working, .needsInput, .finished, .failed].map {
            SidebarListing.spokenLabel(for: TerminalSummary(id: 1, title: "t", activity: $0))
        }
        XCTAssertEqual(words, ["t, idle", "t, working", "t, needs input", "t, finished", "t, failed"])
    }

    func testRowLabelAddsSelectionPlaceAndShortcut() {
        let row = SidebarRow(terminal: TerminalSummary(id: 1, title: "cargo test", activity: .working), number: 1)
        XCTAssertEqual(
            SidebarListing.spokenLabel(for: row, selected: true, position: 1, of: 3),
            "cargo test, working, selected, 1 of 3, Command 1")
        let plain = SidebarRow(terminal: TerminalSummary(id: 2, title: "zsh"), number: nil)
        XCTAssertEqual(SidebarListing.spokenLabel(for: plain, selected: false, position: 2, of: 3), "zsh, idle, 2 of 3")
    }

    func testGroupHeadSaysCountStateAndAttentionOnlyWhileFolded() {
        let sections = SidebarListing.sections(for: MockWorkspace.state)
        XCTAssertEqual(
            SidebarListing.spokenLabel(for: sections[1], position: 2, of: 5),
            "Wormhole, 3 terminals, expanded, 2 of 5")
        XCTAssertEqual(
            SidebarListing.spokenLabel(for: sections[3], position: 4, of: 5),
            "Infra, 2 terminals, collapsed, 4 of 5")
        let alerting = reduce(MockWorkspace.state, .toggleGroup("website"))
        XCTAssertEqual(
            SidebarListing.spokenLabel(for: SidebarListing.sections(for: alerting)[2], position: 3, of: 5),
            "Website, 3 terminals, 1 need attention, collapsed, 3 of 5")
    }
}
