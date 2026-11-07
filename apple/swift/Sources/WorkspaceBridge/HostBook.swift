// What the workspace keeps about its terminals' session hosts besides what the core's `Terminal`s know: which are being
// connected, which could not be shown, and which are on screen.

import WorkspaceModel

struct HostBook {
    /// Terminals being started or attached to, by a token: a result that comes after a restart or a close is dropped.
    var connecting: [TerminalID: Int] = [:]
    var tokens = 0
    /// Terminals whose host runs and could not be shown: they keep its id and say why (`shown`: the bar is up; not while
    /// the one quiet second try is still to come).
    var unavailable: [TerminalID: (why: HostProblem.Why, host: String, shown: Bool)] = [:]
    /// An attach that failed in a way that may pass is tried once more before the bar shows.
    var retried: Set<TerminalID> = []
    /// The launch's terminals are still connecting: the saved rows of the last run are forgotten when the last is.
    var restoring = false
    /// Which terminals a host last fed (on screen) or left headless.
    var visible: [TerminalID: Bool] = [:]
    var quitting = false
    /// Saves the screens each minute (`HostedTerminals.saveTexts`).
    var saver: Task<Void, Never>?
    mutating func forget(_ id: TerminalID) {
        connecting[id] = nil
        unavailable[id] = nil
        retried.remove(id)
        visible[id] = nil
    }

    /// What was told once this run: the fallback and the move to Applications, however many terminals start.
    @MainActor static var told: Set<String> = []
}
