import Testing

import PanesModel

@Suite struct TreeTests {
    @Test func splitRightPutsTheNewPaneBesideAndSplitDownBelowEachTakingHalf() {
        var t = PaneTree.single(1)
        #expect(t.split(1, axis: .row, new: 2) == true)
        #expect(t.root == split(.row, [(0.5, leaf(1)), (0.5, leaf(2))]))
        #expect(t.split(2, axis: .column, new: 3) == true)
        #expect(t.root == split(.row, [(0.5, leaf(1)), (0.5, split(.column, [(0.5, leaf(2)), (0.5, leaf(3))]))]))
        #expect(t.panes == [1, 2, 3])
        #expect(t.count == 3)
    }

    @Test func splittingAlongTheParentsAxisAddsASiblingOutOfTheTargetsShare() {
        var t = tree(split(.row, [(0.6, leaf(1)), (0.4, leaf(2))]))
        #expect(t.split(1, axis: .row, new: 3) == true)
        #expect(t.root == split(.row, [(0.3, leaf(1)), (0.3, leaf(3)), (0.4, leaf(2))]))
        #expect(t.panes == [1, 3, 2])
    }

    @Test func aSplitRefusesAnUnknownTargetAndAPaneThatIsHereAlready() {
        var t = PaneTree.single(1)
        #expect(t.split(9, axis: .row, new: 2) == false)
        #expect(t.split(1, axis: .row, new: 1) == false)
        #expect(t == .single(1))
    }

    @Test func closingAPaneGivesItsShareToItsSiblingsInProportion() {
        var t = tree(split(.row, [(0.5, leaf(1)), (0.3, leaf(2)), (0.2, leaf(3))]))
        #expect(t.remove(1) == true)
        #expect(shares(t) == [0.6, 0.4])
        #expect(t.remove(3) == true)
        #expect(t == .single(2))
        #expect(t.remove(2) == false)
        #expect(t.remove(7) == false)
    }

    @Test func closingAPaneMergesASplitLeftInsideOneOfItsOwnAxis() {
        var t = tree(split(.row, [(0.5, leaf(1)), (0.5, split(.column, [(0.5, leaf(2)), (0.5, split(.row, [(0.5, leaf(3)), (0.5, leaf(4))]))]))]))
        #expect(t.remove(2) == true)
        #expect(t.root == split(.row, [(0.5, leaf(1)), (0.25, leaf(3)), (0.25, leaf(4))]))
    }

    @Test func readingOrderIsLeftToRightThenTopToBottomWhateverTheShares() {
        #expect(grid().panes == [1, 2, 3, 4])
        var uneven = grid()
        #expect(uneven.drag(divider([], 0), to: 100, width: W, height: H) != nil)
        #expect(uneven.drag(divider([1], 0), to: 500, width: W, height: H) != nil)
        #expect(uneven.panes == [1, 2, 3, 4])
    }

    @Test func appendingPutsAPaneAtTheRightEndWithAnEqualShare() {
        var t = PaneTree.single(1)
        #expect(t.append(2) == true)
        #expect(t.root == split(.row, [(0.5, leaf(1)), (0.5, leaf(2))]))
        #expect(t.append(3) == true)
        #expect(t.panes == [1, 2, 3])
        #expect(shares(t).allSatisfy { abs($0 - 1.0 / 3) < 0.001 })
        #expect(t.append(2) == false)
        var column = tree(split(.column, [(0.5, leaf(1)), (0.5, leaf(2))]))
        #expect(column.append(3) == true)
        #expect(column.panes == [1, 3, 2])
    }

    @Test func aDamagedRecordIsRepairedRatherThanRefused() {
        let dup = tree(split(.row, [(0.5, leaf(1)), (0.3, split(.column, [(1, leaf(1))])), (0.2, split(.column, [(1, leaf(2))]))]))
        guard case .split(_, let kids) = dup.root else { Issue.record("not a split"); return }
        #expect(kids.map(\.node) == [leaf(1), leaf(2)])
        #expect(abs(kids[0].ratio - 0.5 / 0.7) < 1e-9)
        let bad = tree(split(.column, [(0, leaf(1)), (-2, leaf(2)), (0, leaf(3))]))
        #expect(shares(bad).allSatisfy { abs($0 - 1.0 / 3) < 0.001 })
        #expect(shares(tree(split(.row, [(.nan, leaf(1)), (0.5, leaf(2))]))) == [0.5, 0.5])
        let nested = tree(split(.row, [(0.5, leaf(1)), (0.5, split(.row, [(0.5, leaf(2)), (0.5, leaf(3))]))]))
        #expect(nested.root == split(.row, [(0.5, leaf(1)), (0.25, leaf(2)), (0.25, leaf(3))]))
        #expect(PaneTree(repairing: .split(.row, [])) == nil)
    }

    @Test func movingAPaneToAnotherEdgeRearrangesAndRefusesWhatChangesNothing() {
        var t = tree(split(.row, [(0.5, leaf(1)), (0.5, leaf(2))]))
        // 1 is left of 2 already: only the shares would differ.
        #expect(t.move(1, beside: 2, zone: .left) == false)
        #expect(t.move(1, beside: 2, zone: .bottom) == true)
        #expect(t.root == .split(.column, [Child(0.5, .leaf(2)), Child(0.5, .leaf(1))]))
        #expect(t.move(1, beside: 1, zone: .top) == false)
        #expect(t.move(9, beside: 1, zone: .top) == false)
    }
}
