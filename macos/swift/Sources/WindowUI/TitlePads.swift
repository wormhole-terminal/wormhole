// The terminal's title row's padding at its two ends, ported from `window/chrome.rs`. Beside an open column
// the breadcrumb starts a small gap from its hairline; at the window's edge it makes room for the lights and
// the collapse button, or the panel toggle. While a column slides the row's content stays clear of the button
// floating at that side until the column's edge reaches it, then rides the edge, and each end meets the
// padding beside the column when it lands, so nothing jumps.

import CoreGraphics

struct TitlePads: Equatable {
    var leading: CGFloat
    var trailing: CGFloat

    /// With both columns out of the layout, or in it.
    static func at(rest facts: WindowFacts, sidebarInFlow: Bool, panelInFlow: Bool) -> TitlePads {
        TitlePads(
            leading: sidebarInFlow ? Chrome.besideColumn : facts.collapseSlot,
            trailing: panelInFlow ? Chrome.besidePanel : Chrome.panelToggleSlot)
    }

    /// `edge`'s column is `offset` points in, from the pads at rest with that column out of the layout.
    func sliding(_ edge: PaneLayout.Edge, offset: CGFloat) -> TitlePads {
        switch edge {
        case .sidebar:
            TitlePads(leading: max(leading - offset, Chrome.besideColumn), trailing: trailing + offset)
        case .panel:
            TitlePads(leading: leading, trailing: max(trailing, offset + Chrome.besidePanel))
        }
    }
}
