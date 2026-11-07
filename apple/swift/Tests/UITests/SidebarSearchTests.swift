import DesignSystem
import SwiftUI
import XCTest
@testable import SidebarUI
@testable import WorkspaceModel

/// The search over the list: the field's pictures on sample data (the core does the filtering, so the sample is the list
/// as it would come back), ⌘F's claim on the keyboard and the pointer a drag out of the list leaves for the panes.
@MainActor
final class SidebarSearchTests: XCTestCase {
    /// "diff" found one terminal in the group: the group stays with that one, the others are not listed.
    private var found: WorkspaceState {
        var state = SidebarFixture.state
        state.search = "diff"
        state.sections = state.sections.compactMap { section in
            guard let head = section.head else { return nil }
            let rows = section.rows.filter { $0.terminal.title.contains("diff") }
            return SidebarSection(id: section.id, head: head, rows: rows, terminalCount: rows.count)
        }
        return state
    }

    private var nothing: WorkspaceState {
        var state = SidebarFixture.state
        state.search = "zzz"
        state.sections = []
        return state
    }

    func testTheFieldAndWhatItFoundInThreeThemes() throws {
        for id in ["wormhole.dark", "wormhole.light", "catppuccin_mocha", "github_light"] {
            try SidebarSnapshot.write(found, theme: id, name: "search-found-\(id)", caught: .init(searchOpen: true))
        }
        try SidebarSnapshot.write(found, theme: "wormhole.dark", contrast: true, name: "search-found-contrast", caught: .init(searchOpen: true))
        try SidebarSnapshot.write(nothing, theme: "wormhole.dark", name: "search-nothing", caught: .init(searchOpen: true))
        try SidebarSnapshot.write(SidebarFixture.state, theme: "wormhole.dark", name: "search-empty-field", caught: .init(searchOpen: true))
    }

    func testTheFieldFocusedWearsTheRing() throws {
        struct Harness: View {
            @State var text = "dif"
            @FocusState var focus: Bool
            var body: some View {
                SidebarSearchField(text: $text, found: 1, total: 6, focus: $focus, exit: {}, submit: {}, toList: {})
                    .padding(Space._2).onAppear { focus = true }
            }
        }
        for id in ["wormhole.dark", "wormhole.light"] {
            let theme = try XCTUnwrap(ThemeLibrary.bundled.theme(id: id))
            try SidebarSnapshot.render(Harness(), theme: theme, name: "search-field-\(id)", size: CGSize(width: 350, height: 48))
        }
    }

    func testCommandFBelongsToTheSidebarOnlyWhileItHasTheKeyboard() {
        let search = SidebarSearch()
        XCTAssertFalse(search.revealIfFocused(), "the terminal's find")
        XCTAssertEqual(search.requests, 0)
        search.hasKeyboard = true
        XCTAssertTrue(search.revealIfFocused())
        XCTAssertEqual(search.requests, 1)
        search.reveal()
        XCTAssertEqual(search.requests, 2, "the palette or VoiceOver asks whatever has the keyboard")
    }

    func testADragOutOfTheListIsReportedUntilItLetsGo() {
        let pointer = TerminalDragPointer()
        pointer.move(4, to: CGPoint(x: 500, y: 200))
        XCTAssertEqual(pointer.over, .init(terminal: 4, point: CGPoint(x: 500, y: 200)))
        pointer.leave()
        XCTAssertNil(pointer.over)
        pointer.move(4, to: CGPoint(x: 600, y: 210))
        pointer.release(4, at: CGPoint(x: 610, y: 212))
        XCTAssertNil(pointer.over)
        XCTAssertEqual(pointer.released?.point, CGPoint(x: 610, y: 212))
        pointer.settle()
        XCTAssertNil(pointer.released)
    }
}
