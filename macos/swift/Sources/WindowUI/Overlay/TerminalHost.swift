// One pane's terminal: a plain container whose only subview is that terminal's Metal view. A terminal that is
// not on screen leaves the window (its display link pauses with it) but stays alive in `TerminalViews`. The
// container tells the pane's overlay the grid it holds after each layout, which is what the size readout
// says during a live resize.

import AppKit
import SwiftUI
import TerminalMetal
import WordsModel
import WorkspaceModel

struct TerminalHost: NSViewRepresentable {
    let views: TerminalViews
    let id: TerminalID
    let overlay: TerminalOverlay
    /// The window's opacity (`WindowLook.effectiveOpacity`), passed in so a change of it updates the view.
    let opacity: CGFloat
    let menu: @MainActor () -> NSMenu?

    func makeNSView(context: Context) -> Container {
        let host = Container(overlay: overlay, menu: menu)
        host.setAccessibilityRole(.group)
        host.setAccessibilityLabel(Words.text("window-terminal", fallback: "Terminal"))
        return host
    }

    func updateNSView(_ host: Container, context: Context) {
        guard let wanted = views.view(for: id) else { return host.show(nil) }
        host.show(wanted, takesKeys: { views.takesKeys() })
        // The pane is painted at the window's opacity, as every surface is (`WindowLook.surface`).
        wanted.backgroundOpacity = Double(opacity)
    }

    static func dismantleNSView(_ host: Container, coordinator: ()) { host.show(nil) }

    final class Container: NSView {
        private let overlay: TerminalOverlay
        private let contextMenu: @MainActor () -> NSMenu?

        init(overlay: TerminalOverlay, menu: @escaping @MainActor () -> NSMenu?) {
            self.overlay = overlay
            contextMenu = menu
            super.init(frame: .zero)
        }

        @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

        func show(_ view: MetalTerminalView?, takesKeys: () -> Bool = { true }) {
            guard subviews.first !== view else { return }
            subviews.forEach { $0.removeFromSuperview() }
            guard let view else { return }
            view.removeFromSuperview()
            view.frame = bounds
            view.autoresizingMask = [.width, .height]
            view.contextMenu = { [weak self] in self?.contextMenu() }
            addSubview(view)
            // The keys go to the terminal that was just shown, not to whatever had focus (unless the sidebar's Space browses).
            if takesKeys() { window?.makeFirstResponder(view) }
        }

        /// The terminal's right-click: the native menu of its grid.
        override func menu(for event: NSEvent) -> NSMenu? { contextMenu() }

        override func layout() {
            super.layout()
            guard let view = subviews.first as? MetalTerminalView else { return }
            let grid = view.gridSize
            let live = (window?.inLiveResize ?? false) || LiveResizeDriving.simulated
            // After the update that caused this layout: the overlay is observed by the views being laid out.
            DispatchQueue.main.async { [overlay] in overlay.size.noteSize(cols: grid.cols, rows: grid.rows, live: live) }
        }
    }
}

/// The control folder's `live-resize-sim` steps the window as a drag does, which AppKit does not call a live resize: the size chip
/// is told it is one while the script holds this.
@MainActor
public enum LiveResizeDriving {
    public static var simulated = false
}
