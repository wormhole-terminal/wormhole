import XCTest

@testable import WorkspaceModel

final class GroupActionTests: XCTestCase {
    private let mock = MockWorkspace.state

    private func group(_ state: WorkspaceState, _ id: String) -> TerminalGroup? {
        state.groups.first { $0.id == id }
    }
    private func sectionIDs(_ state: WorkspaceState) -> [String] { SidebarListing.sections(for: state).map(\.id) }
    private func rows(_ state: WorkspaceState, _ section: String) -> [TerminalID] {
        SidebarListing.sections(for: state).first { $0.id == section }?.rows.map(\.id) ?? []
    }

    func testIdsAreAskedForNotMirrored() {
        XCTAssertEqual(mock.nextTerminalID, 14)
        XCTAssertEqual(WorkspaceState().nextTerminalID, 1)
        XCTAssertEqual(mock.nextGroupID, "group-1")
        var state = WorkspaceState(groups: [TerminalGroup(id: "group-1", name: "a", members: []), TerminalGroup(id: "group-3", name: "c", members: [])])
        XCTAssertEqual(state.nextGroupID, "group-2")
        state.groups.removeAll()
        XCTAssertEqual(state.nextGroupID, "group-1")
    }

    func testRenameColorAndFolder() {
        XCTAssertEqual(group(reduce(mock, .renameGroup("infra", " Ops ")), "infra")?.name, "Ops")
        XCTAssertEqual(group(reduce(mock, .renameGroup("infra", "  ")), "infra")?.name, "Infra", "blank keeps the name")
        XCTAssertEqual(group(reduce(mock, .setGroupColor("infra", .red)), "infra")?.color, .red)
        XCTAssertNil(group(reduce(mock, .setGroupColor("infra", nil)), "infra")?.color)
        let withFolder = reduce(mock, .setGroupFolder("infra", " ~/ops "))
        XCTAssertEqual(group(withFolder, "infra")?.folder, "~/ops")
        XCTAssertEqual(group(withFolder, "infra")?.isPinned, true, "a folder makes the group a place")
        let cleared = reduce(withFolder, .setGroupFolder("infra", "  "))
        XCTAssertNil(group(cleared, "infra")?.folder)
        XCTAssertEqual(group(cleared, "infra")?.isPinned, true, "taking it away leaves it pinned")
        XCTAssertEqual(reduce(mock, .renameGroup("nope", "x")), mock)
    }

    func testPinningAnEmptyGroupAndLettingGoRemovesIt() {
        let made = reduce(mock, .newGroup("Later"))
        let id = made.groups.last?.id ?? ""
        let pinned = reduce(made, .pinGroup(id))
        XCTAssertEqual(group(pinned, id)?.isPinned, true)
        XCTAssertNil(group(reduce(pinned, .pinGroup(id)), id), "unpinned and empty: it goes")
        XCTAssertEqual(group(reduce(mock, .pinGroup("infra")), "infra")?.isPinned, true)
        XCTAssertNotNil(group(reduce(mock, .pinGroup("wormhole")), "wormhole"))
    }

    func testUngroupLeavesTheTerminalsWhereTheGroupStood() {
        let state = reduce(mock, .ungroup("website"))
        XCTAssertNil(group(state, "website"))
        XCTAssertEqual(sectionIDs(state), ["loose-10", "wormhole", "loose-5", "infra", "loose-13"])
        XCTAssertEqual(rows(state, "loose-5"), [5, 6, 7, 11])
    }

    func testCloseGroupEndsItsTerminalsAndTheGroup() {
        let state = reduce(mock, .closeGroup("website"))
        XCTAssertNil(group(state, "website"))
        XCTAssertEqual(state.terminals.map(\.id), [1, 2, 3, 4, 8, 9, 10, 11, 12, 13])
        XCTAssertEqual(state.selected, 1)
        let selectedGone = reduce(mock, .closeGroup("wormhole"))
        XCTAssertEqual(selectedGone.selected, 5, "the row that slides into its place")
        let pinned = reduce(reduce(mock, .pinGroup("website")), .closeGroup("website"))
        XCTAssertEqual(group(pinned, "website")?.members, [], "a pinned group stays as a head")
    }

    func testClosingTheLastTerminalOfAGroupTakesTheGroup() {
        let state = reduce(reduce(mock, .close(8)), .close(9))
        XCTAssertNil(group(state, "infra"))
        let fresh = reduce(mock, .newGroup("Empty"))
        XCTAssertNotNil(group(reduce(fresh, .close(13)), "group-1"), "only groups that lost a terminal go")
    }

    func testRemoveFromGroupStandsRightAfterIt() {
        let state = reduce(mock, .removeFromGroup(2))
        XCTAssertEqual(group(state, "wormhole")?.members, [1, 3, 4])
        XCTAssertEqual(sectionIDs(state), ["loose-10", "wormhole", "loose-2", "website", "loose-11", "infra", "loose-13"])
        XCTAssertEqual(reduce(mock, .removeFromGroup(11)), mock, "a loose terminal has no group to leave")
    }

    func testRemoveFromGroupTakesTheWholeSplit() {
        let state = reduce(mock, .removeFromGroup(5))
        XCTAssertEqual(group(state, "website")?.members, [7])
        XCTAssertEqual(rows(state, "loose-5"), [5, 6, 11])
    }

    func testNewTerminalInGroupStartsInTheGroupsFolder() {
        let state = reduce(mock, .newTerminalInGroup("website"))
        XCTAssertEqual(group(state, "website")?.members, [5, 6, 7, 14])
        XCTAssertEqual(state.terminals.last?.folder, "~/Projects/terminal/web")
        XCTAssertEqual(state.selected, 14)
        XCTAssertEqual(reduce(mock, .newTerminalInGroup("nope")), mock)
    }

    func testNewGroupWithTerminalTakesItsPlaceAndTheNextColour() {
        let state = reduce(mock, .newGroupWithTerminal(11, name: nil))
        let made = group(state, "group-1")
        XCTAssertEqual(made?.name, "Group 1")
        XCTAssertEqual(made?.members, [11])
        XCTAssertEqual(made?.color, GroupColor.inTurn[3])
        XCTAssertEqual(sectionIDs(state), ["loose-10", "wormhole", "website", "group-1", "infra", "loose-13"])
        XCTAssertEqual(group(reduce(mock, .newGroupWithTerminal(11, name: " Notes ")), "group-1")?.name, "Notes")
    }

    func testNewGroupWithTerminalBringsItsSplitAndStandsAfterItsOldGroup() {
        let state = reduce(mock, .newGroupWithTerminal(6, name: "Hero"))
        XCTAssertEqual(group(state, "website")?.members, [7])
        XCTAssertEqual(group(state, "group-1")?.members, [5, 6])
        XCTAssertEqual(sectionIDs(state).prefix(4), ["loose-10", "wormhole", "website", "group-1"])
        let emptied = reduce(reduce(mock, .newGroupWithTerminal(8, name: nil)), .newGroupWithTerminal(9, name: nil))
        XCTAssertNil(group(emptied, "infra"), "a group left empty goes")
    }

    func testNewGroupsNameThemselvesWithTheFirstFreeNumber() {
        var state = mock
        for _ in 1...2 { state = reduce(state, .newGroup("")) }
        XCTAssertEqual(state.groups.suffix(2).map(\.name), ["Group 1", "Group 2"])
        XCTAssertEqual(state.groups.suffix(2).map(\.id), ["group-1", "group-2"])
    }
}

final class GroupColorTests: XCTestCase {
    func testTheNineNamesInTheMenusOrder() {
        XCTAssertEqual(
            GroupColor.allCases.map(\.rawValue),
            ["grey", "blue", "red", "yellow", "green", "pink", "purple", "cyan", "orange"])
    }

    func testNewGroupsTakeTheNextColourInTurnAndNeverGrey() {
        XCTAssertEqual(GroupColor.inTurn.count, 8)
        XCTAssertFalse(GroupColor.inTurn.contains(.grey))
        var state = WorkspaceState()
        for _ in 0..<9 { state = reduce(state, .newGroup("")) }
        XCTAssertEqual(state.groups.map(\.color), (0..<9).map { GroupColor.inTurn[$0 % 8] })
    }

    func testOldSavedNamesStillMean() {
        XCTAssertEqual(GroupColor(key: "teal"), .cyan)
        XCTAssertEqual(GroupColor(key: "gray"), .grey)
        XCTAssertEqual(GroupColor(key: "blue"), .blue)
        XCTAssertNil(GroupColor(key: "mauve"))
    }

    func testAFolderHasItsOwnEight() {
        XCTAssertEqual(FolderColor.allCases.count, 8)
        XCTAssertTrue(FolderColor.allCases.map(\.rawValue).contains("teal"))
    }
}
