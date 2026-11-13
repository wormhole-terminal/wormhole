// text-file: a developer tool's panel (inspect mode), English like the GPUI inspector's; never interface text
// Inspect mode (⇧⌥⌘I in a debug build, `inspect` on the control folder), after the GPUI app's `inspector/`: point at
// anything in a window and a panel says where it is built (file:line and that line of code), what it is called and how it
// is styled in the design system's terms, with the tokens it is drawn with. Hovering highlights, a click pins, the wheel
// walks out to the parents; Copy puts the note on the clipboard, to paste into a request.
//
// The elements are the ones the design marks (`.marked`, `DesignSystem/Inspect`). In a debug build the scale values an
// element's style names can be nudged (`InspectNudge`): the step shows wherever that value is used, and the note says it.

import AppKit
import DesignSystem
import Observation
import SwiftUI

@MainActor @Observable
final class Inspector {
    static let shared = Inspector()

    private(set) var isOpen = false
    private(set) var picking = false
    /// The elements under the last pick, innermost first, and which of them the panel shows.
    private(set) var chain: [InspectedElement] = []
    private(set) var level = 0

    @ObservationIgnored private var panel: NSPanel?
    @ObservationIgnored private var monitor: Any?
    @ObservationIgnored private let highlight = InspectHighlight()
    /// The windows asked for the pointer's moves while picking, to stop asking afterwards.
    @ObservationIgnored private let followed = NSHashTable<NSWindow>.weakObjects()

    /// The element in the panel.
    var shown: InspectedElement? { chain.indices.contains(level) ? chain[level] : nil }
    /// Its note; nil while nothing is picked.
    var note: String? { shown.map { InspectNote.text(for: $0) } }

    func toggle(beside window: NSWindow?) {
        if isOpen { close() } else { open(beside: window) }
    }

    func open(beside window: NSWindow?) {
        guard !isOpen else { return }
        isOpen = true
        Inspection.shared.isOn = true
        let panel = self.panel ?? makePanel()
        self.panel = panel
        if let frame = window?.frame {
            panel.setFrameTopLeftPoint(NSPoint(x: frame.maxX - panel.frame.width - Space._4, y: frame.maxY - Metric.titleRow - Space._2))
        }
        // A test instance's windows stay behind the person's (`TestInstance`).
        if TestInstance.isBackground { panel.orderBack(nil) } else { panel.orderFront(nil) }
        startPicking()
    }

    func close() {
        guard isOpen else { return }
        stopPicking()
        isOpen = false
        Inspection.shared.isOn = false
        (chain, level) = ([], 0)
        highlight.removeFromSuperview()
        panel?.orderOut(nil)
    }

    /// Hover highlights and a click pins from now on, until one does.
    func startPicking() {
        guard isOpen, monitor == nil else { return }
        picking = true
        for window in NSApp.windows where window !== panel && !window.acceptsMouseMovedEvents {
            window.acceptsMouseMovedEvents = true
            followed.add(window)
        }
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.mouseMoved, .leftMouseDown, .scrollWheel]) { [weak self] event in
            guard let self else { return event }
            return self.took(event) ? nil : event
        }
    }

    private func stopPicking() {
        picking = false
        monitor.map(NSEvent.removeMonitor)
        monitor = nil
        followed.allObjects.forEach { $0.acceptsMouseMovedEvents = false }
        followed.removeAllObjects()
    }

    /// Whether the event was the inspector's own (a pinning click, a step of the wheel) and goes no further.
    private func took(_ event: NSEvent) -> Bool {
        guard let window = event.window, window !== panel else { return false }
        switch event.type {
        case .mouseMoved:
            point(at: event.locationInWindow, in: window)
            return false
        case .leftMouseDown:
            pin(at: event.locationInWindow, in: window)
            return true
        case .scrollWheel:
            // One step out per gesture (a trackpad sends many events for one), and per notch of a wheel.
            if event.phase == .began || (event.phase == [] && event.momentumPhase == []) { walkOut() }
            return true
        default:
            return false
        }
    }

    /// The elements under `point` (window coordinates) of `window`, the innermost shown and lit up.
    func point(at point: NSPoint, in window: NSWindow) {
        (chain, level) = (Inspection.shared.elements(at: point, in: window), 0)
        light(in: window)
    }

    /// As `point`, and picking ends: the panel keeps this element until Pick.
    func pin(at point: NSPoint, in window: NSWindow) {
        self.point(at: point, in: window)
        stopPicking()
    }

    /// The element the shown one stands in, as GPUI's wheel walks out; false at the outermost.
    @discardableResult
    func walkOut() -> Bool {
        guard level + 1 < chain.count else { return false }
        level += 1
        if let window = highlight.window { light(in: window) }
        return true
    }

    /// Steps the part of the shown element's style called `property` (`padding x`) up or down; false when it has none
    /// that can be stepped. The outline keeps the frame of the pick: pick again to see the element's new one.
    @discardableResult
    func nudge(_ property: String, by direction: CGFloat) -> Bool {
        guard let shown, let part = InspectNudge.parts(of: shown.site.style).first(where: { $0.property == property }) else { return false }
        InspectNudge.step(part, by: direction)
        return true
    }

    /// Copy: the note on the clipboard (a test instance's is its own file, `DesignSystem.Pasteboard`).
    @discardableResult
    func copy() -> String? {
        guard let note else { return nil }
        Pasteboard.setText(note)
        return note
    }

    private func light(in window: NSWindow) {
        if let shown { highlight.show(shown.frame, in: window) } else { highlight.removeFromSuperview() }
    }

    private func makePanel() -> NSPanel {
        let panel = NSPanel(
            contentRect: CGRect(origin: .zero, size: InspectorView.size),
            styleMask: [.titled, .closable, .resizable, .utilityWindow, .nonactivatingPanel], backing: .buffered, defer: false)
        panel.title = "Inspector"
        panel.isFloatingPanel = true
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.contentView = NSHostingView(rootView: InspectorView(inspector: self).themed())
        // The red light ends inspect mode, as the panel's own close does.
        closing = NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: panel, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.close() }
        }
        return panel
    }

    @ObservationIgnored private var closing: (any NSObjectProtocol)?
}

/// The outline over the element the panel shows.
private final class InspectHighlight: NSView {
    /// How much of the primary fills the element under the outline.
    private static let fill = 0.12

    init() {
        super.init(frame: .zero)
        wantsLayer = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("not from a nib") }

    override func hitTest(_ point: NSPoint) -> NSView? { nil }

    /// Over `frame` (window coordinates) of `window`, above everything in it.
    func show(_ frame: CGRect, in window: NSWindow) {
        guard let content = window.contentView else { return }
        if superview !== content {
            removeFromSuperview()
            content.addSubview(self, positioned: .above, relativeTo: nil)
        }
        self.frame = content.convert(frame, from: nil)
        let primary = NSColor(ThemeStore.shared.tokens.primary)
        layer?.borderColor = primary.cgColor
        layer?.borderWidth = Rule.focusRing
        layer?.backgroundColor = primary.withAlphaComponent(Self.fill).cgColor
    }
}
