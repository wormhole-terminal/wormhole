// The window's actions as the core's (`WormholeBindings.Action`): one case each, the ids widened to the core's. The
// drag's targets (`DropOver`) are the core's `Over` by id.

import WorkspaceModel
import WormholeBindings

extension WormholeBindings.Action {
    init(_ action: WorkspaceAction) {
        switch action {
        case .select(let id): self = .select(Int64(id))
        case .selectStep(let delta): self = .selectStep(Int32(delta))
        case .selectNumber(let number): self = .selectNumber(UInt32(number))
        case .newTerminal: self = .newTerminal
        // Made in two steps by the store: this is only reached through it.
        case .newTerminalInGroup: self = .newTerminal
        case .close(let id): self = .close(Int64(id))
        case .rename(let id, let name): self = .rename(id: Int64(id), name: name)
        case .togglePin(let id): self = .togglePin(Int64(id))
        case .newGroup(let members): self = .newGroup(members: members.map(Int64.init))
        case .renameGroup(let group, let name): self = .renameGroup(group: group, name: name)
        case .toggleGroup(let group): self = .toggleGroup(group)
        case .setGroupPinned(let group, let pinned): self = .setGroupPinned(group: group, pinned: pinned)
        case .ungroup(let group): self = .ungroup(group)
        case .addToGroup(let id, let group): self = .addToGroup(id: Int64(id), group: group)
        case .removeFromGroup(let id): self = .removeFromGroup(Int64(id))
        case .dropTerminal(let id, let over): self = .dropTerminal(dragged: Int64(id), over: Over(over))
        case .dropGroup(let group, let over): self = .dropGroup(group: group, over: Over(over))
        case .splitRight(let id): self = .splitBeside(anchor: Int64(id), axis: .row)
        case .joinSplit(let id, let target): self = .joinSplit(id: Int64(id), target: Int64(target), zone: .right)
        case .leaveSplit(let id): self = .leaveSplit(Int64(id))
        case .toggleSidebar: self = .toggleSidebar
        case .togglePanel: self = .togglePanel
        case .showPanelTab(let tab): self = .showPanelTab(TabId(tab))
        case .search(let text): self = .search(text)
        case .showOverlay(let overlay): self = .showOverlay(overlay.map(WormholeBindings.Overlay.init))
        }
    }
}

extension TabId {
    init(_ tab: PanelTab) {
        switch tab {
        case .notes: self = .notes
        case .changes: self = .changes
        case .history: self = .history
        case .usage: self = .usage
        case .project: self = .project
        case .files: self = .files
        }
    }
}

extension WormholeBindings.Overlay {
    init(_ overlay: WorkspaceModel.Overlay) {
        switch overlay {
        case .palette: self = .palette
        case .shortcuts: self = .shortcuts
        }
    }
}

extension Over {
    init(_ over: DropOver) {
        switch over {
        case .row(let id, let part): self = .row(id: Int64(id), part: Part(part))
        case .head(let group, let part): self = .head(group: group, part: Part(part))
        case .after(let card): self = .after(WormholeBindings.CardRef(card))
        case .tail: self = .tail
        }
    }
}

extension Part {
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
