// Writes a window's record when something about it changed: the workspace's state (terminals, names, groups, columns,
// selection) moved, or the window did. Both are events, never polled; a burst is one write, a moment after its last
// change (a drag of the window or an agent's output says a hundred things a second). The rules of what is saved are the
// core's; the write is a few rows that differ.

import AppKit
import Observation
import WindowUI
import WindowsModel

@MainActor
final class WindowSaver {
    /// How long after the last change the record is written.
    static let settle: Duration = .milliseconds(600)

    private unowned let session: WindowSession
    private let windows: any WindowsBackend
    private var pending: Task<Void, Never>?

    init(session: WindowSession, windows: any WindowsBackend) {
        self.session = session
        self.windows = windows
    }

    /// Follow the state from now on.
    func start() { follow() }

    /// The window moved, resized or changed what it shows: its place is new.
    func frameChanged() { schedule() }

    func cancel() { pending?.cancel() }

    /// Write it now (a quit, or before another window's save that must come second).
    func saveNow() {
        pending?.cancel()
        windows.save(window: session.id, workspace: session.hub.store.source, frame: frame())
    }

    private func follow() {
        withObservationTracking { _ = session.hub.state } onChange: { [weak self] in
            Task { @MainActor in
                self?.schedule()
                self?.follow()
            }
        }
    }

    private func schedule() {
        pending?.cancel()
        pending = Task { [weak self] in
            try? await Task.sleep(for: Self.settle)
            guard !Task.isCancelled else { return }
            self?.saveNow()
        }
    }

    func frame() -> WindowFrame? {
        guard let controller = session.controller, let window = controller.window else { return nil }
        return WindowFrames.frame(of: window, windowed: controller.windowedFrame)
    }
}
