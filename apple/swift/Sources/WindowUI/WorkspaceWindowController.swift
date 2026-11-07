// The window itself: a transparent, non-opaque window with a unified, transparent title bar over a full-size
// content view, so the surfaces and the terminal reach the top edge and the title rows (SwiftUI) sit beside
// the traffic lights, which `TrafficLights` places where the GPUI app does.

import AppKit
import SwiftUI

@MainActor
public final class WorkspaceWindowController: NSWindowController, NSWindowDelegate {
    /// Called once, when the window is closing, so the owner can shut down what it started.
    public var onClose: (() -> Void)?
    /// Called when the window comes to the front or goes behind (`active`) or can no longer be seen (`seen`: not
    /// miniaturised, not hidden, not covered): the workspace looks at what is on screen only while it is in front.
    public var onState: ((_ active: Bool, _ seen: Bool) -> Void)?

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
    public func windowDidBecomeKey(_ notification: Notification) { reportState() }
    public func windowDidResignKey(_ notification: Notification) { reportState() }
    public func windowDidChangeOcclusionState(_ notification: Notification) { reportState() }
    public func windowDidMiniaturize(_ notification: Notification) { reportState() }
    public func windowDidDeminiaturize(_ notification: Notification) { reportState() }
    public func windowDidResize(_ notification: Notification) { centreTrafficLights() }
    public func windowDidExitFullScreen(_ notification: Notification) { centreTrafficLights() }

    /// What the workspace is told: in front is the key window, seen is on screen, not miniaturised and not covered.
    public func reportState() {
        guard let window else { return }
        onState?(window.isKeyWindow, !window.isMiniaturized && window.occlusionState.contains(.visible))
    }

    private func centreTrafficLights() {
        if let window { TrafficLights.place(in: window) }
    }
}
