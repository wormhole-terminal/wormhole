import PanelModel
import WorkspaceModel
import WormholeBindings
import XCTest

/// The panel's keys go where the core's list rules say (`view::focus::list`), through the FFI: nothing of the walk is Swift's.
private struct CoreWalk: FocusRules {
    func step(from: WorkspaceModel.FocusArea?, delta: Int, shown: Set<WorkspaceModel.FocusArea>) -> WorkspaceModel.FocusArea { .terminal }
    func tabStep(from: WorkspaceModel.FocusStop?, delta: Int, stops: [WorkspaceModel.FocusArea: Int]) -> WorkspaceModel.FocusStop { WorkspaceModel.FocusStop(.terminal) }
    func escape(terminal: Bool, popup: Bool, detail: Bool) -> WorkspaceModel.EscapeRung { .toTerminal }
    func walk(_ items: [WorkspaceModel.ListItem], from: Int?, _ key: WorkspaceModel.ListKey) -> WorkspaceModel.ListMove {
        let core: WormholeBindings.ListKey = switch key {
        case .up: .up
        case .down: .down
        case .left: .left
        case .right: .right
        case .first: .first
        case .last: .last
        }
        return switch focusListWalk(items: items.map { WormholeBindings.ListItem(level: UInt8($0.level), fold: $0.isOpen) }, at: from.map(UInt32.init), key: core) {
        case let .to(index): .to(Int(index))
        case let .open(index): .open(Int(index))
        case let .fold(index): .fold(Int(index))
        case .stay: .stay
        }
    }
    func typeAhead(_ names: [String], from: Int?, _ typed: String) -> Int? { focusTypeAhead(names: names, at: from.map(UInt32.init), typed: typed).map(Int.init) }
    func pressFoldsBand(space: Bool) -> Bool { focusPressFoldsBand(space: space) }
}

@MainActor
private final class Sample: PanelWalking {
    var open = true
    var cursor: String?
    var pressed: [String] = []
    var walkRows: [WalkRow] {
        [WalkRow(id: "band", level: 0, isOpen: open, isBand: true, name: "Changes")]
            + (open ? [WalkRow(id: "a", level: 1, name: "alpha"), WalkRow(id: "b", level: 1, name: "beta")] : [])
    }
    func press(_ row: WalkRow, space: Bool, following: Bool) { pressed.append(row.id) }
    func fold(_ row: WalkRow) { if row.id == "band" { open.toggle() } }
}

@MainActor
final class PanelWalkTests: XCTestCase {
    func testTheKeysFollowTheCoresListRules() {
        Focus.install(CoreWalk())
        defer { Focus.install(nil) }
        let list = Sample()
        list.walk(.down)
        XCTAssertEqual(list.cursor, "band", "from nothing, down is the first row")
        list.walk(.last)
        XCTAssertEqual(list.cursor, "b")
        list.walk(.left)
        XCTAssertEqual(list.cursor, "band", "left on a child goes to its parent")
        list.pressCursor(space: false)
        XCTAssertTrue(list.open, "return does nothing on a band")
        list.pressCursor(space: true)
        XCTAssertFalse(list.open, "space folds a band")
        list.pressCursor(space: true)
        list.walk(.down)
        list.pressCursor(space: false)
        XCTAssertEqual(list.pressed, ["a"])
        list.typeAhead("b")
        XCTAssertEqual(list.cursor, "b", "type-ahead jumps to the next name that starts with it")
    }
}
