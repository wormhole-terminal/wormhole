// The one door to the system's open panel for the Settings pages. A test instance (`--background`) never shows one (a modal
// panel would hold the whole app): it answers with the path the control folder gave (`panel-pick <path>`, used once), or
// cancels when none was given.

import AppKit
import Synchronization

public enum PanelPick {
    private struct State {
        var diverted = false
        var next: URL?
    }
    private static let state = Mutex(State())

    /// A test instance answers panels itself from now on.
    public static func divert() { state.withLock { $0.diverted = true } }

    /// What the next panel answers (a file, folder or app), once.
    public static func answer(with url: URL?) { state.withLock { $0.next = url } }

    /// The url `panel` returns: the person's pick, or in a test instance the one given for it (nil: cancelled).
    @MainActor public static func run(_ panel: NSOpenPanel) -> URL? {
        let held = state.withLock { state -> (diverted: Bool, next: URL?) in
            defer { if state.diverted { state.next = nil } }
            return (state.diverted, state.next)
        }
        if held.diverted { return held.next }
        return panel.runModal() == .OK ? panel.url : nil
    }
}
