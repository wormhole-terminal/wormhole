@testable import WorkspaceModel
import WormholeBindings
import XCTest

@testable import WorkspaceBridge

/// The core's records as the views' values, over canned snapshots: each field lands where the views read it.
final class MappingTests: XCTestCase {
    private func row(_ id: Int64, state: RowState = .idle, agent: AgentInfo? = nil, split: SplitInfo? = nil) -> WormholeBindings.SidebarRow {
        WormholeBindings.SidebarRow(
            id: id, name: "t\(id)", number: UInt32(id), keycap: "⌘\(id)", selected: id == 1, pinned: true, state: state,
            agent: agent, message: "Approve?", branch: "main", folder: "~/app", unread: true, bell: false, failureCode: nil,
            changes: 3, notes: 2, secondLine: SecondLine(text: "cargo test", tone: .failed, mono: true, trail: "0:05"),
            split: split ?? SplitInfo(id: nil, bracket: nil, chain: false, onScreen: false, members: 0), position: 2, of: 5,
            spoken: "t\(id), idle, 2 of 5")
    }

    func testARowCarriesTheCoresFactsAndWords() {
        let claude = AgentInfo(key: "claude", name: "Claude Code", shortName: "Claude")
        let mapped = WorkspaceModel.SidebarRow(row(7, state: .needsYou, agent: claude))
        XCTAssertEqual(mapped.id, 7)
        XCTAssertEqual(mapped.terminal.title, "t7")
        XCTAssertEqual(mapped.terminal.activity, .needsInput)
        XCTAssertEqual(mapped.terminal.agent, .claude)
        XCTAssertEqual(mapped.terminal.changeCount, 3)
        XCTAssertEqual(mapped.terminal.noteCount, 2)
        XCTAssertTrue(mapped.terminal.unread && mapped.terminal.isPinned)
        XCTAssertEqual(mapped.number, 7)
        XCTAssertEqual(mapped.keycap, "⌘7")
        XCTAssertEqual(mapped.secondLine, WorkspaceModel.SecondLine(text: "cargo test", tone: .failed, isCommand: true, trail: "0:05"))
        XCTAssertEqual(mapped.position, 2)
        XCTAssertEqual(mapped.spoken, "t7, idle, 2 of 5")
    }

    func testEveryRowStateHasAnActivity() {
        let states: [RowState] = [.needsYou, .working, .finished, .failed, .idle, .exited]
        XCTAssertEqual(states.map(Activity.init), [.needsInput, .working, .finished, .failed, .idle, .exited])
    }

    func testEveryAgentCommandIsAnAgent() {
        for agent in AgentID.allCases { XCTAssertEqual(AgentID(command: agent.command), agent) }
        XCTAssertEqual(AgentID(command: "cursor-agent"), .cursor)
        XCTAssertEqual(AgentID(command: "kiro-cli"), .kiro)
        XCTAssertNil(AgentID(command: "vim"))
    }

    func testSplitMarksAndAGroupHead() {
        let marks = SplitMarks(SplitInfo(id: 9, bracket: .mid, chain: true, onScreen: true, members: 3))
        XCTAssertEqual(marks, SplitMarks(bracket: .mid, chain: true, onScreen: true, members: 3))
        let head = WorkspaceModel.GroupHead(
            WormholeBindings.GroupHead(
                id: 4, name: "Web", color: "teal", pinned: true, folded: true, folder: "~/web", position: 1, of: 2, spoken: "Web"))
        XCTAssertEqual(head, WorkspaceModel.GroupHead(id: 4, name: "Web", color: .cyan, isPinned: true, isFolded: true, folder: "~/web", spoken: "Web"))
        let plain = WorkspaceModel.GroupHead(
            WormholeBindings.GroupHead(id: 5, name: "", color: "", pinned: false, folded: false, folder: nil, position: 1, of: 1, spoken: ""))
        XCTAssertNil(plain.color, "no colour")
    }

    func testASnapshotKeepsItsSectionsSelectionAndTimer() {
        let core = WormholeBindings.SidebarSnapshot(
            revision: 3, selected: 1,
            sections: [
                WormholeBindings.SidebarSection(key: "loose-1", head: nil, rows: [row(1)], terminalCount: 1, alertCount: 0),
                WormholeBindings.SidebarSection(key: "g-2", head: nil, rows: [], terminalCount: 4, alertCount: 1),
            ], terminalCount: 5, refreshInMs: 1500)
        let mapped = WorkspaceModel.SidebarSnapshot(core)
        XCTAssertEqual(mapped.sections.map(\.id), ["loose-1", "g-2"])
        XCTAssertEqual(mapped.sections[1].terminalCount, 4)
        XCTAssertEqual(mapped.sections[1].alertCount, 1)
        XCTAssertEqual(mapped.selected, 1)
        XCTAssertEqual(mapped.refreshIn, .milliseconds(1500))
    }

    func testChromeAndTabs() {
        let chrome = ChromeState(
            ChromeSnapshot(revision: 1, sidebarVisible: false, panelVisible: true, panelTab: .history, search: "x", overlay: .palette))
        XCTAssertEqual(chrome, ChromeState(sidebarVisible: false, panelVisible: true, panelTab: .history, search: "x", overlay: .palette))
        for tab in PanelTab.allCases { XCTAssertEqual(PanelTab(TabId(tab)), tab, "\(tab) survives the round trip") }
    }

    func testActionsAndDropTargetsGoToTheCoreWithTheirIds() {
        XCTAssertEqual(WormholeBindings.Action(.rename(3, "x")), .rename(id: 3, name: "x"))
        XCTAssertEqual(WormholeBindings.Action(.newGroup([1, 2])), .newGroup(members: [1, 2]))
        XCTAssertEqual(WormholeBindings.Action(.splitRight(4)), .splitBeside(anchor: 4, axis: .row))
        XCTAssertEqual(WormholeBindings.Action(.showPanelTab(.notes)), .showPanelTab(.notes))
        XCTAssertEqual(
            WormholeBindings.Action(.dropTerminal(1, over: .after(.split(7)))), .dropTerminal(dragged: 1, over: .after(.split(7))))
        XCTAssertEqual(Over(.row(2, .middle)), .row(id: 2, part: .middle))
        XCTAssertEqual(Over(.tail), .tail)
    }

    func testAPlanComesBackAsTheViewsValues() {
        let plan = WorkspaceModel.DropPlan(.move(landing: .group(group: 2, before: 5), leaveSplit: true, withMates: false))
        XCTAssertEqual(plan, .move(.group(2, before: 5), leavesSplit: true, withMates: false))
        XCTAssertEqual(WorkspaceModel.DropPlan(.move(landing: .top(before: .group(3)), leaveSplit: false, withMates: true)),
            .move(.top(before: .group(3)), leavesSplit: false, withMates: true))
        XCTAssertEqual(WorkspaceModel.DropPlan(.join(target: 8)), .join(8))
    }
}
