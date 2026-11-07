// Whether this view's own window is full screen. In full screen macOS hides the traffic lights, so the
// title rows give their room back. Read from the window itself (not a global notification) so a second
// window cannot answer for the first.

import AppKit
import SwiftUI

struct FullScreenReader: NSViewRepresentable {
    @Binding var isFullScreen: Bool

    func makeNSView(context: Context) -> NSView { Watcher { isFullScreen = $0 } }
    func updateNSView(_ view: NSView, context: Context) {}

    private final class Watcher: NSView {
        private let report: (Bool) -> Void

        init(report: @escaping (Bool) -> Void) {
            self.report = report
            super.init(frame: .zero)
        }

        @available(*, unavailable) required init?(coder: NSCoder) { fatalError("not decoded") }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            NotificationCenter.default.removeObserver(self)
            guard let window else { return }
            for name in [NSWindow.didEnterFullScreenNotification, NSWindow.didExitFullScreenNotification] {
                NotificationCenter.default.addObserver(
                    self, selector: #selector(changed), name: name, object: window)
            }
            changed()
        }

        @objc private func changed() {
            report(window?.styleMask.contains(.fullScreen) ?? false)
        }
    }
}
