// Feeds `CommandHold` from the window's modifier and key events, and shows the hints when its timer is up. Events,
// not polling: a local event monitor and the window's resign notification, and one sleep for the 250 ms.

import AppKit
import Observation

@MainActor @Observable
final class CommandHoldMonitor {
    private(set) var showing = false
    @ObservationIgnored private var hold = CommandHold()
    @ObservationIgnored private var monitor: Any?
    @ObservationIgnored private var resign: (any NSObjectProtocol)?
    @ObservationIgnored private var timer: Task<Void, Never>?

    func start() {
        guard monitor == nil else { return }
        monitor = NSEvent.addLocalMonitorForEvents(matching: [.flagsChanged, .keyDown]) { [weak self] event in
            MainActor.assumeIsolated { self?.handle(event) }
            return event
        }
        resign = NotificationCenter.default.addObserver(forName: NSWindow.didResignKeyNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.blur() }
        }
    }

    func stop() {
        monitor.map(NSEvent.removeMonitor); monitor = nil
        resign.map(NotificationCenter.default.removeObserver); resign = nil
        blur()
    }

    private func handle(_ event: NSEvent) {
        if event.type == .keyDown {
            hold.key()
        } else {
            let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
            hold.modifiers(commandAlone: flags == .command, at: Date())
        }
        refresh()
    }

    private func blur() { hold.blur(); refresh() }

    private func refresh() {
        timer?.cancel()
        showing = hold.showing(at: Date())
        guard let due = hold.due(at: Date()) else { return }
        timer = Task { [weak self] in
            try? await Task.sleep(for: .seconds(due))
            guard !Task.isCancelled else { return }
            self?.refresh()
        }
    }
}
