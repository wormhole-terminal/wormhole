// Terminals moving between two windows' workspaces, programs running. The core does the bookkeeping and keeps the running
// `Terminal`s (`Workspace.takeOut`/`receive`); what stays Swift's is each terminal's wake (`TerminalRelay`, which the
// view's source listens through) and letting the old view go without ending the program.

import TerminalCore
import UniFFISource
import WorkspaceModel
import WormholeBindings

extension RustWorkspaceBackend {
    public func takeOut(_ ids: [TerminalID]) -> TerminalDeparture? {
        let taken = core.takeOut(ids: ids.map(Int64.init))
        guard let departure = taken.departure else { return nil }
        let moved = departure.ids().map(Int.init)
        // Before the close is handled, which forgets a closed terminal's relay and its view's source would end it.
        let carried = moved.reduce(into: [TerminalID: TerminalRelay]()) { $0[$1] = relays[$1] }
        for id in moved { sources[id]?.value?.detach() }
        _ = handle(taken.events)
        return TerminalDeparture(ids: moved, payload: RustDeparture(core: departure, relays: carried))
    }

    public func receive(_ departure: TerminalDeparture) {
        guard let cargo = departure.payload as? RustDeparture else { return }
        relays.merge(cargo.relays) { _, moved in moved }
        _ = handle(core.receive(departure: cargo.core))
        // A terminal in a session host is let go of, at the quit, by the window that holds it now.
        for id in departure.ids { core.terminal(id: Int64(id)).map { hosted?.desk.adopt(tab: Int64(id), terminal: $0) } }
    }
}

/// What `takeOut` carries to the other window: the core's departure and the wakes of the terminals' views.
final class RustDeparture {
    let core: Departure
    let relays: [TerminalID: TerminalRelay]

    init(core: Departure, relays: [TerminalID: TerminalRelay]) {
        self.core = core
        self.relays = relays
    }
}

struct WeakSource {
    weak var value: UniFFIFrameSource?
    init(_ value: UniFFIFrameSource) { self.value = value }
}
