// An `NSWindow`'s place as the core's rules take it (`WindowsModel.WindowFrame`): one global space with y growing
// downwards and the primary display first, where AppKit has y growing upwards from the primary display's bottom edge.
// The conversion lives here, once; the rules (what stays reachable, where a new window steps to) are the core's.

import AppKit
import WindowsModel

@MainActor
public enum WindowFrames {
    /// The displays attached, the primary (the one with the menu bar) first, by their usable area.
    public static func displays() -> [DisplayArea] {
        NSScreen.screens.map { area(of: $0) }
    }

    /// The display `window` is mostly on (the primary one when it is on none).
    public static func display(of window: NSWindow) -> DisplayArea? {
        (window.screen ?? NSScreen.screens.first).map { area(of: $0) }
    }

    /// `rect` between AppKit's space (y up) and the global one (y down): flipping against the primary display's height
    /// is its own inverse, so one function does both ways.
    static func flipped(_ rect: NSRect) -> NSRect {
        NSRect(x: rect.minX, y: primaryHeight - rect.maxY, width: rect.width, height: rect.height)
    }

    private static var primaryHeight: CGFloat { NSScreen.screens.first?.frame.height ?? 0 }

    private static func area(of screen: NSScreen) -> DisplayArea {
        let visible = flipped(screen.visibleFrame)
        let number = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber // text: AppKit key
        return DisplayArea(
            id: number.map { UInt64($0.uint32Value) } ?? 0, x: visible.minX, y: visible.minY, width: visible.width,
            height: visible.height)
    }

    /// A test instance cannot enter the system's full screen (AppKit allows it only for the active app), so it keeps the state
    /// itself: the windows it holds as full screen. A saved window comes back through the same call a person's green light
    /// makes, and the frame record reads it, so the save and the restore run for real up to the system's own transition.
    public static var simulatesFullScreen = false
    private static var simulated: Set<ObjectIdentifier> = []

    public static func isFullScreen(_ window: NSWindow) -> Bool {
        simulatesFullScreen ? simulated.contains(ObjectIdentifier(window)) : window.styleMask.contains(.fullScreen)
    }

    /// What the green light's full-screen item does.
    public static func toggleFullScreen(_ window: NSWindow) {
        guard simulatesFullScreen else { return window.toggleFullScreen(nil) }
        if !simulated.insert(ObjectIdentifier(window)).inserted { simulated.remove(ObjectIdentifier(window)) }
    }

    /// Where `window` is, with what to come back to when it is full screen or zoomed: `windowed`, the rectangle it had
    /// before (the system does not say it, the controller keeps the last one).
    public static func frame(of window: NSWindow, windowed: NSRect) -> WindowFrame {
        let kind: FrameKind = isFullScreen(window) ? .fullscreen : window.isZoomed ? .maximized : .windowed
        let rect = flipped(kind == .windowed ? window.frame : windowed)
        return WindowFrame(
            kind: kind, x: rect.minX, y: rect.minY, width: rect.width, height: rect.height,
            display: display(of: window)?.id, minimised: window.isMiniaturized)
    }

    /// A windowed frame at `rect` of AppKit's space, on display `display`.
    public static func windowed(_ rect: NSRect, on display: UInt64?) -> WindowFrame {
        let global = flipped(rect)
        return WindowFrame(kind: .windowed, x: global.minX, y: global.minY, width: global.width, height: global.height, display: display, minimised: false)
    }

    /// Put `window` where `frame` says (already fitted to the displays), without animation.
    public static func place(_ frame: WindowFrame, on window: NSWindow) {
        window.setFrame(flipped(NSRect(x: frame.x, y: frame.y, width: frame.width, height: frame.height)), display: false)
    }

    /// What the system does for the kinds: zoom and full screen are asked once the window is on the screen.
    public static func settle(_ frame: WindowFrame, on window: NSWindow) {
        switch frame.kind {
        case .windowed: break
        case .maximized: window.zoom(nil)
        case .fullscreen: if !isFullScreen(window) { toggleFullScreen(window) }
        }
    }
}
