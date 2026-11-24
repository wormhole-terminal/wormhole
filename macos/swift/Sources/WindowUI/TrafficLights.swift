// Where the traffic lights are drawn: 12 pt in from the left, centred in the 38 pt title row, as the GPUI
// app places them (`style/lights.rs`). AppKit centres them in its own, shorter bar, so the bar's container is
// grown to the row's height, and the three buttons are moved to the 12 pt line keeping their own spacing and
// to the row's middle: a bar that grows keeps them where they were from its bottom (macOS 27).

import AppKit

@MainActor
enum TrafficLights {
    private static let kinds: [NSWindow.ButtonType] = [.closeButton, .miniaturizeButton, .zoomButton]

    static func place(in window: NSWindow) {
        guard !window.styleMask.contains(.fullScreen),
            let close = window.standardWindowButton(.closeButton), let container = close.superview?.superview
        else { return }
        container.frame.size.height = Chrome.titleHeight
        container.frame.origin.y = window.frame.height - Chrome.titleHeight
        let shift = Chrome.lightsX - close.frame.minX
        for button in kinds.compactMap(window.standardWindowButton) {
            button.frame.origin.x += shift
            if let bar = button.superview { button.frame.origin.y = ((bar.bounds.height - button.frame.height) / 2).rounded() }
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
