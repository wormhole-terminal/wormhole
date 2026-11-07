// How wide the side columns are drawn. The wanted widths are the user's; a narrow window takes from them so
// the terminal always keeps its minimum, the two columns sharing what is left.

import CoreGraphics

struct ColumnWidths: Equatable {
    var sidebar: CGFloat
    var panel: CGFloat

    init(sidebar wanted: CGFloat, panel panelWanted: CGFloat, window: CGFloat, sidebarVisible: Bool, panelVisible: Bool) {
        let room = max(0, window - Chrome.terminalMin)
        sidebar = min(wanted, panelVisible ? room / 2 : room)
        panel = min(panelWanted, sidebarVisible ? room - sidebar : room)
    }
}
