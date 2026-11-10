// One terminal view per terminal id, kept alive while another is shown: switching must not recreate a
// running shell. Views are made lazily on first show, by a closure the app gives (it owns the sources), and
// shut down when their terminal is gone.

import Observation
import TerminalMetal
import WorkspaceModel

@MainActor @Observable
public final class TerminalViews {
    @ObservationIgnored private var views: [TerminalID: MetalTerminalView] = [:]
    @ObservationIgnored private let make: @MainActor (TerminalID) -> MetalTerminalView?
    /// Moves when any view's grid changes, so what shows a size (the status bar) is drawn again.
    public private(set) var gridRevision = 0

    /// `make` returns nil when a source cannot start; it reports why itself.
    public init(make: @escaping @MainActor (TerminalID) -> MetalTerminalView?) {
        self.make = make
    }

    /// The terminal's view, made on first use.
    public func view(for id: TerminalID) -> MetalTerminalView? {
        if let view = views[id] { return view }
        guard let view = make(id) else { return nil }
        view.onGridChange = { [weak self] in self?.gridRevision += 1 }
        views[id] = view
        return view
    }

    /// The view if it was made already; never starts a terminal.
    public func existing(_ id: TerminalID) -> MetalTerminalView? { views[id] }

    /// Shut a terminal's view down and forget it: the next `view(for:)` starts it again.
    func discard(_ id: TerminalID) { views.removeValue(forKey: id)?.shutdown() }

    /// Shut down the views of terminals that no longer exist.
    func retain(only ids: Set<TerminalID>) {
        for id in views.keys where !ids.contains(id) {
            views.removeValue(forKey: id)?.shutdown()
        }
    }

    /// Every terminal's view that exists: settings that reach all of them (fonts, colours) walk it.
    public func forEach(_ body: (MetalTerminalView) -> Void) { views.values.forEach(body) }

    public func shutdown() {
        for view in views.values { view.shutdown() }
        views.removeAll()
    }
}
