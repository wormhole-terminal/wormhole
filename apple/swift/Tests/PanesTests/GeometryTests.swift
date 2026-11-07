import Testing

import PanesModel

@Suite struct GeometryTests {
    @Test func panesGetTheirShareOfTheTabAndTheLastEdgeIsTheTabsOwn() {
        let t = tree(split(.row, [(0.25, leaf(1)), (0.75, split(.column, [(0.5, leaf(2)), (0.5, leaf(3))]))]))
        let rects = t.rects(width: W, height: H)
        #expect(rects[0].rect == PaneRect(0, 0, 200, 600))
        #expect(rects[1].rect == PaneRect(200, 0, 600, 300))
        #expect(rects[2].rect == PaneRect(200, 300, 600, 300))
        let rounded = tree(split(.row, [(0.33333, leaf(1)), (0.33333, leaf(2)), (0.33333, leaf(3))]))
        #expect(rounded.rects(width: W, height: H)[2].rect.right == W)
    }

    @Test func dividersLieOnTheSharedEdgesSplitsBeforeTheirChildren() {
        let t = tree(split(.row, [(0.25, leaf(1)), (0.75, split(.column, [(0.5, leaf(2)), (0.5, leaf(3))]))]))
        let dividers = t.dividers(width: W, height: H)
        #expect(dividers.count == 2)
        #expect(dividers[0].id == divider([], 0))
        #expect(dividers[0].rect == PaneRect(200, 0, 0, 600) && dividers[0].position == 200)
        #expect(dividers[1].id == divider([1], 0))
        #expect(dividers[1].rect == PaneRect(200, 300, 600, 0) && dividers[1].position == 300)
        #expect(PaneTree.single(1).dividers(width: W, height: H).isEmpty)
    }

    @Test func draggingADividerMovesOnlyTheTwoPanesItSeparates() {
        var t = tree(split(.row, [(0.25, leaf(1)), (0.25, leaf(2)), (0.5, leaf(3))]))
        #expect(t.drag(divider([], 0), to: 220, width: W, height: H) == 220)
        #expect(shares(t) == [0.275, 0.225, 0.5])
        let second = t.rects(width: W, height: H)[1].rect
        #expect(abs(second.x - 220) < 1e-9 && abs(second.w - 180) < 1e-9 && second.h == 600)
    }

    @Test func aDragStopsWhereAPaneWouldGetTooSmall() {
        func drag(_ t: inout PaneTree, _ id: DividerID, _ to: Double) -> Double? { t.drag(id, to: to, width: W, height: H) }
        var t = tree(split(.row, [(0.5, leaf(1)), (0.5, leaf(2))]))
        #expect(drag(&t, divider([], 0), 20) == PaneMin.width)
        #expect(drag(&t, divider([], 0), 790) == W - PaneMin.width)
        var nested = tree(split(.row, [(0.5, split(.column, [(0.5, split(.row, [(0.5, leaf(1)), (0.5, leaf(2))])), (0.5, leaf(5))])), (0.5, leaf(3))]))
        #expect(drag(&nested, divider([], 0), 100) == 2 * PaneMin.width)
        var three = tree(split(.row, [(0.25, leaf(1)), (0.25, leaf(2)), (0.5, leaf(3))]))
        #expect(drag(&three, divider([], 0), 300) == 400 - PaneMin.width)
        var rows = tree(split(.column, [(0.5, split(.row, [(0.5, leaf(1)), (0.5, leaf(2))])), (0.5, leaf(3))]))
        #expect(drag(&rows, divider([], 0), 10) == PaneMin.height)
        #expect(drag(&rows, divider([], 0), 590) == H - PaneMin.height)
        #expect(drag(&rows, divider([0], 0), 50) == PaneMin.width)
    }

    @Test func aDragInATabTooSmallForBothMinimumsChangesNothing() {
        var t = tree(split(.row, [(0.5, leaf(1)), (0.5, leaf(2))]))
        #expect(t.drag(divider([], 0), to: 100, width: 200, height: H) == nil)
        #expect(shares(t) == [0.5, 0.5])
        #expect(t.drag(divider([], 5), to: 100, width: W, height: H) == nil)
        #expect(t.drag(divider([3], 0), to: 100, width: W, height: H) == nil)
        var single = PaneTree.single(1)
        #expect(single.drag(divider([], 0), to: 100, width: W, height: H) == nil)
    }

    @Test func aDoubleClickEqualizesTheDividersSplitAndThatOneOnly() {
        var t = tree(split(.row, [(0.7, leaf(1)), (0.3, split(.column, [(0.8, leaf(2)), (0.2, leaf(3))]))]))
        #expect(t.equalize(divider([1], 0)) == true)
        #expect(shares(t) == [0.7, 0.3])
        #expect(t.rects(width: W, height: H)[1].rect.h == 300)
        #expect(t.equalize(divider([], 0)) == true)
        #expect(shares(t) == [0.5, 0.5])
        #expect(t.equalize(divider([], 1)) == false)
    }

    @Test func theNeighbourInADirectionSharesAnEdgeAndOverlapsMost() {
        let g = grid()
        func next(_ from: PaneID, _ d: Direction) -> PaneID? { g.neighbour(of: from, d, width: W, height: H) }
        #expect(next(1, .right) == 2 && next(1, .down) == 3)
        #expect(next(1, .left) == nil && next(1, .up) == nil)
        #expect(next(4, .up) == 2 && next(4, .left) == 3)
        #expect(next(9, .left) == nil)
        let uneven = tree(split(.row, [(0.5, leaf(1)), (0.5, split(.column, [(0.3, leaf(2)), (0.7, leaf(3))]))]))
        #expect(uneven.neighbour(of: 1, .right, width: W, height: H) == 3)
        #expect(uneven.neighbour(of: 2, .left, width: W, height: H) == 1)
        let tie = tree(split(.row, [(0.5, split(.column, [(0.2, leaf(1)), (0.8, leaf(2))])), (0.5, split(.column, [(0.5, leaf(3)), (0.5, leaf(4))]))]))
        #expect(tie.neighbour(of: 2, .right, width: W, height: H) == 4)
        #expect(tie.neighbour(of: 3, .left, width: W, height: H) == 2)
    }

    @Test func aShareIsSetOutOfTheNeighbourAndNothingElseMoves() {
        let room = PaneRoom(W, H)
        func percent(_ value: Double?) -> Double? { value.map { ($0 * 100).rounded() / 100 } }
        var panes = TabPanes.single(1)
        _ = panes.split(.row, new: 2)
        let thirty = percent(panes.setShare(of: 2, beside: 1, percent: 30, room: room))
        #expect(thirty == 30 && shares(panes.tree) == [0.7, 0.3])
        var three = TabPanes.restored(nil, ids: [1, 2, 3])!
        let twenty = percent(three.setShare(of: 2, beside: 1, percent: 20, room: room))
        #expect(twenty == 20 && shares(three.tree) == [0.467, 0.2, 0.333])
        let before = three
        let notNeighbours = three.setShare(of: 3, beside: 1, percent: 30, room: room)
        let itself = three.setShare(of: 1, beside: 1, percent: 30, room: room)
        let tooSmall = three.setShare(of: 1, beside: 2, percent: 30, room: PaneRoom(300, H))
        #expect(notNeighbours == nil && itself == nil && tooSmall == nil)
        #expect(three == before)
    }
}
