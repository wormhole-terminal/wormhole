// Title rows are SwiftUI views over a full-size content view, so the system title bar no longer receives
// their clicks. This backing view hands a press to the window as a drag and a double click to zoom, as a
// title bar does.

import AppKit
import SwiftUI

struct WindowDragArea: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView { DragView() }
    func updateNSView(_ view: NSView, context: Context) {}

    private final class DragView: NSView {
        /// The press a drag is still to start from, and where the window was at it.
        private var press: (event: NSEvent, origin: NSPoint)?

        /// A window that is not in front is dragged by its title bar too.
        override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }

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
                press = nil
                Self.doubleClick(window)
            } else if let window {
                press = (event, window.frame.origin)
                window.performDrag(with: event)
            }
        }

        /// The drag is asked for again while the pointer moves and the window has not: macOS 27.2's window server refused
        /// every drag this view asked for at the press (26 of 26, each with "no client initiated window drag trait gesture to
        /// activate for mouse" in its log), and in the same log one that another app asked for again 156 ms into a press,
        /// after the pointer's first move, was not refused. Where the drag asked for at the press is taken, the window
        /// moves and nothing more is asked.
        override func mouseDragged(with event: NSEvent) {
            guard let press, let window, window.frame.origin == press.origin else { return }
            window.performDrag(with: press.event)
        }

        override func mouseUp(with event: NSEvent) { press = nil }
    }
}
