import Testing

import PanesModel

@Suite struct TabPanesTests {
    @Test func aSplitFocusesTheNewPaneAndClosingTheFocusedOneMovesTheKeyboardOn() {
        var tab = TabPanes.single(1)
        #expect(tab.isSplit == false)
        #expect(tab.split(.row, new: 2) && tab.focused == 2 && tab.panes == [1, 2])
        #expect(tab.split(.column, new: 3) && tab.focused == 3 && tab.panes == [1, 2, 3])
        #expect(tab.split(.row, new: 1) == false)
        #expect(tab.focus(2) && tab.close(2) && tab.focused == 3)
        #expect(tab.close(3) && tab.focused == 1 && tab.tree.count == 1)
        #expect(tab.close(1) == false)
        #expect(tab.split(.row, new: 4) && tab.focus(1) && tab.close(4) && tab.focused == 1)
        #expect(tab.focus(9) == false)
    }

    @Test func zoomFillsTheTabWithTheFocusedPaneUntilSomethingMoves() {
        var tab = TabPanes.single(1)
        #expect(tab.toggleZoom() == false)
        #expect(tab.split(.row, new: 2) && tab.toggleZoom())
        #expect(tab.zoomed == 2)
        #expect(tab.shown(width: W, height: H).map(\.rect) == [PaneRect(0, 0, W, H)])
        #expect(tab.dividers(width: W, height: H).isEmpty)
        #expect(tab.toggleZoom() && tab.zoomed == nil)
        #expect(tab.dividers(width: W, height: H).count == 1)
        #expect(tab.toggleZoom() && tab.focus(2) && tab.zoomed == 2)
        #expect(tab.focus(1) && tab.zoomed == nil)
        #expect(tab.toggleZoom() && tab.split(.column, new: 3) && tab.zoomed == nil)
        #expect(tab.toggleZoom() && tab.zoomed == 3 && tab.close(3) && tab.zoomed == nil)
    }

    @Test func theKeyboardMovesToTheNeighbourOfTheFocusedPane() {
        var tab = TabPanes.single(1)
        #expect(tab.split(.row, new: 2) && tab.split(.column, new: 3))
        let room = PaneRoom(W, H)
        #expect(tab.neighbour(.up, room: room) == 2)
        #expect(tab.neighbour(.left, room: room) == 1)
        #expect(tab.neighbour(.right, room: room) == nil)
    }

    @Test func aRestoredTabFitsItsLayoutToThePanesTheStoreHas() {
        let tab = TabPanes.restored(nil, ids: [5, 6, 7])!
        #expect(tab.panes == [5, 6, 7] && tab.focused == 5 && tab.tree.preset == nil)
        #expect(TabPanes.restored(nil, ids: [5]) == .single(5))
        #expect(TabPanes.restored(nil, ids: []) == nil)
        var saved = TabPanes.single(1)
        #expect(saved.split(.column, new: 2) && saved.focus(1) && saved.toggleZoom())
        let fitted = TabPanes.restored(saved, ids: [2, 3])!
        #expect(fitted.panes == [2, 3] && fitted.focused == 2 && fitted.zoomed == nil)
        #expect(TabPanes.restored(saved, ids: [8, 9])!.panes == [8, 9])
        #expect(saved.focus(2) == true)
        #expect(TabPanes.restored(saved, ids: [1, 2]) == saved)
    }

    /// What a drop throws, nil when it works.
    private func refusal(_ body: () throws(DropError) -> Void) -> DropError? {
        do { try body() } catch { return error }
        return nil
    }

    @Test func dropsKeepTheMinimumsAndSayWhyTheyRefuse() {
        var tab = TabPanes.single(1)
        let room = PaneRoom(W, H)
        #expect(refusal { () throws(DropError) in try tab.join(2, beside: 1, zone: .right, room: room) } == nil)
        #expect(tab.focused == 2)
        #expect(refusal { () throws(DropError) in try tab.join(3, beside: 9, zone: .left, room: room) } == .unknown)
        #expect(refusal { () throws(DropError) in try tab.join(3, beside: 1, zone: .left, room: PaneRoom(300, H)) } == .tooSmall)
        #expect(refusal { () throws(DropError) in try tab.move(1, beside: 2, zone: .left, room: room) } == .unchanged)
        #expect(tab.moveRefusal(1, beside: 2, zone: .bottom, room: room) == nil)
        #expect(tab.joinRefusal(3, beside: 1, zone: .left, room: PaneRoom(300, H)) == .tooSmall)
        #expect(tab.leave(2) == .dissolved)
    }

    @Test func aPresetKeepsTheFirstPanesAndTheFocusedOneAndTheRestLeave() {
        var tab = TabPanes.single(1)
        #expect(tab.plan(.grid) == PresetPlan(keep: [1], new: 3, out: []))
        #expect(tab.split(.row, new: 2) && tab.split(.row, new: 3))
        #expect(tab.plan(.twoColumns) == PresetPlan(keep: [1, 3], new: 0, out: [2]))
        #expect(tab.plan(.single) == PresetPlan(keep: [3], new: 0, out: [1, 2]))
        #expect(tab.focus(1) == true)
        #expect(tab.plan(.twoRows) == PresetPlan(keep: [1, 2], new: 0, out: [3]))
        #expect(tab.plan(.grid) == PresetPlan(keep: [1, 2, 3], new: 1, out: []))
    }

    @Test func arrangingBuildsThePresetsShapeAndTheMenuRecognisesIt() {
        var tab = TabPanes.single(1)
        #expect(tab.tree.preset == .single)
        #expect(tab.arrange(.grid, newIDs: [2, 3]) == nil)
        #expect(tab.arrange(.grid, newIDs: [2, 3, 4, 5]) == [])
        #expect(tab.tree == grid() && tab.tree.preset == .grid && tab.focused == 1)
        #expect(tab.shown(width: W, height: H).allSatisfy { $0.rect.w == 400 && $0.rect.h == 300 })
        #expect(tab.focus(4) && tab.toggleZoom())
        #expect(tab.arrange(.twoColumns, newIDs: []) == [2, 3])
        #expect(tab.panes == [1, 4] && tab.focused == 4 && tab.zoomed == nil)
        #expect(tab.arrange(.twoRows, newIDs: []) == [] && tab.tree.preset == .twoRows)
        #expect(tab.arrange(.single, newIDs: []) == [1] && tab.panes == [4])
        let rows = tree(split(.column, [(0.3, split(.row, [(0.2, leaf(1)), (0.8, leaf(2))])), (0.7, split(.row, [(0.5, leaf(3)), (0.5, leaf(4))]))]))
        #expect(rows.preset == .grid)
        #expect(Preset.allCases.map(\.title) == ["2 Columns", "2 Rows", "2×2 Grid", "Single Pane"])
    }
}
