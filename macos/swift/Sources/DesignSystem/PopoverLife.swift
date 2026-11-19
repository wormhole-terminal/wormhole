// When the popover a view is in closes, from AppKit's own word. `onDisappear` does not serve for a popover's content: it comes
// when the close has run its course, which is after the next popover may have appeared, and in a window behind the others (a
// test instance) not at all. `NSPopover` says that it will close at once, each time, however it is closed.

import AppKit
import SwiftUI

extension View {
    /// `closed` each time the popover holding this view begins to close.
    public func onPopoverClose(_ closed: @escaping @MainActor () -> Void) -> some View { background(PopoverLife(closed: closed)) }
}

private struct PopoverLife: NSViewRepresentable {
    let closed: @MainActor () -> Void

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        context.coordinator.listen(for: view)
        return view
    }

    func updateNSView(_ view: NSView, context: Context) { context.coordinator.closed = closed }

    func makeCoordinator() -> Coordinator { Coordinator(closed: closed) }

    /// An observer by selector: the notification centre lets go of it by itself when it is released.
    @MainActor final class Coordinator: NSObject {
        var closed: @MainActor () -> Void
        private weak var view: NSView?

        init(closed: @escaping @MainActor () -> Void) { self.closed = closed }

        func listen(for view: NSView) {
            self.view = view
            NotificationCenter.default.addObserver(self, selector: #selector(willClose), name: NSPopover.willCloseNotification, object: nil)
        }

        /// Only the popover this view is in.
        @objc private func willClose(_ note: Notification) {
            guard let view, let content = (note.object as? NSPopover)?.contentViewController?.view, view.isDescendant(of: content) else { return }
            closed()
        }
    }
}
