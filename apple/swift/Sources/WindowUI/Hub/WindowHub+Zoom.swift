// Text zoom for the selected terminal (⌘+ ⌘- ⌘0): it eases in the view, and once it has arrived the program is told
// the grid and the size chip says it (`SizeReadoutModel.textZoomed`). Not saved, as in the GPUI app.

import TerminalMetal

extension WindowHub {
    /// A point bigger (`1`) or smaller (`-1`); `nil` is the size in Settings.
    public func zoomText(step: Double?) {
        guard let id = state.selected, let view = terminals.view(for: id), let overlay = overlay(for: id) else { return }
        view.onZoomSettled = { [weak overlay] cols, rows in overlay?.size.noteSize(cols: cols, rows: rows, live: false) }
        overlay.size.textZoomed(onScreen: true)
        if let step { view.zoom(step: step) } else { view.resetZoom() }
    }
}
