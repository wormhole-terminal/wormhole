// The window itself: a transparent, non-opaque window with a unified, transparent title bar over a full-size
// content view, so the surfaces and the terminal reach the top edge and the title rows (SwiftUI) sit beside
// the traffic lights, which `TrafficLights` places where the GPUI app does.

import AppKit
import SwiftUI

@MainActor
public final class WorkspaceWindowController: NSWindowController, NSWindowDelegate {
    /// Called once, when the window is closing, so the owner can shut down what it started.
    public var onClose: (() -> Void)?

    public init<Content: View>(title: String, contentSize: CGSize, content: Content) {
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: contentSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered, defer: false)
        window.title = title  // hidden, but Mission Control and VoiceOver read it
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        // Transparent like the GPUI app's: the blur and the fill behind the surfaces are the content's own
        // (`WindowBackdrop`), so a translucent theme shows the desktop and an opaque one does not.
        window.isOpaque = false
        window.backgroundColor = .clear
        window.minSize = NSSize(width: 640, height: 360)
        window.isReleasedWhenClosed = false
        let hosting = NSHostingView(rootView: content)
        // The window's size is the user's; the SwiftUI layout must not resize it as columns collapse.
        hosting.sizingOptions = []
        window.contentView = hosting
        window.center()
        super.init(window: window)
        window.delegate = self
        centreTrafficLights()
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    public func windowWillClose(_ notification: Notification) { onClose?() }
    public func windowDidResize(_ notification: Notification) { centreTrafficLights() }
    public func windowDidExitFullScreen(_ notification: Notification) { centreTrafficLights() }

    private func centreTrafficLights() {
        if let window { TrafficLights.place(in: window) }
    }
}
