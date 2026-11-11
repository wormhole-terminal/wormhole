// Esc ends a drag in flight (GPUI's `sidebar/keys.rs`). The drag is the pointer's own gesture, so the list may not have the
// keyboard when Esc comes: while a drag is live a local key monitor of the app answers it, wherever the keys are. The
// monitor is armed by the drag's first move and put away when the drag ends however it ends.

import AppKit

@MainActor
final class DragEscape {
    private var monitor: Any?
    /// The drag was ended by Esc and the button is still down: later moves of the same gesture are nothing.
    private(set) var ended = false

    func arm(_ cancel: @escaping @MainActor () -> Void) {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard event.keyCode == 53 else { return event }
            MainActor.assumeIsolated {
                self?.ended = true
                cancel()
            }
            return nil
        }
    }

    /// The gesture is over (button up, or Esc): the monitor goes and the next drag starts afresh.
    func disarm() {
        if let monitor { NSEvent.removeMonitor(monitor) }
        monitor = nil
    }

    func finish() {
        disarm()
        ended = false
    }
}
