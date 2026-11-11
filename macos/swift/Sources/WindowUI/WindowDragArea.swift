// Title rows are SwiftUI views over a full-size content view, so the system title bar no longer receives
// their clicks. This backing view hands a press to the window as a drag and a double click to zoom, as a
// title bar does.

import AppKit
import SwiftUI

struct WindowDragArea: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView { DragView() }
    func updateNSView(_ view: NSView, context: Context) {}

    private final class DragView: NSView {
        override var mouseDownCanMoveWindow: Bool { true }

        /// What System Settings says a double click on a title bar does ("Double-click a window's title bar to": Zoom, Minimize or
        /// nothing), as the system's own title bar reads it (`AppleActionOnDoubleClick`; older systems: `AppleMiniaturizeOnDoubleClick`).
        private static func doubleClick(_ window: NSWindow?) {
            let defaults = UserDefaults.standard
            switch defaults.string(forKey: "AppleActionOnDoubleClick") {  // text: a defaults key
            case "Minimize": window?.performMiniaturize(nil)
            case "None": break
            case nil where defaults.bool(forKey: "AppleMiniaturizeOnDoubleClick"): window?.performMiniaturize(nil)  // text: a defaults key
            default: window?.performZoom(nil)
            }
        }

        override func mouseDown(with event: NSEvent) {
            if event.clickCount == 2 {
                Self.doubleClick(window)
            } else {
                window?.performDrag(with: event)
            }
        }
    }
}
