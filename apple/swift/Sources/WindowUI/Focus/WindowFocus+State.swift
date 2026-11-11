// Where the keys are, in words, for the control folder's `state`: the area, the item its cursor is on and whether the
// ring shows (the GPUI app's `focus_detail`).

import WorkspaceModel

extension WindowFocus {
    public var detail: String {
        // What the Esc ladder would close first: a find bar, a diff or file over the terminal.
        let open = (hub.selectedOverlay?.find.isOpen == true ? " +find" : "") + (hub.panel.cover != nil ? " +cover" : "") // text: control state line
        guard let area = current() else { return "none" + open }
        let item: String? =
            switch area {
            case .titleRow: ["toggle-sidebar", "new-terminal"][min(titleIndex, 1)]
            case .statusBar: hub.statusItems.indices.contains(statusIndex) ? "terminal id=\(hub.statusItems[statusIndex])" : nil // text: control state line
            case .panelTabs: hub.state.panelTab.rawValue
            default: nil
            }
        let ring = ring(area, area == .titleRow || area == .statusBar ? cursor(area) : 0) ? " ring" : "" // text: control state line
        return [area.rawValue, item].compactMap { $0 }.joined(separator: " ") + ring + open
    }
}
