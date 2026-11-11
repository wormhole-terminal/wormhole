import WorkspaceModel
import XCTest

@testable import WorkspaceBridge

/// The focus rules through the real core: F6's order, ⇥'s stops, the Esc ladder and a list's keys.
final class FocusFFITests: XCTestCase {
    private let rules = CoreFocusRules()
    private let everything = Set(FocusArea.allCases)

    func testF6SkipsHiddenAreasAndWraps() {
        XCTAssertEqual(rules.step(from: .terminal, delta: 1, shown: everything), .panelTabs)
        let bare: Set<FocusArea> = [.sidebar, .terminal]
        XCTAssertEqual(rules.step(from: .terminal, delta: 1, shown: bare), .sidebar)
        XCTAssertEqual(rules.step(from: .sidebar, delta: -1, shown: bare), .terminal)
        XCTAssertEqual(rules.step(from: nil, delta: 1, shown: bare), .sidebar, "from nowhere as from the terminal")
    }

    func testTabEntersTheTitleRowButtonByButtonOnlyWhenItHasStops() {
        var stops = Dictionary(uniqueKeysWithValues: FocusArea.allCases.map { ($0, 1) })
        stops[.titleRow] = 2
        let first = rules.tabStep(from: FocusStop(.statusBar), delta: 1, stops: stops)
        XCTAssertEqual(first, FocusStop(.titleRow, 0))
        XCTAssertEqual(rules.tabStep(from: first, delta: 1, stops: stops), FocusStop(.titleRow, 1))
        stops[.titleRow] = 0
        XCTAssertEqual(rules.tabStep(from: FocusStop(.statusBar), delta: 1, stops: stops), FocusStop(.sidebar))
    }

    func testEscClimbsNearestFirstAndOnlyAFocusedTerminalSendsItToTheShell() {
        XCTAssertEqual(rules.escape(terminal: false, popup: true, detail: true), .closePopup)
        XCTAssertEqual(rules.escape(terminal: true, popup: false, detail: true), .closeDetail, "a find bar closes before the shell hears Esc")
        XCTAssertEqual(rules.escape(terminal: false, popup: false, detail: false), .toTerminal)
        XCTAssertEqual(rules.escape(terminal: true, popup: false, detail: false), .toShell)
    }

    func testAListFoldsGoesToItsParentAndTypesAhead() {
        let items = [ListItem(level: 0, isOpen: true), ListItem(level: 1, isOpen: nil), ListItem(level: 0, isOpen: false)]
        XCTAssertEqual(rules.walk(items, from: 0, .left), .fold(0))
        XCTAssertEqual(rules.walk(items, from: 0, .right), .to(1), "an open band goes to its first item")
        XCTAssertEqual(rules.walk(items, from: 1, .left), .to(0), "an item goes to its band")
        XCTAssertEqual(rules.walk(items, from: 2, .right), .open(2))
        XCTAssertEqual(rules.walk(items, from: nil, .up), .to(2))
        XCTAssertEqual(rules.typeAhead(["api", "server", "scratch"], from: 1, "sc"), 2)
        XCTAssertTrue(rules.pressFoldsBand(space: true))
        XCTAssertFalse(rules.pressFoldsBand(space: false), "Return never folds a band away under the keys")
    }
}
