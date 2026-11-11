// The core's words about where a program runs, as the window's values.

import WorkspaceModel
import WormholeBindings

extension HostingState {
    init(_ core: TerminalHosting) {
        switch core {
        case .local: self = .local
        case .persistent(let id): self = .persistent(hostID: id)
        case .detached(let id): self = .detached(hostID: id)
        case .lost(let id): self = .lost(hostID: id)
        }
    }
}
