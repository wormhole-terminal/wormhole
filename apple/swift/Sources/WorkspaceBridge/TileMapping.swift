// A tile's mark, how loud a terminal is and a folded group's tiles, as the core words them.

import WorkspaceModel
import WormholeBindings

extension WorkspaceModel.TileGlyph {
    init(_ core: WormholeBindings.TileGlyph) {
        switch core {
        case .working: self = .working
        case .needsYou: self = .needsYou
        case .done: self = .done
        case .failed: self = .failed
        case .idle: self = .idle
        case .terminal(let unread): self = .terminal(unread: unread)
        case .shell: self = .shell
        }
    }
}

extension WorkspaceModel.Urgency {
    init(_ core: WormholeBindings.Urgency) {
        switch core {
        case .needsYou: self = .needsYou
        case .failed: self = .failed
        case .working: self = .working
        case .bell: self = .bell
        case .newOutput: self = .newOutput
        case .running: self = .running
        case .done: self = .done
        case .idle: self = .idle
        }
    }
}

extension WorkspaceModel.MemberTile {
    init(_ core: WormholeBindings.MemberTile) {
        self.init(
            id: Int(core.id), name: core.name, glyph: WorkspaceModel.TileGlyph(core.glyph), activity: Activity(core.state),
            agent: core.agent.flatMap { AgentID(command: $0.key) }, urgency: WorkspaceModel.Urgency(core.urgency))
    }
}
