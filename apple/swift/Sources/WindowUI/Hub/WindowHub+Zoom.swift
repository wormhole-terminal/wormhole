// Text zoom (⌘+ ⌘- ⌘0) is the window's: every terminal of it zooms, and one made or moved in later adopts it
// (`TerminalViews.zoom`; `window_actions::zoom_font`). Each eases in its view, and once it has arrived the program is told
// the grid and the size chip says it (`SizeReadoutModel.textZoomed`), for the terminals in sight. Not saved, as in the GPUI app.

import TerminalCore
import TerminalMetal

extension WindowHub {
    /// What the size chip of each pane in sight says (the control folder's `size-chip`): `id: text`, or `none` before it has spoken.
    public var sizeChips: [String] {
        state.panes.shown.map(\.id).map { id in "\(id): \(overlay(for: id)?.size.text ?? "none")" }
    }

    /// A point bigger (`1`) or smaller (`-1`); `nil` is the size in Settings.
    public func zoomText(step: Double?) {
        guard let id = state.selected, let selected = terminals.view(for: id) else { return }
        let zoom = step.map { TextZoom.zoomed(base: selected.fontSettings.pointSize, zoom: terminals.zoom, step: $0) } ?? 0
        guard zoom != terminals.zoom else { return }
        terminals.zoom = zoom
        let shown = Set(state.panes.shown.map(\.id)).union([id])
        for other in terminals.ids {
            guard let view = terminals.existing(other), let overlay = overlay(for: other) else { continue }
            view.onZoomSettled = { [weak overlay] cols, rows in overlay?.size.noteSize(cols: cols, rows: rows, live: false) }
            overlay.size.textZoomed(onScreen: shown.contains(other))
            view.setZoom(zoom)
        }
    }
}
