// Where everything of the window's body is at one moment of a slide, from how far in each column is. Pure: the
// numbers `window/layout.rs` and `window/slide.rs` arrive at with their divs, so a test can pin them. A column
// that comes in lies over the terminal and joins the layout when it is in; one that goes leaves at once. The
// sidebar's edge carries the terminal (shifted, not resized), the panel's edge cuts it off, and the terminal
// is never sized until a column has landed: its PTY resizes once.

import CoreGraphics

struct PaneFrames: Equatable {
    /// How the columns stand: their widths at rest, how far in each is (0...1), whether it is in the layout
    /// and whether it is sliding now.
    struct Column: Equatable {
        var width: CGFloat
        var shown: CGFloat
        var inFlow: Bool
        var sliding: Bool

        /// How many points of the column are in.
        var visible: CGFloat { width * shown }
        /// How far a slide has brought the column's edge; none at rest.
        var offset: CGFloat { sliding ? visible : 0 }
        var inLayout: CGFloat { inFlow ? width : 0 }
    }

    /// The terminal column's own width: the window less what is in the layout beside it.
    let contentWidth: CGFloat
    /// Where its slot starts and how much of it shows; what shows is clipped, never resized.
    let slotX: CGFloat
    let slotVisible: CGFloat
    /// How far the sidebar's edge has carried the terminal.
    let carry: CGFloat
    let pads: TitlePads
    /// The sidebar's and panel's clips, from their window edges.
    let sidebarVisible: CGFloat
    let panelVisible: CGFloat
    /// Where each column's hairline lies (its left edge); nil while there is none to draw.
    let sidebarRule: CGFloat?
    let panelRule: CGFloat?

    init(window: CGFloat, sidebar: Column, panel: Column, facts: WindowFacts, ruleWidth: CGFloat) {
        let slot = max(window - sidebar.inLayout - panel.inLayout, PaneLayout.terminalMin)
        contentWidth = slot
        slotX = sidebar.inLayout
        carry = sidebar.offset
        slotVisible = slot - panel.offset
        var pads = TitlePads.at(rest: facts, sidebarInFlow: sidebar.inFlow, panelInFlow: panel.inFlow)
        if sidebar.sliding { pads = pads.sliding(.sidebar, offset: sidebar.offset) }
        if panel.sliding { pads = pads.sliding(.panel, offset: panel.offset) }
        self.pads = pads
        sidebarVisible = sidebar.visible
        panelVisible = panel.visible
        // Hairlines only lie along a column that is in: a sliding one paints its own surface over the blur.
        sidebarRule = ruleWidth > 0 && sidebar.inFlow && !sidebar.sliding && sidebar.width > 0 ? sidebar.width : nil
        panelRule = ruleWidth > 0 && panel.inFlow && !panel.sliding && panel.width > 0 ? window - panel.width - ruleWidth : nil
    }
}
