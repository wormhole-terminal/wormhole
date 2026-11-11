// How the window and the panel's lists share the keyboard: F6 and ⇥ ask for the list (`request`), and the window reads
// whether one holds it (`listFocused`) and whether the tab on show has one at all (`lists`), because Notes, Project
// and Usage have none. The lists report their own focus; nothing here draws.

import Observation
import PanelModel

@MainActor @Observable
public final class PanelKeyboard {
    /// Counts up with every ask; the list on screen answers by taking the keyboard.
    public private(set) var requests = 0
    /// The ⇥ stop (a section) that ask is for.
    public private(set) var wanted = 0
    /// The list on screen, which says where its sections start.
    weak var walker: (any PanelWalking)?
    /// How many lists are on screen (the tab on show brings one or none).
    public private(set) var lists = 0
    /// How many of them hold the keyboard.
    public private(set) var holding = 0

    public init() {}

    public var listFocused: Bool { holding > 0 }

    /// How many ⇥ stops the tab on show has: one per section (`panel_lists`), at least one.
    public var stops: Int { max(walker?.listStarts.count ?? 0, 1) }
    /// The stop the cursor is in.
    public var stopAt: Int { walker?.listAt ?? 0 }

    public func request(stop: Int = 0) {
        wanted = stop
        requests += 1
    }

    func appeared() { lists += 1 }
    func disappeared(focused: Bool) {
        lists -= 1
        if focused { holding -= 1 }
    }
    func focusChanged(_ focused: Bool) { holding += focused ? 1 : -1 }
}
