// Where the traffic lights are drawn: 12 pt in from the left, on the middle line of the 38 pt title row, as the GPUI
// app places them (`style/lights.rs`). AppKit centres them in its own, shorter bar, so the three buttons are moved:
// to the 12 pt line keeping their own spacing, and to the row's middle, which is a place in the window (19 pt under
// its top), whatever height the bar has. AppKit puts them back when it lays its title bar out again, so the window's
// controller places them again each time (`WorkspaceWindowController`).

import AppKit

@MainActor
enum TrafficLights {
    private static let kinds: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]

    static func place(in window: NSWindow) {
        guard !window.styleMask.contains(.fullScreen), let close = window.standardWindowButton(.closeButton) else { return }
        let shift = Chrome.lightsX - close.frame.minX
        for button in kinds.compactMap(window.standardWindowButton) {
            guard let bar = button.superview else { continue }
            let middle = bar.convert(NSPoint(x: 0, y: window.frame.height - Chrome.titleHeight / 2), from: nil).y
            let origin = NSPoint(x: button.frame.minX + shift, y: (middle - button.frame.height / 2).rounded())
            // Only a real move: the controller hears of every change of the close button's frame.
            if button.frame.origin != origin { button.setFrameOrigin(origin) }
        }
    }

    /// Where the lights end from the window's left edge, which whatever has to clear them reads; the assumed
    /// span when the window has none yet.
    static func end(of window: NSWindow?) -> CGFloat {
        guard let window, let close = window.standardWindowButton(.closeButton),
            let zoom = window.standardWindowButton(.zoomButton)
        else { return Chrome.lightsX + Chrome.assumedLightsSpan }
        return Chrome.lightsX + zoom.frame.maxX - close.frame.minX
    }
}
