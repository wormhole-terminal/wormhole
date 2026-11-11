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
            spoken: "t\(id), idle, 2 of 5", repo: "/Users/ref/app", dirty: true, ahead: 2, behind: 1, repoFiles: 4,
            glyph: .terminal(unread: true), agentHidden: true, forgetText: true)
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

    func testARowCarriesGitAndTheMenuChecksAndTheTilesGlyph() {
        let terminal = WorkspaceModel.SidebarRow(row(2)).terminal
        XCTAssertEqual(terminal.repo, "/Users/ref/app")
        XCTAssertTrue(terminal.isDirty)
        XCTAssertEqual([terminal.ahead, terminal.behind, terminal.repoFiles ?? 0], [2, 1, 4])
        XCTAssertTrue(terminal.isAgentHidden && terminal.forgetsText)
        XCTAssertEqual(terminal.glyph, .terminal(unread: true))
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
                id: 4, name: "Web", color: "teal", pinned: true, folded: true, folder: "~/web", position: 1, of: 2, spoken: "Web",
                folderPath: "/Users/ref/web", onlyInFolder: true))
        XCTAssertEqual(
            head,
            WorkspaceModel.GroupHead(
                id: 4, name: "Web", color: .cyan, isPinned: true, isFolded: true, folder: "~/web", spoken: "Web",
                folderPath: "/Users/ref/web", onlyInFolder: true))
        let custom = WorkspaceModel.GroupHead(
            WormholeBindings.GroupHead(
                id: 6, name: "", color: "#12abff", pinned: false, folded: false, folder: nil, position: 1, of: 1, spoken: "",
                folderPath: nil, onlyInFolder: false))
        XCTAssertNil(custom.color)
        XCTAssertEqual(custom.customColor, "#12abff", "a free pick is none of the nine names")
        let plain = WorkspaceModel.GroupHead(
            WormholeBindings.GroupHead(
                id: 5, name: "", color: "", pinned: false, folded: false, folder: nil, position: 1, of: 1, spoken: "", folderPath: nil,
                onlyInFolder: false))
        XCTAssertNil(plain.color, "no colour")
    }

    func testASnapshotKeepsItsSectionsSelectionAndTimer() {
        let core = WormholeBindings.SidebarSnapshot(
            revision: 3, selected: 1,
            sections: [
                WormholeBindings.SidebarSection(
                    key: "loose-1", head: nil, rows: [row(1)], terminalCount: 1, alertCount: 0, tiles: [], alert: nil, workingCount: 0),
                WormholeBindings.SidebarSection(
                    key: "g-2", head: nil, rows: [], terminalCount: 4, alertCount: 1,
                    tiles: [WormholeBindings.MemberTile(id: 3, name: "api", glyph: .needsYou, state: .needsYou, agent: nil, urgency: .needsYou)],
                    alert: .needsYou, workingCount: 2),
            ], terminalCount: 5, refreshInMs: 1500)
        let mapped = WorkspaceModel.SidebarSnapshot(core)
        XCTAssertEqual(mapped.sections.map(\.id), ["loose-1", "g-2"])
        XCTAssertEqual(mapped.sections[1].terminalCount, 4)
        XCTAssertEqual(mapped.sections[1].alertCount, 1)
        XCTAssertEqual(mapped.sections[1].tiles, [MemberTile(id: 3, name: "api", glyph: .needsYou, activity: .needsInput, urgency: .needsYou)])
        XCTAssertEqual(mapped.sections[1].alert, .needsYou)
        XCTAssertEqual(mapped.sections[1].workingCount, 2)
        XCTAssertEqual(mapped.selected, 1)
        XCTAssertEqual(mapped.refreshIn, .milliseconds(1500))
    }

    private func chromeSnapshot(toggle: WormholeBindings.PanelToggle = .open) -> ChromeSnapshot {
        ChromeSnapshot(
            revision: 1, sidebarVisible: false, panelVisible: true, panelTab: .history, search: "x", overlay: .palette,
            panelWanted: true, panelToggle: toggle, windowWidth: 1000, sidebarWidth: 0, terminalWidth: 640, panelWidth: 360,
            sidebarChosenWidth: 350, panelChosenWidth: 360, sidebarMaxNow: 420, panelMaxNow: 720,
            limits: WormholeBindings.ColumnLimits(
                sidebarMin: 240, sidebarMax: 420, panelMin: 240, panelMax: 720, terminalMin: 320, windowMinWidth: 640, windowMinHeight: 360),
            tabs: [WormholeBindings.PanelTab(tab: .changes, key: "changes", label: "Changes", enabled: true, selected: false)],
            layout: LayoutInfo(
                presets: [PresetInfo(preset: .twoColumns, title: "Two columns", slots: 2, current: true, enabled: false)],
                canSplitRow: true, canSplitColumn: false, paneCount: 2, zoomed: false),
            selected: 1, zoomed: nil, numbersFollowList: true)
    }

    func testChromeAndTabs() {
        let chrome = ChromeState(chromeSnapshot(toggle: .waiting))
        XCTAssertTrue(chrome.panelVisible, "what the person wants, whether or not there is room")
        XCTAssertEqual(chrome.panelTab, .history)
        XCTAssertEqual(chrome.layout?.panelToggle, .waiting)
        XCTAssertEqual(chrome.layout?.columns.panel, 360)
        XCTAssertEqual(chrome.layout?.columns.limits.terminalMin, 320)
        XCTAssertEqual(chrome.layout?.tabs, [PanelTabItem(tab: .changes, label: "Changes")])
        XCTAssertEqual(chrome.layout?.menu.presets, [LayoutPreset(id: .twoColumns, title: "Two columns", slots: 2, current: true, enabled: false)])
        XCTAssertEqual([chrome.layout?.menu.canSplitRow, chrome.layout?.menu.canSplitColumn], [true, false])
        XCTAssertEqual(WorkspaceModel.PanelToggle(.closed(hidesSidebar: true)), .closed(hidesSidebar: true))
        for tab in PanelTab.allCases { XCTAssertEqual(PanelTab(TabId(tab)), tab, "\(tab) survives the round trip") }
    }

    func testActionsAndDropTargetsGoToTheCoreWithTheirIds() {
        XCTAssertEqual(WormholeBindings.Action(.rename(3, "x")), .rename(id: 3, name: "x"))
        XCTAssertEqual(WormholeBindings.Action(.newGroup([1, 2])), .newGroup(members: [1, 2]))
        XCTAssertEqual(WormholeBindings.Action(.splitRight(4)), .splitBeside(anchor: 4, axis: .row))
        XCTAssertEqual(WormholeBindings.Action(.splitDown(4)), .splitBeside(anchor: 4, axis: .column))
        XCTAssertEqual(WormholeBindings.Action(.movePane(3, beside: 1, zone: .top)), .movePane(id: 3, target: 1, zone: .top))
        XCTAssertEqual(WormholeBindings.Action(.focusDirection(.left)), .focusDirection(.left))
        XCTAssertEqual(WormholeBindings.Action(.showPanelTab(.notes)), .showPanelTab(.notes))
        XCTAssertEqual(
            WormholeBindings.Action(.dropTerminal(1, over: .after(.split(7)))), .dropTerminal(dragged: 1, over: .after(.split(7))))
        XCTAssertEqual(Over(.row(2, .middle)), .row(id: 2, part: .middle))
        XCTAssertEqual(Over(.tail), .tail)
    }

    func testTheNewActionsGoToTheCore() {
        XCTAssertEqual(WormholeBindings.Action(.setGroupColor(3, .blue)), .setGroupColor(group: 3, color: .blue))
        XCTAssertEqual(WormholeBindings.Action(.setGroupColor(3, nil)), .setGroupColor(group: 3, color: nil))
        XCTAssertEqual(WormholeBindings.Action(.setGroupCustomColor(3, red: 1, green: 2, blue: 3)), .setGroupCustomColor(group: 3, r: 1, g: 2, b: 3))
        XCTAssertEqual(WormholeBindings.Action(.setGroupFolder(3, "/x")), .setGroupFolder(group: 3, folder: "/x"))
        XCTAssertEqual(WormholeBindings.Action(.closeGroup(3)), .closeGroup(3))
        XCTAssertEqual(WormholeBindings.Action(.setAgentHidden(2, true)), .setAgentHidden(id: 2, hidden: true))
        XCTAssertEqual(WormholeBindings.Action(.showBeside(1, other: 2)), .showBeside(id: 1, other: 2))
        XCTAssertEqual(WormholeBindings.Action(.applyLayout(.twoRows)), .applyLayout(.twoRows))
        XCTAssertEqual(WormholeBindings.Action(.dragEdge(.panel, extent: 400)), .dragEdge(edge: .panel, extent: 400))
        XCTAssertEqual(WormholeBindings.Action(.setWindowWidth(900)), .setWindowWidth(900))
        XCTAssertEqual(WormholeBindings.Action(.setFeature(.worktrees, false)), .setFeature(feature: .worktrees, on: false))
        XCTAssertEqual(WormholeBindings.Action(.setTitleRow(["folder", "space", "splits", "nonsense"])), .setTitleRow([.folder, .space, .splits]))
        XCTAssertEqual(
            WormholeBindings.Action(.moveTerminal(4, .group(2, before: nil), leavesSplit: true, withMates: false)),
            .moveTerminalAs(id: 4, landing: .group(group: 2, before: nil), leaveSplit: true, withMates: false))
        for part in WormholeBindings.TitlePart.allCasesForTest { XCTAssertEqual(WormholeBindings.TitlePart(key: part.key), part) }
    }

    func testTheTitleRowAndTheAllTerminalsListMapToTheViewsValues() {
        let title = TitleState(
            TitleSnapshot(
                revision: 1,
                terminal: TitleTerminal(
                    id: 2, name: "claude", agent: nil, glyph: .working, state: .working, folder: "~/app", cwd: "/Users/ref/app",
                    branch: "main", message: nil),
                left: [WormholeBindings.TitleSlot(part: .folder, present: true, chevronBefore: false), WormholeBindings.TitleSlot(part: .branchStatus, present: false, chevronBefore: false)],
                right: [WormholeBindings.TitleSlot(part: .openIn, present: true, chevronBefore: false)], hidden: [.ports],
                pathSteps: [PathStep(parents: "~/", last: "app")], pathMenu: [PathFolder(path: "/Users/ref", name: "ref")], ports: [3000],
                pullRequest: WormholeBindings.PullRequest(
                    number: 42, url: "https://github.com/o/r/pull/42", state: .open, draft: true, checks: .failing, review: nil),
                pullRequestWords: "Pull request #42, open, draft, checks failing"))
        XCTAssertEqual(title.ports, [3000])
        XCTAssertEqual(title.pullRequest?.number, 42)
        XCTAssertEqual(title.pullRequest?.checks, .failing)
        XCTAssertEqual(title.pullRequest?.words, "Pull request #42, open, draft, checks failing")
        XCTAssertEqual(title.left.map(\.part), ["folder", "branch_status"])
        XCTAssertEqual(title.left.map(\.present), [true, false])
        XCTAssertEqual(title.right.map(\.part), ["open_in"])
        XCTAssertEqual(title.hidden, ["ports"])
        XCTAssertEqual(title.pathSteps, [PathStepText(parents: "~/", last: "app")])
        XCTAssertEqual(title.terminal?.glyph, .working)
        let all = WorkspaceModel.TerminalsSnapshot(
            WormholeBindings.TerminalsSnapshot(
                revision: 4, selected: 2,
                terminals: [
                    WormholeBindings.TerminalSummary(
                        id: 2, name: "api", number: nil, selected: false, pinned: false, state: .failed, since: 0, glyph: .failed, agent: nil,
                        message: nil, branch: nil, folder: "~/api", cwd: nil, command: nil, group: 7, groupName: "Web", split: nil,
                        folded: true, agentHidden: false, forgetText: false, unread: false, bell: false, notice: 0)
                ], counts: WormholeBindings.TerminalCounts(needsYou: 1, working: 2, finished: 3, failed: 4, quiet: 5)))
        XCTAssertEqual(all.terminals.first?.group, 7)
        XCTAssertTrue(all.terminals.first?.isFolded ?? false)
        XCTAssertEqual(all.counts, TerminalCounts(needsYou: 1, working: 2, finished: 3, failed: 4, quiet: 5))
    }

    func testAPlanComesBackAsTheViewsValues() {
        let plan = WorkspaceModel.DropPlan(.move(landing: .group(group: 2, before: 5), leaveSplit: true, withMates: false))
        XCTAssertEqual(plan, .move(.group(2, before: 5), leavesSplit: true, withMates: false))
        XCTAssertEqual(WorkspaceModel.DropPlan(.move(landing: .top(before: .group(3)), leaveSplit: false, withMates: true)),
            .move(.top(before: .group(3)), leavesSplit: false, withMates: true))
        XCTAssertEqual(WorkspaceModel.DropPlan(.join(target: 8)), .join(8))
    }

}

private extension WormholeBindings.TitlePart {
    static let allCasesForTest: [Self] = [
        .folder, .branch, .branchStatus, .terminal, .terminalStatus, .space, .agentGauge, .ports, .pullRequest, .openIn, .splits,
    ]
}
