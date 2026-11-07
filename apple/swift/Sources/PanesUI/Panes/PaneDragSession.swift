// A divider drag in progress, which the window's Esc ladder can end: Esc puts the panes back as they were
// when the button went down (`cancel_pane_drag`).

import Observation
import PanesModel

@MainActor @Observable
public final class PaneDragSession {
    public private(set) var divider: DividerID?
    @ObservationIgnored private var before: TabPanes?
    @ObservationIgnored private var restore: ((TabPanes) -> Void)?
    /// How far the pointer stood from the divider's line when the button went down, so it doesn't jump.
    @ObservationIgnored var grab: Double = 0

    public init() {}

    var isActive: Bool { divider != nil }

    func begin(_ divider: DividerID, panes: TabPanes, grab: Double, restore: @escaping (TabPanes) -> Void) {
        (self.divider, before, self.restore, self.grab) = (divider, panes, restore, grab)
    }

    func end() { (divider, before, restore) = (nil, nil, nil) }

    /// Esc: whether a drag was going on, and if so the panes go back to where it began.
    @discardableResult
    public func cancel() -> Bool {
        guard let before, let restore else { return false }
        restore(before)
        end()
        return true
    }
}
