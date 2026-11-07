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

        override func mouseDown(with event: NSEvent) {
            if event.clickCount == 2 {
                window?.performZoom(nil)
            } else {
                window?.performDrag(with: event)
            }
        }
    }
}
