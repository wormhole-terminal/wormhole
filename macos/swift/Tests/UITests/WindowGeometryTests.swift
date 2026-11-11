import CoreGraphics
import Testing

@testable import WindowUI

/// The numbers `window/layout.rs`, `window/chrome.rs` and `view/layout` arrive at, so the Swift window cannot
/// drift from the GPUI app's.
@Suite struct WindowGeometryTests {
    private let facts = WindowFacts()

    private func column(_ width: CGFloat, shown: CGFloat = 1, inFlow: Bool = true, sliding: Bool = false) -> PaneFrames.Column {
        PaneFrames.Column(width: width, shown: shown, inFlow: inFlow, sliding: sliding)
    }

    @Test func theCollapseButtonSitsAfterTheAssumedLights() {
        #expect(facts.collapseX == 76)
        let slot: CGFloat = 112
        #expect(facts.collapseSlot == slot)
        // Wider lights push it right, keeping the 12 pt gap; full screen puts its glyph on the 12 pt edge.
        #expect(WindowFacts(isFullScreen: false, lightsEnd: 82).collapseX == 94)
        #expect(WindowFacts(isFullScreen: true).collapseX == 6)
    }

    @Test func panelYieldsFirstThenHides() {
        let layout = PaneLayout()
        #expect(layout.columns(window: 1320) == PaneLayout.Columns(sidebar: 350, terminal: 610, panel: 360))
        #expect(layout.columns(window: 1000).panel == 330)
        #expect(!layout.panelVisible(window: 900))
        #expect(layout.columns(window: 900).sidebar == 350)
    }

    @Test func aDragStopsAtTheLimitsAndClosesPastTheSlack() {
        let range = PaneLayout.panelRange
        #expect(ColumnResize(extent: 500, range: range, max: 430).stop == .max)
        #expect(ColumnResize(extent: 500, range: range, max: 430).readout(closing: "Close") == "430 · max")
        #expect(ColumnResize(extent: 200, range: range, max: 430).stop == .min)
        #expect(ColumnResize(extent: 160, range: range, max: 430).open == false)
        #expect(ColumnResize(extent: 160, range: range, max: 430).readout(closing: "Close") == "Close")
    }

    @Test func titlePadsFollowASlidingColumnToWhereItLands() {
        let rest = TitlePads.at(rest: facts, sidebarInFlow: false, panelInFlow: false)
        #expect(rest == TitlePads(leading: 112, trailing: 34))
        #expect(rest.sliding(.sidebar, offset: 0) == rest)
        #expect(rest.sliding(.sidebar, offset: 350) == TitlePads(leading: 4, trailing: 34 + 350))
        #expect(rest.sliding(.sidebar, offset: 20).leading + 20 == rest.leading)
        #expect(rest.sliding(.panel, offset: 360) == TitlePads(leading: 112, trailing: 366))
    }

    @Test func aSidebarComingInCarriesTheTerminalWithoutResizingIt() {
        let half = PaneFrames(
            window: 1320, sidebar: column(350, shown: 0.5, inFlow: false, sliding: true), panel: column(360), facts: facts,
            ruleWidth: 1)
        // Laid out as if the sidebar were not there, shifted by what is in, clipped to its place.
        let content: CGFloat = 960
        #expect(half.contentWidth == content)
        #expect(half.carry == 175 && half.slotX == 0 && half.slotVisible == half.contentWidth)
        #expect(half.sidebarEdge == nil && half.panelEdge == 960)
        let landed = PaneFrames(window: 1320, sidebar: column(350), panel: column(360), facts: facts, ruleWidth: 1)
        #expect(landed.contentWidth == 610 && landed.slotX == 350 && landed.carry == 0)
        #expect(landed.sidebarEdge == 350)
    }

    @Test func aPanelCutsTheTerminalOffInsteadOfSizingIt() {
        let half = PaneFrames(
            window: 1320, sidebar: column(350), panel: column(360, shown: 0.5, inFlow: false, sliding: true), facts: facts,
            ruleWidth: 1)
        #expect(half.contentWidth == 970 && half.slotVisible == 970 - 180)
        #expect(half.panelVisible == 180)
    }
}
