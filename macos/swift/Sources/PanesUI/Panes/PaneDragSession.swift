// A divider drag in progress, which the window's Esc ladder can end: Esc puts the divider back where it stood when the button
// went down (`cancel_pane_drag`).

import Observation
import WorkspaceModel

@MainActor @Observable
public final class PaneDragSession {
    public private(set) var divider: PaneDividerState.ID?
    @ObservationIgnored private var restore: (() -> Void)?
    /// How far the pointer stood from the divider's line when the button went down, so it doesn't jump.
    @ObservationIgnored var grab: Double = 0

    public init() {}

    var isActive: Bool { divider != nil }

    func begin(_ divider: PaneDividerState.ID, grab: Double, restore: @escaping () -> Void) {
        (self.divider, self.grab, self.restore) = (divider, grab, restore)
    }

    /// Esc put the divider back while the button is still down: the rest of that gesture moves nothing.
    @ObservationIgnored private(set) var cancelled = false

    func end() { (divider, restore, cancelled) = (nil, nil, false) }

    /// Esc: whether a drag was going on, and if so the divider goes back to where it began.
    @discardableResult
    public func cancel() -> Bool {
        guard let restore else { return false }
        restore()
        (divider, self.restore, cancelled) = (nil, nil, true)
        return true
    }
}
