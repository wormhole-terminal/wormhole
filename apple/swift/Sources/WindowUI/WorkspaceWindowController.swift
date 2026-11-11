// The window itself: a transparent, non-opaque window with a unified, transparent title bar over a full-size
// content view, so the surfaces and the terminal reach the top edge and the title rows (SwiftUI) sit beside
// the traffic lights, which `TrafficLights` places where the GPUI app does.

import AppKit
import SwiftUI
import TerminalCore

@MainActor
public final class WorkspaceWindowController: NSWindowController, NSWindowDelegate {
    /// Called once, when the window is closing, so the owner can shut down what it started.
    public var onClose: (() -> Void)?
    /// Called when the window comes to the front or goes behind (`active`) or can no longer be seen (`seen`: not
    /// miniaturised, not hidden, not covered): the workspace looks at what is on screen only while it is in front.
    public var onState: ((_ active: Bool, _ seen: Bool) -> Void)?
    /// Called when the window moved, resized, went into the Dock or out, zoomed or changed full screen: its place is new.
    public var onFrameChange: (() -> Void)?
    /// Asked when the person closes the window: false keeps it open (the app asks a question first, or quits instead).
    public var shouldClose: (() -> Bool)?
    /// The cell of the one terminal that fills the window and the strip its area has beyond the grid, both in points; nil
    /// where a split's panes share no grid. Asked when a drag starts and ends (`view::motion::live_resize`).
    public var gridFacts: (() -> (cell: CGSize, spare: CGSize)?)?
    /// The frame a window drag began at: what tells the edges the person held.
    private var dragStart = NSRect.zero
    /// The grid's cell and strip when the drag began.
    private var dragFacts: (cell: CGSize, spare: CGSize)?
    /// The rectangle the window had before it was zoomed or made full screen: what a record of it comes back to.
    public private(set) var windowedFrame = NSRect.zero

    public init<Content: View>(title: String, contentSize: CGSize, minSize: CGSize, content: Content) {
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
        window.contentMinSize = minSize
        window.isReleasedWhenClosed = false
        let hosting = NSHostingView(rootView: content)
        // The window's size is the user's; the SwiftUI layout must not resize it as columns collapse.
        hosting.sizingOptions = []
        window.contentView = hosting
        window.center()
        super.init(window: window)
        windowedFrame = window.frame
        window.delegate = self
        centreTrafficLights()
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

    public func windowShouldClose(_ sender: NSWindow) -> Bool { shouldClose?() ?? true }
    public func windowWillClose(_ notification: Notification) { onClose?() }
    public func windowDidBecomeKey(_ notification: Notification) { reportState() }
    public func windowDidResignKey(_ notification: Notification) { reportState() }
    public func windowDidChangeOcclusionState(_ notification: Notification) { reportState() }
    public func windowDidMiniaturize(_ notification: Notification) {
        reportState()
        onFrameChange?()
    }
    public func windowDidDeminiaturize(_ notification: Notification) {
        reportState()
        onFrameChange?()
    }
    /// Whether a drag of the window's edge steps by the terminal's cell (the default) or resizes freely (`smooth_resize`).
    public var snapsToGrid: () -> Bool = { true }

    public func windowWillStartLiveResize(_ notification: Notification) {
        dragStart = window?.frame ?? .zero
        dragFacts = snapsToGrid() ? gridFacts?() : nil
    }

    /// A drag steps by the terminal's cell, so no part of a cell is left at the right or bottom (as Terminal does). Every step
    /// counts from where the drag began and the strip the grid had then: the layout inside the window follows the frame a
    /// beat later, so what it holds mid-drag is not the window's size now.
    public func windowWillResize(_ sender: NSWindow, to proposed: NSSize) -> NSSize {
        guard snapsToGrid(), let facts = dragFacts ?? gridFacts?() else { return proposed }
        let least = sender.frameRect(forContentRect: NSRect(origin: .zero, size: sender.contentMinSize)).size
        let from = dragFacts == nil ? sender.frame.size : dragStart.size
        return ResizeSteps.snapped(proposed, from: from, spare: facts.spare, cell: facts.cell, least: least)
    }

    /// A window may still end off the cell grid (the columns gave way as it narrowed): it gives up the strip, from the side
    /// the person was not holding. Full screen, zoomed and tiled windows are the system's size and stay.
    public func windowDidEndLiveResize(_ notification: Notification) {
        dragFacts = nil
        guard let window, snapsToGrid(), !window.isZoomed, !window.styleMask.contains(.fullScreen), let facts = gridFacts?(),
            let strip = ResizeSteps.settle(spare: facts.spare, cell: facts.cell), let visible = window.screen?.visibleFrame,
            let settled = ResizeSteps.settledFrame(
                window.frame, strip: strip, visible: visible, heldLeft: abs(window.frame.minX - dragStart.minX) > 0.5,
                heldTop: abs(window.frame.maxY - dragStart.maxY) > 0.5)
        else { return }
        window.setFrame(settled, display: true)
    }

    public func windowDidMove(_ notification: Notification) { frameChanged() }
    public func windowDidResize(_ notification: Notification) {
        centreTrafficLights()
        frameChanged()
    }
    public func windowDidExitFullScreen(_ notification: Notification) {
        centreTrafficLights()
        frameChanged()
    }
    public func windowDidEnterFullScreen(_ notification: Notification) { onFrameChange?() }

    /// The last place the window was while it was an ordinary window, then the news.
    private func frameChanged() {
        if let window, !window.isZoomed, !window.styleMask.contains(.fullScreen), !window.isMiniaturized {
            windowedFrame = window.frame
        }
        onFrameChange?()
    }

    /// What the workspace is told: in front is the key window, seen is on screen, not miniaturised and not covered.
    public func reportState() {
        guard let window else { return }
        onState?(window.isKeyWindow, !window.isMiniaturized && window.occlusionState.contains(.visible))
    }

    private func centreTrafficLights() {
        if let window { TrafficLights.place(in: window) }
    }
}
