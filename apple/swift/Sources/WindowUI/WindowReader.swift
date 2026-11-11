// What this view's own window says that the chrome lays itself out by: whether it is full screen (macOS hides
// the traffic lights, so the rows give their room back) and where the lights end. Read from the window itself
// (not a global notification) so a second window cannot answer for the first.

import AppKit
import SwiftUI

struct WindowFacts: Equatable {
    var isFullScreen = false
    /// From the window's left edge; the lights' span is measured because later macOS versions draw them larger.
    var lightsEnd = Chrome.lightsX + Chrome.assumedLightsSpan

    /// The collapse button starts here: after the lights, and where it is at least `collapseX`. Without lights
    /// it mirrors the panel toggle, its glyph on the 12 pt edge.
    var collapseX: CGFloat {
        isFullScreen ? Chrome.panelToggleRight : max(Chrome.collapseX, lightsEnd + Chrome.afterLights)
    }
    /// What the button takes from a row it sits over, and the gap after it.
    var collapseSlot: CGFloat { collapseX + Chrome.hit + Chrome.afterCollapse }
}

struct WindowReader: NSViewRepresentable {
    @Binding var facts: WindowFacts

    func makeNSView(context: Context) -> NSView { Watcher { facts = $0 } }
    func updateNSView(_ view: NSView, context: Context) {}

    private final class Watcher: NSView {
        private let report: (WindowFacts) -> Void
        /// A resize reports on every frame of a drag; only a different answer is worth a redraw.
        private var last: WindowFacts?

        init(report: @escaping (WindowFacts) -> Void) {
            self.report = report
            super.init(frame: .zero)
        }

        @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            NotificationCenter.default.removeObserver(self)
            guard let window else { return }
            let names: [Notification.Name] = [
                NSWindow.didEnterFullScreenNotification, NSWindow.didExitFullScreenNotification,
                NSWindow.didResizeNotification,
            ]
            for name in names {
                NotificationCenter.default.addObserver(self, selector: #selector(changed), name: name, object: window)
            }
            changed()
        }

        @objc private func changed() {
            guard let window else { return }
            let facts = WindowFacts(
                isFullScreen: window.styleMask.contains(.fullScreen), lightsEnd: TrafficLights.end(of: window))
            guard facts != last else { return }
            last = facts
            report(facts)
        }
    }
}
