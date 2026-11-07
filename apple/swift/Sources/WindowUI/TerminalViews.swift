// One terminal view per terminal id, kept alive while another is shown: switching must not recreate a
// running shell. Views are made lazily on first show, by a closure the app gives (it owns the sources), and
// shut down when their terminal is gone.

import TerminalMetal
import WorkspaceModel

@MainActor
public final class TerminalViews {
    private var views: [TerminalID: MetalTerminalView] = [:]
    private let make: @MainActor (TerminalID) -> MetalTerminalView?

    /// `make` returns nil when a source cannot start; it reports why itself.
    public init(make: @escaping @MainActor (TerminalID) -> MetalTerminalView?) {
        self.make = make
    }

    /// The terminal's view, made on first use.
    public func view(for id: TerminalID) -> MetalTerminalView? {
        if let view = views[id] { return view }
        guard let view = make(id) else { return nil }
        views[id] = view
        return view
    }

    /// The view if it was made already; never starts a terminal.
    public func existing(_ id: TerminalID) -> MetalTerminalView? { views[id] }

    /// Shut down the views of terminals that no longer exist.
    func retain(only ids: Set<TerminalID>) {
        for id in views.keys where !ids.contains(id) {
            views.removeValue(forKey: id)?.shutdown()
        }
    }

    public func shutdown() {
        for view in views.values { view.shutdown() }
        views.removeAll()
    }
}
