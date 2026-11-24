import DesignSystem
import Testing

/// The one distance of the one fill, and each column's own numbers when it is off.
@Suite struct ColumnMarginsTests {
    private let one = ColumnMargins(edge: 8)

    @Test func withItOffEveryColumnKeepsItsOwnNumbers() {
        let margins = ColumnMargins.own
        #expect(margins.sides(own: 10, in: .sidebar) == (10, 10))
        #expect(margins.sides(own: 10, in: .panel) == (10, 10))
        #expect(margins.top(own: 4) == 4)
        #expect(margins.bottom(own: 8, gap: 8) == 8)
        #expect(margins.value(own: 12) == 12)
    }

    @Test func withItOnTheTerminalsCardKeepsTheDistanceBesideAColumnSoTheGapIsOne() {
        #expect(one.sides(own: 10, in: .sidebar) == (8, 0), "the window's edge on the left, the terminal's card on the right")
        #expect(one.sides(own: 10, in: .panel) == (0, 8))
    }

    @Test func withItOnTheFirstCardStartsOnTheTitleRowAndTheLastEndsOneDistanceFromTheEdge() {
        #expect(one.top(own: 4) == 0)
        #expect(one.bottom(own: 0, gap: 8) == 0, "a card keeps the gap below itself already")
        #expect(one.bottom(own: 0, gap: 4) == 4, "the dense list's gap is 4: the other 4 are the list's")
        #expect(one.bottom(own: 8, gap: 16) == 0)
    }

    @Test func theSidebarListTakesItsMarginsFromThem() {
        for density in SidebarDensity.allCases {
            for cards in [true, false] {
                let own = SidebarMetrics(density: density, cards: cards)
                #expect(own.marginLeading == own.marginTrailing)
                #expect(own.listTop == 4 && own.listBottom == 0)
                let shared = SidebarMetrics(density: density, cards: cards, margins: one)
                #expect(shared.marginLeading == 8 && shared.marginTrailing == 0)
                #expect(shared.listTop == 0)
                #expect(shared.cardGap + shared.listBottom == 8, "\(density) cards \(cards): the last card ends 8 above the edge")
                #expect(shared.cardGap == own.cardGap, "the dense list keeps its 4 between its cards")
            }
        }
    }
}
