// A drag's targets and landings as the core names them: by id, so the core's rules place what was dropped.

import WorkspaceModel
import WormholeBindings

extension Over {
    init(_ over: DropOver) {
        switch over {
        case .row(let id, let part): self = .row(id: Int64(id), part: WormholeBindings.Part(part))
        case .head(let group, let part): self = .head(group: group, part: WormholeBindings.Part(part))
        case .after(let card): self = .after(WormholeBindings.CardRef(card))
        case .tail: self = .tail
        }
    }
}

extension WormholeBindings.Part {
    init(_ part: DropPart) {
        switch part {
        case .top: self = .top
        case .middle: self = .middle
        case .bottom: self = .bottom
        }
    }
}

extension WormholeBindings.CardRef {
    init(_ card: WorkspaceModel.CardRef) {
        switch card {
        case .group(let id): self = .group(id)
        case .loose(let id): self = .loose(Int64(id))
        case .split(let id): self = .split(Int64(id))
        }
    }
}

extension WormholeBindings.Landing {
    init(_ landing: WorkspaceModel.Landing) {
        switch landing {
        case .top(let before): self = .top(before: before.map(Item.init))
        case .group(let group, let before): self = .group(group: group, before: before.map(Int64.init))
        }
    }
}

extension Item {
    init(_ item: TopItem) {
        switch item {
        case .terminal(let id): self = .terminal(Int64(id))
        case .group(let id): self = .group(id)
        }
    }
}
