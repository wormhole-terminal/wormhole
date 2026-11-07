// The window itself: a unified, transparent title bar over a full-size content view, so the sidebar's
// material and the terminal reach the top edge and the title rows (SwiftUI) sit beside the traffic lights.

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

    /// The title rows are 38 pt, taller than the system's 28 pt bar, so the lights would sit above the
    /// rows' centre. Growing the bar's container makes AppKit centre them in it again.
    private func centreTrafficLights() {
        guard let window, !window.styleMask.contains(.fullScreen),
            let container = window.standardWindowButton(.closeButton)?.superview?.superview
        else { return }
        container.frame.size.height = Chrome.titleHeight
        container.frame.origin.y = window.frame.height - Chrome.titleHeight
    }
}
